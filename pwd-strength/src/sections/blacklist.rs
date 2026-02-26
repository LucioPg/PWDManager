//! Blacklist section - checks if password is in common password list.

use crate::blacklist::is_blacklisted;
use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Checks if the password is in the blacklist of common passwords.
///
/// # Returns
/// - `Ok(Some(reason))` if password is blacklisted
/// - `Ok(None)` if password is not in blacklist
pub fn blacklist_section(password: &SecretString) -> SectionResult {
    if is_blacklisted(password.expose_secret()) {
        return Ok(Some(
            "Password is in the top 10,000 most common".to_string(),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to safely set env var in tests
    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value); }
    }

    /// Helper to safely remove env var in tests
    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key); }
    }

    fn setup_with_tempfile(passwords: &[&str]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        for pwd in passwords {
            writeln!(temp_file, "{}", pwd).expect("Failed to write");
        }
        temp_file
    }

    #[test]
    #[serial]
    fn test_blacklist_section_common_password() {
        crate::blacklist::reset_blacklist_for_testing();

        let temp_file = setup_with_tempfile(&["password", "123456", "qwerty"]);
        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        let _ = crate::blacklist::init_blacklist();

        let pwd = SecretString::new("password".to_string().into());
        let result = blacklist_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));

        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_blacklist_section_strong_password() {
        crate::blacklist::reset_blacklist_for_testing();

        let temp_file = setup_with_tempfile(&["password", "123456", "qwerty"]);
        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);

        let _ = crate::blacklist::init_blacklist();

        let pwd = SecretString::new("CorrectHorseBatteryStaple!123".to_string().into());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(None));

        remove_env("PWD_BLACKLIST_PATH");
    }
}
