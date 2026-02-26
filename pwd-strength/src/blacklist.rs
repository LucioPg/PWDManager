//! Blacklist management module
//!
//! Handles loading and querying the password blacklist.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::RwLock;
use thiserror::Error;

static COMMON_PASSWORDS: RwLock<Option<HashSet<String>>> = RwLock::new(None);

#[derive(Error, Debug)]
pub enum BlacklistError {
    #[error("Blacklist file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to read blacklist file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Blacklist file is empty")]
    EmptyFile,
}

/// Returns the blacklist file path.
///
/// Priority:
/// 1. Environment variable `PWD_BLACKLIST_PATH`
/// 2. Default path `./assets/10k-most-common.txt`
pub fn get_blacklist_path() -> PathBuf {
    std::env::var("PWD_BLACKLIST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./assets/10k-most-common.txt"))
}

/// Initializes the password blacklist from external file.
///
/// # Environment Variable
///
/// Set `PWD_BLACKLIST_PATH` to specify a custom blacklist file location.
/// If not set, defaults to `./assets/10k-most-common.txt`.
///
/// # Errors
///
/// Returns error if:
/// - File does not exist
/// - File cannot be read
/// - File is empty
///
/// # Example
///
/// ```rust,ignore
/// // Custom path via environment
/// unsafe { std::env::set_var("PWD_BLACKLIST_PATH", "/etc/myapp/blacklist.txt"); }
/// pwd_strength::init_blacklist()?;
///
/// // Or use default path
/// pwd_strength::init_blacklist()?;
/// ```
pub fn init_blacklist() -> Result<usize, BlacklistError> {
    // Idempotente: se gia inizializzata, ritorna subito
    {
        let guard = COMMON_PASSWORDS.read().unwrap();
        if guard.is_some() {
            return Ok(guard.as_ref().map(|s| s.len()).unwrap_or(0));
        }
    }

    let path = get_blacklist_path();

    if !path.exists() {
        return Err(BlacklistError::FileNotFound(path));
    }

    let content = std::fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Err(BlacklistError::EmptyFile);
    }

    let set: HashSet<String> = content
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect();

    let count = set.len();
    {
        let mut guard = COMMON_PASSWORDS.write().unwrap();
        *guard = Some(set);
    }

    #[cfg(feature = "tracing")]
    tracing::info!("Blacklist initialized: {} passwords from {:?}", count, path);

    Ok(count)
}

/// Returns a cloned reference to the loaded blacklist.
///
/// Returns `None` if `init_blacklist()` has not been called.
pub fn get_blacklist() -> Option<HashSet<String>> {
    let guard = COMMON_PASSWORDS.read().unwrap();
    guard.clone()
}

/// Checks if a password is in the blacklist.
///
/// Returns `true` if password is in the blacklist (case-insensitive).
/// Returns `false` if blacklist is not initialized or password is not found.
pub fn is_blacklisted(password: &str) -> bool {
    let guard = COMMON_PASSWORDS.read().unwrap();
    guard
        .as_ref()
        .map(|bl| bl.contains(&password.to_lowercase()))
        .unwrap_or(false)
}

/// Resets the blacklist for testing purposes.
#[cfg(test)]
pub fn reset_blacklist_for_testing() {
    let mut guard = COMMON_PASSWORDS.write().unwrap();
    *guard = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use serial_test::serial;

    /// Helper to safely set env var in tests
    fn set_env(key: &str, value: &str) {
        // SAFETY: This is only for testing purposes in single-threaded test context
        unsafe { std::env::set_var(key, value); }
    }

    /// Helper to safely remove env var in tests
    fn remove_env(key: &str) {
        // SAFETY: This is only for testing purposes in single-threaded test context
        unsafe { std::env::remove_var(key); }
    }

    #[test]
    #[serial]
    fn test_get_blacklist_path_default() {
        // Remove env var if set
        remove_env("PWD_BLACKLIST_PATH");

        let path = get_blacklist_path();
        assert_eq!(path, PathBuf::from("./assets/10k-most-common.txt"));
    }

    #[test]
    #[serial]
    fn test_get_blacklist_path_from_env() {
        let custom_path = "/custom/path/blacklist.txt";
        set_env("PWD_BLACKLIST_PATH", custom_path);

        let path = get_blacklist_path();
        assert_eq!(path, PathBuf::from(custom_path));

        // Cleanup
        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_init_blacklist_file_not_found() {
        reset_blacklist_for_testing();
        set_env("PWD_BLACKLIST_PATH", "/nonexistent/path/blacklist.txt");

        let result = init_blacklist();
        assert!(result.is_err());

        match result {
            Err(BlacklistError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }

        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_init_blacklist_empty_file() {
        reset_blacklist_for_testing();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "").expect("Failed to write empty content");

        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        let result = init_blacklist();
        assert!(matches!(result, Err(BlacklistError::EmptyFile)));

        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_init_blacklist_success() {
        reset_blacklist_for_testing();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "password123").expect("Failed to write");
        writeln!(temp_file, "qwerty").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        let result = init_blacklist();
        assert!(result.is_ok());

        let count = result.unwrap();
        assert_eq!(count, 2);

        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_is_blacklisted_true() {
        reset_blacklist_for_testing();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "testpassword").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        // Initialize with our test file
        let _ = init_blacklist();

        assert!(is_blacklisted("testpassword"));
        assert!(is_blacklisted("TESTPASSWORD")); // case insensitive

        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_is_blacklisted_false() {
        reset_blacklist_for_testing();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "common123").expect("Failed to write");

        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        let _ = init_blacklist();

        assert!(!is_blacklisted("veryuncommonpassword987"));

        remove_env("PWD_BLACKLIST_PATH");
    }
}
