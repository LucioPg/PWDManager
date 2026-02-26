//! Password strength evaluator - main evaluation logic.

use pwd_types::{PasswordEvaluation, PasswordScore};
use secrecy::{ExposeSecret, SecretString};

#[cfg(feature = "async")]
use tokio::sync::mpsc;

#[cfg(feature = "async")]
use tokio_util::sync::CancellationToken;

use crate::sections::{
    blacklist_section, character_variety_section, length_section, pattern_analysis_section,
};

/// Evaluates password strength and returns a detailed evaluation.
///
/// # Arguments
/// * `password` - The password to evaluate
/// * `token` - Optional cancellation token (async feature only)
///
/// # Returns
/// A `PasswordEvaluation` containing score and reasons.
pub fn evaluate_password_strength(
    password: &SecretString,
    #[cfg(feature = "async")] token: Option<CancellationToken>,
) -> PasswordEvaluation {
    let mut reasons = Vec::new();
    let mut is_cancelled = false;
    let mut score: Option<i64> = None;

    let pwd = password.expose_secret();
    let pwd_len = pwd.len();

    // Orchestrator: execute sections in sequence
    let sections: Vec<(&str, fn(&SecretString) -> Result<Option<String>, ()>)> = vec![
        ("blacklist", blacklist_section),
        ("length", length_section),
        ("variety", character_variety_section),
        ("pattern", pattern_analysis_section),
    ];

    for (section_name, section_fn) in sections {
        // Check cancellation before each section (async only)
        #[cfg(feature = "async")]
        {
            if let Some(ref t) = token {
                if t.is_cancelled() {
                    reasons.push("Evaluation cancelled".to_string());
                    is_cancelled = true;
                    break;
                }
            }
        }

        match section_fn(password) {
            Ok(Some(reason)) => {
                reasons.push(reason);
            }
            Ok(None) => {
                // Section passed, continue
            }
            Err(()) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Fatal error in password evaluation section: {}", section_name);
                reasons.push("Error".to_string());
                score = None;
                break;
            }
        }
    }

    // Calculate strength and final score
    if !is_cancelled {
        // Length bonus: up to 20 points (0.5 per character, max 20)
        let bonus = (pwd_len as f64 * 0.5).min(20.0) as i64;
        let score_ref = score.get_or_insert(0);
        *score_ref += bonus;

        // Character variety: up to 60 points (15 per type)
        let has_upper = pwd.chars().any(|c| c.is_uppercase());
        let has_lower = pwd.chars().any(|c| c.is_lowercase());
        let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
        let has_special = pwd.chars().any(|c| !c.is_alphanumeric());
        let variety_count = [has_upper, has_lower, has_digit, has_special]
            .iter()
            .filter(|&&b| b)
            .count();
        let score_ref = score.get_or_insert(0);
        *score_ref += (variety_count * 15) as i64;

        // Extra length bonus: +5 if > 12, +10 if > 16
        let score_ref = score.get_or_insert(0);
        if pwd_len > 16 {
            *score_ref += 10;
        } else if pwd_len > 12 {
            *score_ref += 5;
        }

        // Multiple special chars bonus: +5 if 2+ special chars
        let special_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();
        if special_count >= 2 {
            let score_ref = score.get_or_insert(0);
            *score_ref += 5;
        }

        // Entropy bonus: based on unique chars
        let unique_chars: std::collections::HashSet<char> = pwd.chars().collect();
        let unique_count = unique_chars.len();
        let score_ref = score.get_or_insert(0);
        if unique_count >= 16 {
            *score_ref += 10;
        } else if unique_count >= 12 {
            *score_ref += 5;
        }

        // Penalties for reasons (each reason subtracts points)
        let score_ref = score.get_or_insert(0);
        *score_ref -= (reasons.len() as i64) * 10;
    }

    PasswordEvaluation {
        score: score.map(|s| PasswordScore::new(s)),
        reasons,
    }
}

