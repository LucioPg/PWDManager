//! Character variety section - checks for uppercase, lowercase, numbers, special chars.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Checks if the password contains a variety of character types.
///
/// # Returns
/// - `Ok(Some(reason))` if missing required character types
/// - `Ok(None)` if all character types are present
pub fn character_variety_section(password: &SecretString) -> SectionResult {
    let pwd = password.expose_secret();
    let has_upper = pwd.chars().any(|c| c.is_uppercase());
    let has_lower = pwd.chars().any(|c| c.is_lowercase());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
    let has_special = pwd.chars().any(|c| !c.is_alphanumeric());

    let missing: Vec<_> = vec![
        if !has_upper { Some("uppercase") } else { None },
        if !has_lower { Some("lowercase") } else { None },
        if !has_digit { Some("numbers") } else { None },
        if !has_special { Some("special characters") } else { None },
    ]
    .into_iter()
    .flatten()
    .collect();

    if !missing.is_empty() {
        return Ok(Some(format!("Missing: {}", missing.join(", "))));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variety_section_missing_uppercase() {
        let pwd = SecretString::new("lowercase123!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("uppercase"));
        }
    }

    #[test]
    fn test_variety_section_missing_lowercase() {
        let pwd = SecretString::new("UPPERCASE123!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("lowercase"));
        }
    }

    #[test]
    fn test_variety_section_missing_numbers() {
        let pwd = SecretString::new("NoNumbers!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("numbers"));
        }
    }

    #[test]
    fn test_variety_section_missing_special() {
        let pwd = SecretString::new("NoSpecial123".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("special"));
        }
    }

    #[test]
    fn test_variety_section_all_categories() {
        let pwd = SecretString::new("HasAll123!@#".to_string().into());
        let result = character_variety_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
