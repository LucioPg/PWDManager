use argon2::Argon2;
use keyring::{Entry, Error as KeyringError};
use rand::Rng;

use crate::backend::password_utils::detect_system_language;
use crate::backend::settings_types::DicewareLanguage;

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
    /// DB exists but open fails (keyring empty or has wrong key) → recovery needed
    MissingKeyWithDb,
    /// Passphrase does not open the DB
    RecoveryKeyInvalid,
    /// Error reading/writing salt file
    SaltFileError(String),
}

impl std::fmt::Display for DBKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBKeyError::NoEntry => write!(f, "Keyring entry not found"),
            DBKeyError::KeyringError(msg) => write!(f, "Keyring error: {}", msg),
            DBKeyError::MissingKeyWithDb => write!(f, "Database encryption key missing or invalid"),
            DBKeyError::RecoveryKeyInvalid => write!(f, "Invalid recovery key"),
            DBKeyError::SaltFileError(msg) => write!(f, "Salt file error: {}", msg),
        }
    }
}

impl std::error::Error for DBKeyError {}

/// Generates a 64-character hex string (32 random bytes, hex-encoded).
/// Used as raw key material via `PRAGMA key = "x'...'"`.
pub(crate) fn generate_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill(&mut key_bytes);
    key_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Stores a key in the OS keyring under the given service/username.
pub(crate) fn store_db_key(service: &str, username: &str, key: &str) -> Result<(), DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.set_password(key)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))
}

/// Retrieves a key from the OS keyring.
pub(crate) fn retrieve_db_key(service: &str, username: &str) -> Result<String, DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.get_password().map_err(|e| match e {
        KeyringError::NoEntry => DBKeyError::NoEntry,
        _ => DBKeyError::KeyringError(e.to_string()),
    })
}

/// Deletes the keyring entry if it exists. Used for cleanup.
pub(crate) fn delete_db_key(service: &str, username: &str) {
    if let Ok(entry) = Entry::new(service, username) {
        let _ = entry.delete_credential();
    }
}

/// Derives a 64-char hex key from a passphrase and salt using Argon2id.
/// MUST be called via `spawn_blocking` — CPU-bound operation (~50ms).
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<String, DBKeyError> {
    if salt.len() != 16 {
        return Err(DBKeyError::SaltFileError(
            format!("Salt must be 16 bytes, got {}", salt.len()),
        ));
    }
    let argon2 = Argon2::default();
    let mut output = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut output)
        .map_err(|e| DBKeyError::KeyringError(format!("Key derivation failed: {}", e)))?;
    Ok(output.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Returns the salt file path for a given DB path.
pub fn salt_file_path(db_path: &str) -> String {
    format!("{}.salt", db_path)
}

/// Reads 16 bytes from the salt file and returns them.
pub fn read_salt(db_path: &str) -> Result<Vec<u8>, DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let hex = std::fs::read_to_string(&salt_path).map_err(|e| {
        DBKeyError::SaltFileError(format!("Cannot read salt file '{}': {}", salt_path, e))
    })?;
    let hex = hex.trim();
    if hex.len() != 32 {
        return Err(DBKeyError::SaltFileError(format!(
            "Invalid salt length: expected 32 hex chars, got {}",
            hex.len()
        )));
    }
    (0..16)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| {
                DBKeyError::SaltFileError(format!("Invalid hex in salt file: {}", e))
            })
        })
        .collect()
}

/// Writes 16 salt bytes to the salt file as 32 hex characters.
pub fn write_salt(db_path: &str, salt: &[u8; 16]) -> Result<(), DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let hex: String = salt.iter().map(|b| format!("{:02x}", b)).collect();
    std::fs::write(&salt_path, &hex).map_err(|e| {
        DBKeyError::SaltFileError(format!("Cannot write salt file '{}': {}", salt_path, e))
    })
}

/// Generates 16 random bytes for use as Argon2 salt.
pub fn generate_db_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::rng().fill(&mut salt);
    salt
}

/// Generates a 6-word diceware passphrase in CamelCase using the system language.
pub fn generate_recovery_passphrase() -> Result<String, DBKeyError> {
    let lang: DicewareLanguage = detect_system_language();
    let embedded_lang: diceware::EmbeddedList = lang.into();
    let config = diceware::Config::new()
        .with_embedded(embedded_lang)
        .with_words(6)
        .with_camel_case(true);
    diceware::make_passphrase(config)
        .map_err(|e| DBKeyError::KeyringError(format!("Failed to generate passphrase: {}", e)))
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
    fn test_derive_key_is_deterministic() {
        let salt = generate_db_salt();
        let key1 = derive_key("test passphrase", &salt).unwrap();
        let key2 = derive_key("test passphrase", &salt).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 64);
    }

    #[test]
    fn test_derive_key_different_passphrases_produce_different_keys() {
        let salt = generate_db_salt();
        let key1 = derive_key("passphrase one", &salt).unwrap();
        let key2 = derive_key("passphrase two", &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_rejects_wrong_salt_length() {
        let result = derive_key("test", &[0u8; 8]);
        assert!(matches!(result, Err(DBKeyError::SaltFileError(_))));
    }

    #[test]
    fn test_salt_file_roundtrip() {
        let dir = std::env::temp_dir().join("pwd_test_salt");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("test.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        write_salt(&db_path, &salt).unwrap();
        let read_back = read_salt(&db_path).unwrap();
        assert_eq!(salt.to_vec(), read_back);

        let _ = std::fs::remove_file(salt_file_path(&db_path));
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_read_salt_missing_file() {
        let result = read_salt("/nonexistent/path/database.db");
        assert!(matches!(result, Err(DBKeyError::SaltFileError(_))));
    }

    #[test]
    fn test_generate_db_salt_is_unique() {
        let salt1 = generate_db_salt();
        let salt2 = generate_db_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_generate_recovery_passphrase_format() {
        let passphrase = generate_recovery_passphrase().unwrap();
        // CamelCase words: no spaces, starts with uppercase
        assert!(!passphrase.contains(' '), "Should not contain spaces");
        assert!(
            passphrase.chars().next().unwrap().is_uppercase(),
            "Should start with uppercase"
        );
        // Should have multiple words (CamelCase boundaries)
        let word_count = passphrase.chars().filter(|c| c.is_uppercase()).count();
        assert!(word_count >= 6, "Should have at least 6 uppercase chars (6 words)");
    }

    #[test]
    fn test_generate_recovery_passphrase_is_unique() {
        let p1 = generate_recovery_passphrase().unwrap();
        let p2 = generate_recovery_passphrase().unwrap();
        assert_ne!(p1, p2);
    }
}
