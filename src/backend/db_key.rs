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
    /// CPU-bound derivation failure (Argon2, diceware) — not a keyring error
    DerivationError(String),
    /// File deletion failure during database reset
    DatabaseCleanupError(String),
}

impl std::fmt::Display for DBKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBKeyError::NoEntry => write!(f, "Keyring entry not found"),
            DBKeyError::KeyringError(msg) => write!(f, "Keyring error: {}", msg),
            DBKeyError::MissingKeyWithDb => write!(f, "Database encryption key missing or invalid"),
            DBKeyError::RecoveryKeyInvalid => write!(f, "Invalid recovery key"),
            DBKeyError::SaltFileError(msg) => write!(f, "Salt file error: {}", msg),
            DBKeyError::DerivationError(msg) => write!(f, "Derivation error: {}", msg),
            DBKeyError::DatabaseCleanupError(msg) => write!(f, "Database cleanup error: {}", msg),
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
        .map_err(|e| DBKeyError::DerivationError(format!("Key derivation failed: {}", e)))?;
    Ok(output.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Returns the salt file path for a given DB path.
pub fn salt_file_path(db_path: &str) -> String {
    format!("{}.salt", db_path)
}

/// Reads 16 bytes from the salt file and returns them as a fixed-size array.
pub fn read_salt(db_path: &str) -> Result<[u8; 16], DBKeyError> {
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
    let bytes: Vec<u8> = (0..16)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| {
                DBKeyError::SaltFileError(format!("Invalid hex in salt file: {}", e))
            })
        })
        .collect::<Result<Vec<u8>, _>>()?;

    bytes.try_into()
        .map_err(|_| DBKeyError::SaltFileError("Salt must be exactly 16 bytes".into()))
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
        .map_err(|e| DBKeyError::DerivationError(format!("Failed to generate passphrase: {}", e)))
}

/// Derives the DB key from a user-entered recovery passphrase.
/// Reads the salt file automatically. MUST be called via `spawn_blocking`.
pub fn derive_key_from_passphrase(passphrase: &str, db_path: &str) -> Result<String, DBKeyError> {
    let salt = read_salt(db_path)?;
    derive_key(passphrase, &salt)
}

/// Generates a new salt, derives key from passphrase, stores in keyring.
/// Returns the derived key hex string.
/// CPU-bound: call via `spawn_blocking`.
pub fn generate_and_store_key(
    passphrase: &str,
    db_path: &str,
) -> Result<String, DBKeyError> {
    let salt = generate_db_salt();
    write_salt(db_path, &salt)?;

    let result = derive_key(passphrase, &salt)
        .and_then(|key| {
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        });

    match result {
        Ok(key) => Ok(key),
        Err(e) => {
            // Cleanup orphaned salt file on failure
            let salt_path = salt_file_path(db_path);
            if let Err(cleanup_err) = std::fs::remove_file(&salt_path) {
                tracing::warn!("Failed to clean up orphaned salt file {}: {}", salt_path, cleanup_err);
            }
            Err(e)
        }
    }
}

/// Deletes the database file and salt file for a fresh start.
pub fn reset_database(db_path: &str) -> Result<(), DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let mut db_errors = Vec::new();
    let mut salt_errors = Vec::new();

    if std::path::Path::new(db_path).exists() {
        if let Err(e) = std::fs::remove_file(db_path) {
            db_errors.push(format!("Failed to delete DB: {}", e));
        }
    }
    // WAL/SHM files are intentionally ignored on failure:
    // they are transient SQLite files that may or may not exist,
    // and failing to delete them is not a critical error.
    for suffix in &["-wal", "-shm"] {
        let path = format!("{}{}", db_path, suffix);
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
    if std::path::Path::new(&salt_path).exists() {
        if let Err(e) = std::fs::remove_file(&salt_path) {
            salt_errors.push(format!("Failed to delete salt: {}", e));
        }
    }
    // delete_db_key returns () — just call it, ignore any error
    delete_db_key(SERVICE_NAME, KEY_USERNAME);

    if !db_errors.is_empty() {
        Err(DBKeyError::DatabaseCleanupError(db_errors.join("; ")))
    } else if !salt_errors.is_empty() {
        Err(DBKeyError::SaltFileError(salt_errors.join("; ")))
    } else {
        Ok(())
    }
}

/// Replaced by the recovery key flow in init_db().
/// Kept temporarily for compilation. Will be removed in the init_db rewrite.
#[allow(dead_code)]
#[deprecated = "Replaced by the recovery key flow in init_db()"]
pub fn get_or_create_db_key(_db_path: &str) -> Result<String, DBKeyError> {
    match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
        Ok(key) => Ok(key),
        Err(DBKeyError::NoEntry) => Err(DBKeyError::MissingKeyWithDb),
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
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        write_salt(&db_path, &salt).unwrap();
        let read_back = read_salt(&db_path).unwrap();
        assert_eq!(salt, read_back);
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

    #[test]
    fn test_reset_database_removes_files() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("database.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        // Create files
        std::fs::write(&db_path, "test data").unwrap();
        write_salt(&db_path, &salt).unwrap();

        assert!(std::path::Path::new(&db_path).exists());

        reset_database(&db_path).unwrap();

        assert!(!std::path::Path::new(&db_path).exists());
        assert!(!std::path::Path::new(&salt_file_path(&db_path)).exists());
    }

    #[test]
    fn test_generate_and_store_key() {
        cleanup();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("database.db").to_str().unwrap().to_string();

        let key = generate_and_store_key("MyTestPassphrase123", &db_path).unwrap();

        assert_eq!(key.len(), 64);
        assert!(std::path::Path::new(&salt_file_path(&db_path)).exists());

        // Verify the stored key matches what derive_key produces
        let salt = read_salt(&db_path).unwrap();
        let derived = derive_key("MyTestPassphrase123", &salt).unwrap();
        assert_eq!(key, derived);

        // Cleanup: the test writes to SERVICE_NAME (real keyring), not TEST_SERVICE
        delete_db_key(SERVICE_NAME, KEY_USERNAME);
    }
}