/// Async version that sends evaluation result via channel.
#[cfg(feature = "async")]
pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>,
) {
    use std::time::Duration;

    #[cfg(feature = "tracing")]
    tracing::info!("evaluation is about to start...");

    tokio::time::sleep(Duration::from_millis(300)).await;
    let evaluation = evaluate_password_strength(password, Some(token));

    if let Err(e) = tx.send(evaluation).await {
        #[cfg(feature = "tracing")]
        tracing::error!("Failed to send password evaluation result: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwd_types::PasswordStrength;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use serial_test::serial;

    fn setup_with_tempfile(passwords: &[&str]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        for pwd in passwords {
            writeln!(temp_file, "{}", pwd).expect("Failed to write");
        }
        temp_file
    }

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value); }
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key); }
    }

    fn setup_blacklist() {
        crate::blacklist::reset_blacklist_for_testing();
        let temp_file = setup_with_tempfile(&["password", "123456", "qwerty", "admin"]);
        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);
        let _ = crate::blacklist::init_blacklist();
    }

    fn cleanup_blacklist() {
        remove_env("PWD_BLACKLIST_PATH");
    }

    #[test]
    #[serial]
    fn test_evaluate_weak_short_password() {
        setup_blacklist();
        let pwd = SecretString::new("abc".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(evaluation.score.is_some());
        assert!(evaluation.score.unwrap().value() < 50);
        assert!(!evaluation.reasons.is_empty());

        cleanup_blacklist();
    }

    #[test]
    #[serial]
    fn test_evaluate_medium_password() {
        setup_blacklist();
        let pwd = SecretString::new("MyPass123!".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::MEDIUM);
        let score = evaluation.score.unwrap();
        assert!(score.value() >= 50 && score.value() < 70, "Expected MEDIUM score (50-69), got {}", score.value());

        cleanup_blacklist();
    }

    #[test]
    #[serial]
    fn test_evaluate_strong_password() {
        setup_blacklist();
        let pwd = SecretString::new("VeryStrongPassword123!@#".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert!(matches!(
            evaluation.strength(),
            PasswordStrength::STRONG | PasswordStrength::EPIC | PasswordStrength::GOD
        ));
        assert!(evaluation.score.unwrap().value() >= 70);

        cleanup_blacklist();
    }

    #[test]
    #[serial]
    fn test_evaluate_blacklisted_password() {
        setup_blacklist();
        let pwd = SecretString::new("password".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        let has_blacklist_reason = evaluation.reasons.iter()
            .any(|r| r.contains("10,000") || r.contains("common"));
        assert!(has_blacklist_reason);

        cleanup_blacklist();
    }

    #[test]
    #[serial]
    fn test_evaluate_empty_password() {
        setup_blacklist();
        let pwd = SecretString::new("".to_string().into());

        #[cfg(feature = "async")]
        let evaluation = evaluate_password_strength(&pwd, None);

        #[cfg(not(feature = "async"))]
        let evaluation = evaluate_password_strength(&pwd);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(!evaluation.reasons.is_empty());

        cleanup_blacklist();
    }

    #[test]
    #[serial]
    fn test_evaluate_score_boundaries() {
        setup_blacklist();
        let test_passwords = vec![
            "",
            "a",
            "password",
            "MyPass123!",
            "VeryStrongPassword123!@#",
        ];

        for pwd_str in test_passwords {
            let pwd = SecretString::new(pwd_str.to_string().into());

            #[cfg(feature = "async")]
            let evaluation = evaluate_password_strength(&pwd, None);

            #[cfg(not(feature = "async"))]
            let evaluation = evaluate_password_strength(&pwd);

            if let Some(score) = evaluation.score {
                assert!(
                    score.value() <= 100,
                    "Score {} out of bounds for password '{}'",
                    score.value(),
                    pwd_str
                );
            }
        }

        cleanup_blacklist();
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use pwd_types::PasswordStrength;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use serial_test::serial;

    fn setup_with_tempfile(passwords: &[&str]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        for pwd in passwords {
            writeln!(temp_file, "{}", pwd).expect("Failed to write");
        }
        temp_file
    }

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value); }
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key); }
    }

    fn setup_blacklist() {
        crate::blacklist::reset_blacklist_for_testing();
        let temp_file = setup_with_tempfile(&["password", "123456", "qwerty", "admin"]);
        let path = temp_file.path().to_str().unwrap();
        set_env("PWD_BLACKLIST_PATH", path);
        let _ = crate::blacklist::init_blacklist();
    }

    fn cleanup_blacklist() {
        remove_env("PWD_BLACKLIST_PATH");
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_with_cancellation() {
        setup_blacklist();
        let token = CancellationToken::new();
        token.cancel();

        let pwd = SecretString::new("SomePassword123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        assert_eq!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(evaluation.score.is_none());
        assert!(!evaluation.reasons.is_empty());

        cleanup_blacklist();
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_without_cancellation() {
        setup_blacklist();
        let token = CancellationToken::new();

        let pwd = SecretString::new("TestPass123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        assert_ne!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(evaluation.score.is_some());

        cleanup_blacklist();
    }

    #[tokio::test]
    #[serial]
    async fn test_evaluate_password_strength_tx() {
        setup_blacklist();
        let (tx, mut rx) = mpsc::channel(1);
        let token = CancellationToken::new();

        let pwd = SecretString::new("TestPass123!".to_string().into());

        evaluate_password_strength_tx(&pwd, token, tx).await;

        let evaluation = rx.recv().await.expect("Should receive evaluation");
        assert!(evaluation.score.is_some());

        cleanup_blacklist();
    }
}
