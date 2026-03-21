use keyring::{Entry, Error as KeyringError};
use rand::Rng;

/// Service name used in the OS keyring (Windows Credential Manager).
pub const SERVICE_NAME: &str = "PWDManager";

/// Username/identifier for the DB encryption key in the keyring.
pub const KEY_USERNAME: &str = "db_encryption_key";

/// Error type for keyring operations.
#[derive(Debug)]
pub enum DBKeyError {
    /// The requested credential does not exist in the keyring.
    NoEntry,
    /// A keyring system error (service unavailable, access denied, etc.)
    KeyringError(String),
}

impl std::fmt::Display for DBKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBKeyError::NoEntry => write!(f, "Keyring entry not found"),
            DBKeyError::KeyringError(msg) => write!(f, "Keyring error: {}", msg),
        }
    }
}

impl std::error::Error for DBKeyError {}

/// Generates a 64-character hex string (32 random bytes, hex-encoded).
/// Used as raw key material via `PRAGMA key = "x'...'"`.
fn generate_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill(&mut key_bytes);
    key_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Stores a key in the OS keyring under the given service/username.
fn store_db_key(service: &str, username: &str, key: &str) -> Result<(), DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.set_password(key)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))
}

/// Retrieves a key from the OS keyring.
fn retrieve_db_key(service: &str, username: &str) -> Result<String, DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.get_password().map_err(|e| match e {
        KeyringError::NoEntry => DBKeyError::NoEntry,
        _ => DBKeyError::KeyringError(e.to_string()),
    })
}

/// Deletes the keyring entry if it exists. Used for cleanup.
fn delete_db_key(service: &str, username: &str) {
    if let Ok(entry) = Entry::new(service, username) {
        let _ = entry.delete_credential();
    }
}

/// Gets the existing key from keyring, or generates and stores a new one.
/// This is the main entry point called by `init_db()`.
///
/// Handles the DB ↔ keyring state consistency:
/// - Both exist → use existing key
/// - Neither exists → generate and store new key
/// - DB exists, keyring empty → error (encrypted DB with lost key)
/// - Keyring has key, no DB → clean up orphaned key, generate new one
pub fn get_or_create_db_key(db_path: &str) -> Result<String, DBKeyError> {
    let db_exists = std::path::Path::new(db_path).exists();
    let key_result = retrieve_db_key(SERVICE_NAME, KEY_USERNAME);

    match (db_exists, key_result) {
        // Normal case: DB and key both exist
        (true, Ok(key)) => Ok(key),

        // Fresh install: neither DB nor key
        (false, Err(DBKeyError::NoEntry)) => {
            let key = generate_key();
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        }

        // Orphaned keyring entry: no DB but key exists → clean up and regenerate
        (false, Ok(_)) => {
            delete_db_key(SERVICE_NAME, KEY_USERNAME);
            let key = generate_key();
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        }

        // Dangerous: encrypted DB exists but key is missing → data loss risk
        (true, Err(DBKeyError::NoEntry)) => Err(DBKeyError::KeyringError(
            "Encrypted database found but keyring entry is missing. \
             The database cannot be decrypted.".into(),
        )),

        // Keyring system error (e.g., access denied)
        (_, Err(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SERVICE: &str = "PWDManager-test";
    const TEST_USER: &str = "test-key";

    fn cleanup() {
        if let Ok(entry) = Entry::new(TEST_SERVICE, TEST_USER) {
            let _ = entry.delete_credential();
        }
    }

    #[test]
    fn test_generate_key_returns_64_char_hex_string() {
        let key = generate_key();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_key_is_unique() {
        let key1 = generate_key();
        let key2 = generate_key();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_store_and_retrieve_key() {
        cleanup();
        let key = generate_key();
        store_db_key(TEST_SERVICE, TEST_USER, &key).unwrap();
        let retrieved = retrieve_db_key(TEST_SERVICE, TEST_USER).unwrap();
        assert_eq!(key, retrieved);
        cleanup();
    }

    #[test]
    fn test_retrieve_nonexistent_key_returns_no_entry() {
        cleanup();
        let result = retrieve_db_key(TEST_SERVICE, TEST_USER);
        assert!(matches!(result, Err(DBKeyError::NoEntry)));
        cleanup();
    }

    #[test]
    fn test_get_or_create_always_returns_valid_key() {
        // get_or_create_db_key uses the real SERVICE_NAME/KEY_USERNAME.
        // It should always succeed (create if missing) when no DB exists.
        let result = get_or_create_db_key("nonexistent_database.db");
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 64);
        // Cleanup: remove the key we just created
        delete_db_key(SERVICE_NAME, KEY_USERNAME);
    }
}
