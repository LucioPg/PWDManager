//! Length section - checks password minimum length.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

const MIN_LENGTH: usize = 8;

/// Checks if the password meets minimum length requirements.
///
/// # Returns
/// - `Ok(Some(reason))` if password is too short
/// - `Ok(None)` if password has sufficient length
pub fn length_section(password: &SecretString) -> SectionResult {
    if password.expose_secret().len() < MIN_LENGTH {
        return Ok(Some(format!(
            "Password must be at least {} characters",
            MIN_LENGTH
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_section_too_short() {
        let pwd = SecretString::new("Short1!".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(
            result,
            Ok(Some("Password must be at least 8 characters".to_string()))
        );
    }

    #[test]
    fn test_length_section_exactly_minimum() {
        let pwd = SecretString::new("12345678".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_length_section_valid() {
        let pwd = SecretString::new("LongEnough123!".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
