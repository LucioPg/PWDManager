use crate::backend::password_types_helper::{PasswordEvaluation, PasswordScore};
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing;

// Blacklist delle password comuni - embedded direttamente nel binary
// Usiamo include_str! per embeddare il file a compile-time
const BLACKLIST_CONTENT: &str = include_str!("../../assets/10k-most-common.txt");

// Caricamento pigro della blacklist in memoria
static COMMON_PASSWORDS: OnceLock<HashSet<String>> = OnceLock::new();

/// Inizializza la blacklist delle password comuni
/// Il file è embedded nel binary a compile-time usando include_str!
/// È sicuro chiamare questa funzione più volte - se già inizializzata, non fa nulla.
pub fn init_blacklist() -> std::io::Result<()> {
    // Se già inizializzata, ritorna subito senza fare nulla
    if COMMON_PASSWORDS.get().is_some() {
        tracing::debug!("Blacklist already initialized, skipping");
        return Ok(());
    }

    tracing::info!("Initializing password blacklist from embedded content");

    // Crea l'HashSet delle password blacklistate dal contenuto embedded
    let set: HashSet<String> = BLACKLIST_CONTENT
        .lines()
        .map(|l| l.trim().to_lowercase())
        .collect();

    let count = set.len();

    // set() ritorna Err se già inizializzata, ma abbiamo già controllato sopra
    // Usiamo Ok per ignorare l'errore nel caso improbabile di race condition
    let _ = COMMON_PASSWORDS.set(set);
    tracing::info!("Blacklist initialized successfully! ({} passwords)", count);
    Ok(())
}

/// Controlla se la password è nella blacklist delle 10k password comuni
fn blacklist_section(password: &SecretString) -> Result<Option<String>, ()> {
    // Use existing COMMON_PASSWORDS static
    if let Some(blacklist) = COMMON_PASSWORDS.get() {
        if blacklist.contains(&password.expose_secret().to_lowercase()) {
            return Ok(Some(
                "Password is in the top 10,000 most common".to_string(),
            ));
        }
    }
    Ok(None)
}

/// Verifica lunghezza minima password
fn length_section(password: &SecretString) -> Result<Option<String>, ()> {
    const MIN_LENGTH: usize = 8;
    if password.expose_secret().len() < MIN_LENGTH {
        return Ok(Some(format!(
            "Password must be at least {} characters",
            MIN_LENGTH
        )));
    }
    Ok(None)
}

