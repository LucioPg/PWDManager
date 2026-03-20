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

/// Gets the existing key from keyring, or generates and stores a new one.
/// This is the main entry point called by `init_db()`.
///
/// - `Ok(key)` if a key exists or was successfully created
/// - `Err(DBKeyError::NoEntry)` is never returned (we create on miss)
/// - `Err(DBKeyError::KeyringError)` if the keyring is unavailable
pub fn get_or_create_db_key() -> Result<String, DBKeyError> {
    match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
        Ok(key) => Ok(key),
        Err(DBKeyError::NoEntry) => {
            let key = generate_key();
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        }
        Err(e) => Err(e),
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
        // It should always succeed (create if missing).
        let result = get_or_create_db_key();
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 64);
    }
}
