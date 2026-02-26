//! Pattern analysis section - detects repetitive and sequential patterns.

use secrecy::{ExposeSecret, SecretString};
use super::SectionResult;

/// Analyzes password for repetitive and sequential patterns.
///
/// # Returns
/// - `Ok(Some(reason))` if problematic patterns found
/// - `Ok(None)` if no problematic patterns
pub fn pattern_analysis_section(password: &SecretString) -> SectionResult {
    let chars: Vec<char> = password.expose_secret().chars().collect();
    if chars.len() < 3 {
        return Ok(None);
    }

    // Check repeated chars (e.g., "aaa")
    let mut repeated_count = 1;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            repeated_count += 1;
            if repeated_count >= 3 {
                return Ok(Some("Password contains repetitive patterns".to_string()));
            }
        } else {
            repeated_count = 1;
        }
    }

    // Check for longer sequences (4+ consecutive characters)
    for window_size in [4, 5] {
        if chars.len() < window_size {
            continue;
        }

        for i in window_size..=chars.len() {
            let window = &chars[i - window_size..i];

            // Check if all characters in window are sequential
            let is_sequential = window.windows(2).all(|w| {
                let prev = w[0] as i32;
                let curr = w[1] as i32;
                curr == prev + 1 || curr == prev - 1
            });

            if is_sequential {
                return Ok(Some("Password contains sequential patterns".to_string()));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_section_repetitive_chars() {
        let pwd = SecretString::new("aaaaBBBB1111".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("repetitive"));
        }
    }

    #[test]
    fn test_pattern_section_sequential_numbers() {
        let pwd = SecretString::new("test1234abcd".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("sequential"));
        }
    }

    #[test]
    fn test_pattern_section_sequential_letters() {
        let pwd = SecretString::new("abcdTest123".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(matches!(result, Ok(Some(_))));
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("sequential"));
        }
    }

    #[test]
    fn test_pattern_section_strong_password() {
        let pwd = SecretString::new("RandomPass123!@#Word".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_pattern_section_too_short() {
        let pwd = SecretString::new("ab".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