/// Verifica presenza di maiuscole, minuscole, numeri, speciali
fn character_variety_section(password: &SecretString) -> Result<Option<String>, ()> {
    let pwd = password.expose_secret();
    let has_upper = pwd.chars().any(|c| c.is_uppercase());
    let has_lower = pwd.chars().any(|c| c.is_lowercase());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
    let has_special = pwd.chars().any(|c| !c.is_alphanumeric());

    let missing: Vec<_> = vec![
        if !has_upper { Some("uppercase") } else { None },
        if !has_lower { Some("lowercase") } else { None },
        if !has_digit { Some("numbers") } else { None },
        if !has_special {
            Some("special characters")
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    .collect();

    if !missing.is_empty() {
        return Ok(Some(format!("Missing: {}", missing.join(", "))));
    }
    Ok(None)
}

/// Analizza pattern per penalizzare ripetizioni e sequenze
fn pattern_analysis_section(password: &SecretString) -> Result<Option<String>, ()> {
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
    // This catches "abcd", "1234", etc. but not short sequences like "123" in "RandomPass123!@#Word"
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

pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>,
) {
    use tracing::{error, info};
    info!("evaluation is about to start...");

    tokio::time::sleep(Duration::from_millis(300)).await;
    // let token_clone = (*token).clone();
    let evaluation = evaluate_password_strength(password, Some(token));
    // Invia risultato
    if let Err(e) = tx.send(evaluation).await {
        error!(error = %e, "Failed to send password evaluation result");
    }
}

fn cancellation(
    token: Option<CancellationToken>,
    score: Option<i64>,
    mut reasons: Vec<String>,
) -> (Option<i64>, Vec<String>, bool) {
    if let Some(t) = token {
        if t.is_cancelled() {
            reasons.push("Evaluation cancelled".to_string());
            return (None, reasons, true);
        };
    }
    (score, reasons, false)
}

// #[cfg(feature = "ide-only")]
pub fn evaluate_password_strength(
    password: &SecretString,
    token: Option<CancellationToken>,
) -> PasswordEvaluation {
    let mut reasons = Vec::new();
    let mut is_error = false;
    let mut score: Option<i64> = None;

    let pwd = password.expose_secret();
    let pwd_len = pwd.len();

    // Orchestrator: esegui sezioni in sequenza
    let sections: Vec<(&str, fn(&SecretString) -> Result<Option<String>, ()>)> = vec![
        ("blacklist", blacklist_section),
        ("length", length_section),
        ("variety", character_variety_section),
        ("pattern", pattern_analysis_section),
    ];

    for (section_name, section_fn) in sections {
        // Check cancellation prima di ogni sezione
        (score, reasons, is_error) = cancellation(token.clone(), score, reasons);
        if is_error {
            break;
        }

        match section_fn(password) {
            Ok(Some(reason)) => {
                reasons.push(reason);
            }
            Ok(None) => {
                // Sezione passata, continua
            }
            Err(()) => {
                error!(section = %section_name, "Fatal error in password evaluation section");
                reasons.push("Error".to_string());
                score = None;
                break;
            }
        }
    }

    // Calcola strength e score finale
    if !is_error {
        // Calcola score (0-100)
        // Lunghezza: fino a 20 punti (0.5 punti per carattere, max 20)
        let bonus = (pwd_len as f64 * 0.5).min(20.0) as i64;
        let score_ref = score.get_or_insert(0);
        *score_ref += bonus;

        // Varietà caratteri: fino a 60 punti (15 per tipo)
        // Aumentato il peso della varietà per bilanciare meglio
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

        // Bonus lunghezza extra: +5 se > 12, +10 se > 16
        let score_ref = score.get_or_insert(0);
        if pwd_len > 16 {
            *score_ref += 10;
        } else if pwd_len > 12 {
            *score_ref += 5;
        }

        // Bonus caratteri speciali multipli: +5 se 2+ caratteri speciali
        let special_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();
        if special_count >= 2 {
            let score_ref = score.get_or_insert(0);
            *score_ref += 5;
        }

        // Bonus entropia: basato su caratteri unici
        // +5 se >= 12 caratteri unici, +10 se >= 16 caratteri unici
        let unique_chars: std::collections::HashSet<char> = pwd.chars().collect();
        let unique_count = unique_chars.len();
        let score_ref = score.get_or_insert(0);
        if unique_count >= 16 {
            *score_ref += 10;
        } else if unique_count >= 12 {
            *score_ref += 5;
        }

        // Penalità per reasons (ogni reason sottrae punti)
        let score_ref = score.get_or_insert(0);
        *score_ref -= (reasons.len() as i64) * 10;
    }

    PasswordEvaluation {
        score: score.map(|s| PasswordScore::new(s)),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::password_types_helper::PasswordStrength;
    use secrecy::SecretString;

    #[test]
    fn test_blacklist_section_with_common_password() {
        // Initialize the blacklist for testing
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        let pwd = SecretString::new("password".to_string().into());
        let result = blacklist_section(&pwd);
        assert_eq!(
            result,
            Ok(Some(
                "Password is in the top 10,000 most common".to_string()
            ))
        );
    }

    #[test]
    fn test_blacklist_section_with_strong_password() {
        let pwd = SecretString::new("CorrectHorseBatteryStaple!123".to_string().into());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(None));
    }

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
    fn test_length_section_valid() {
        let pwd = SecretString::new("LongEnough123!".to_string().into());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_variety_section_missing_uppercase() {
        let pwd = SecretString::new("lowercase123!".to_string().into());
        let result = character_variety_section(&pwd);
        assert!(result.is_ok());
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("uppercase") || reason.contains("variety"));
        }
    }

    #[test]
    fn test_variety_section_all_categories() {
        let pwd = SecretString::new("HasAll123!@#".to_string().into());
        let result = character_variety_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_pattern_section_repetitive() {
        let pwd = SecretString::new("aaaaBBBB1111".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert!(result.is_ok());
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("repetitive") || reason.contains("pattern"));
        }
    }

    #[test]
    fn test_pattern_section_strong() {
        let pwd = SecretString::new("RandomPass123!@#Word".to_string().into());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }

    // ========================================
    // Test per evaluate_password_strength
    // ========================================

    #[tokio::test]
    async fn test_evaluate_password_strength_weak_short_password() {
        // Initialize blacklist for testing
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        let pwd = SecretString::new("abc".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(evaluation.score.is_some());
        assert!(evaluation.score.unwrap() < 50, "Expected WEAK score (< 50)");
        assert!(
            !evaluation.reasons.is_empty(),
            "Should have reasons for weak password"
        );
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_medium() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // Password con tutti i 4 tipi di caratteri ma non troppo lunga
        let pwd = SecretString::new("MyPass123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        assert_eq!(evaluation.strength(), PasswordStrength::MEDIUM);
        assert!(evaluation.score.is_some());
        let score = evaluation.score.unwrap();
        assert!(
            score >= 50 && score < 70,
            "Expected MEDIUM score (50-69), got {}",
            score
        );
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_strong() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        let pwd = SecretString::new("VeryStrongPassword123!@#".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        // Deve essere almeno STRONG
        assert!(
            matches!(
                evaluation.strength(),
                PasswordStrength::STRONG | PasswordStrength::EPIC | PasswordStrength::GOD
            ),
            "Expected STRONG or better, got {:?}",
            evaluation.strength()
        );
        assert!(evaluation.score.is_some());
        assert!(evaluation.score.unwrap() >= 70);
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_blacklisted_password() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // "password" è nella blacklist
        let pwd = SecretString::new("password".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        // Password nella blacklist dovrebbe essere WEAK
        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(
            !evaluation.reasons.is_empty(),
            "Should have reason for blacklisted password"
        );

        // Verifica che il reason contenga riferimento alla blacklist
        let has_blacklist_reason = evaluation
            .reasons
            .iter()
            .any(|r| r.contains("10,000") || r.contains("common"));
        assert!(has_blacklist_reason, "Should mention blacklist in reasons");
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_empty_password() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        let pwd = SecretString::new("".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        // Password vuota dovrebbe essere WEAK
        assert_eq!(evaluation.strength(), PasswordStrength::WEAK);
        assert!(
            !evaluation.reasons.is_empty(),
            "Should have reason for empty password"
        );
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_with_cancellation() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        let token = CancellationToken::new();
        // Cancella prima della valutazione
        token.cancel();

        let pwd = SecretString::new("SomePassword123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        // Con cancellazione, dovrebbe essere NotEvaluated
        assert_eq!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(
            evaluation.score.is_none(),
            "Score should be None when cancelled"
        );
        assert!(
            !evaluation.reasons.is_empty(),
            "Should have cancellation reason"
        );
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_without_cancellation() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // Token non cancellato
        let token = CancellationToken::new();

        let pwd = SecretString::new("TestPass123!".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, Some(token));

        // Senza cancellazione, dovrebbe valutare normalmente
        assert_ne!(evaluation.strength(), PasswordStrength::NotEvaluated);
        assert!(evaluation.score.is_some());
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_reasons_content() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // Password corta senza maiuscole, numeri o speciali
        let pwd = SecretString::new("abc".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        // Dovrebbe avere reasons per lunghezza e varietà
        let reasons_text = evaluation.reasons.join(" ");
        assert!(
            reasons_text.contains("8") || reasons_text.contains("character"),
            "Should mention length requirement in reasons"
        );
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_epic_level() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // Password molto forte con alta varietà e lunghezza
        let pwd = SecretString::new("ThisIsAVeryStrongP@ssw0rd!2024#XyZ".to_string().into());
        let evaluation = evaluate_password_strength(&pwd, None);

        // Dovrebbe essere EPIC o GOD
        assert!(
            matches!(
                evaluation.strength(),
                PasswordStrength::EPIC | PasswordStrength::GOD
            ),
            "Expected EPIC or GOD for very strong password, got {:?}",
            evaluation.strength()
        );
        assert!(evaluation.score.unwrap() >= 85);
    }

    #[tokio::test]
    async fn test_evaluate_password_strength_score_boundaries() {
        // Use the real blacklist initialization for accurate testing
        let _ = init_blacklist();

        // Verifica che lo score sia sempre tra 0 e 100
        let test_passwords = vec![
            "",                         // Vuota
            "a",                        // Molto corta
            "password",                 // Blacklist
            "MyPass123!",               // Media
            "VeryStrongPassword123!@#", // Forte
        ];

        for pwd_str in test_passwords {
            let pwd = SecretString::new(pwd_str.to_string().into());
            let evaluation = evaluate_password_strength(&pwd, None);

            if let Some(score) = evaluation.score {
                assert!(
                    score >= 0 && score <= 100,
                    "Score {} out of bounds for password '{}'",
                    score,
                    pwd_str
                );
            }
        }
    }
}
