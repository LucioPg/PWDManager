use crate::backend::password_types_helper::{PasswordEvaluation, PasswordStrength};
use ::std::panic;
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
pub fn init_blacklist() -> std::io::Result<()> {
    tracing::info!("Initializing password blacklist from embedded content");

    // Crea l'HashSet delle password blacklistate dal contenuto embedded
    let set: HashSet<String> = BLACKLIST_CONTENT
        .lines()
        .map(|l| l.trim().to_lowercase())
        .collect();

    let count = set.len();
    COMMON_PASSWORDS
        .set(set)
        .expect("Blacklist already initialized");
    tracing::info!("Blacklist initialized successfully! ({} passwords)", count);
    Ok(())
}

/// Legacy: inizializza la blacklist da un percorso file (mantenuto per compatibilità)
#[deprecated(note = "Usare init_blacklist() senza parametri")]
pub fn init_blacklist_from_path(_file_path: &str) -> std::io::Result<()> {
    init_blacklist()
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
    let evaluation = evaluate_password_strength(password, Some(token)).await;
    // Invia risultato
    if let Err(e) = tx.send(evaluation).await {
        error!(error = %e, "Failed to send password evaluation result");
    }
}

fn cancellation(
    token: Option<CancellationToken>,
    mut strength: PasswordStrength,
    mut reasons: Vec<String>,
    mut is_error: bool,
) -> (PasswordStrength, Vec<String>, bool) {
    if token.is_some() {
        if token.unwrap().is_cancelled() {
            strength = PasswordStrength::NotEvaluated;
            reasons.push("Evaluation cancelled".to_string());
            is_error = true;
            return (strength, reasons, is_error);
        };
    }
    (strength, reasons, is_error)
}

// #[cfg(feature = "ide-only")]
pub async fn evaluate_password_strength(
    password: &SecretString,
    token: Option<CancellationToken>,
) -> PasswordEvaluation {
    let mut reasons = Vec::new();
    let mut strength = PasswordStrength::NotEvaluated;
    let mut is_error = false;
    let mut score: i32 = 0;

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
        (strength, reasons, is_error) = cancellation(token.clone(), strength, reasons, is_error);
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
                is_error = true;
                strength = PasswordStrength::NotEvaluated;
                break;
            }
        }
    }

    // Calcola strength e score finale
    if !is_error {
        // Calcola score (0-100)
        // Lunghezza: fino a 20 punti (0.5 punti per carattere, max 20)
        score += (pwd_len as f64 * 0.5).min(20.0) as i32;

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
        score += (variety_count * 15) as i32;

        // Bonus lunghezza extra: +5 se > 12, +10 se > 16
        if pwd_len > 16 {
            score += 10;
        } else if pwd_len > 12 {
            score += 5;
        }

        // Bonus caratteri speciali multipli: +5 se 2+ caratteri speciali
        let special_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();
        if special_count >= 2 {
            score += 5;
        }

        // Bonus entropia: basato su caratteri unici
        // +5 se >= 12 caratteri unici, +10 se >= 16 caratteri unici
        let unique_chars: std::collections::HashSet<char> = pwd.chars().collect();
        let unique_count = unique_chars.len();
        if unique_count >= 16 {
            score += 10;
        } else if unique_count >= 12 {
            score += 5;
        }

        // Penalità per reasons (ogni reason sottrae punti)
        score -= (reasons.len() as i32) * 10;

        // Clampa score tra 0 e 100
        score = score.clamp(0, 100);

        // Determina strength basata su score con i nuovi livelli
        // Soglie aggiornate per rendere MEDIUM più ampia
        strength = if score > 95 {
            PasswordStrength::GOD
        } else if score >= 85 {
            PasswordStrength::EPIC
        } else if score >= 70 {
            PasswordStrength::STRONG
        } else if score >= 50 {
            PasswordStrength::MEDIUM
        } else {
            PasswordStrength::WEAK
        };
    }

    PasswordEvaluation {
        strength,
        reasons,
        score: if is_error { None } else { Some(score) },
    }
}

// pub async fn evaluate_password_strength(
//     password: &SecretString,
//     token: CancellationToken,
// ) -> PasswordStrength {
//     let password_clone = password.clone();
//
//     let strength = tokio::task::spawn_blocking(move || {
//         if token.is_cancelled() {
//             return Err(PasswordStrength::WEAK);
//         }
//
//         let pass_ref = password_clone.expose_secret();
//
//         // 1. Controllo Blacklist (Sola lettura, thread-safe)
//         if let Some(blacklist) = COMMON_PASSWORDS.get() {
//             if blacklist.contains(&pass_ref.to_lowercase()) {
//                 return Err(PasswordStrength::WEAK);
//             }
//         } else {
//             println!("Attenzione Blacklist: NON CARICATA");
//         }
//
//         let chars: Vec<char> = pass_ref.chars().collect();
//         let len = chars.len();
//
//         if len == 0 {
//             return Err(PasswordStrength::WEAK);
//         }
//
//         Ok(calculate_internal_score(chars))
//     })
//     .await;
//     match strength {
//         Ok(Ok(s)) => s,
//         Ok(Err(s)) => s,
//         Err(_) => PasswordStrength::WEAK,
//     }
// }

fn calculate_internal_score(chars: Vec<char>) -> PasswordStrength {
    let result = panic::catch_unwind(move || {
        let mut score: i32 = 0;
        let len = chars.len();
        // 2. Analisi Varietà
        let has_upper = chars.iter().any(|c| c.is_uppercase());
        let has_lower = chars.iter().any(|c| c.is_lowercase());
        let has_digit = chars.iter().any(|c| c.is_digit(10));
        let has_special = chars.iter().any(|c| !c.is_alphanumeric());

        // Lunghezza: 0.5 pt per carattere (max 20)
        score += ((len as f64) * 0.5).min(20.0) as i32;

        // Varietà: 15 pt per tipo (max 60)
        if has_upper {
            score += 15;
        }
        if has_lower {
            score += 15;
        }
        if has_digit {
            score += 15;
        }
        if has_special {
            score += 15;
        }

        // 3. Penalità Ripetizioni (es. "aaaa")
        let mut repeats = 0;
        for i in 0..len.saturating_sub(1) {
            if chars[i] == chars[i + 1] {
                repeats += 1;
            }
        }
        score -= repeats * 5;

        // 4. Penalità Sequenze (es. "abc", "123")
        let mut sequences = 0;
        for window in chars.windows(3) {
            let v0 = window[0] as i32;
            let v1 = window[1] as i32;
            let v2 = window[2] as i32;
            if (v1 == v0 + 1 && v2 == v1 + 1) || (v1 == v0 - 1 && v2 == v1 - 1) {
                sequences += 1;
            }
        }
        score -= sequences * 10;

        // 5. Valutazione Finale con nuovi livelli
        let types_count = [has_upper, has_lower, has_digit, has_special]
            .iter()
            .filter(|&&b| b)
            .count();

        // Se troppo corta o solo un tipo di carattere, non può essere oltre WEAK
        if len < 8 || types_count < 2 {
            score = score.min(49);
        }

        score = score.clamp(0, 100);

        // Soglie aggiornate per rendere MEDIUM più ampia
        match score {
            s if s > 95 => PasswordStrength::GOD,
            s if s >= 85 => PasswordStrength::EPIC,
            s if s >= 70 => PasswordStrength::STRONG,
            s if s >= 50 => PasswordStrength::MEDIUM,
            _ => PasswordStrength::WEAK,
        }
    });
    result.unwrap_or_else(|_| {
        eprintln!("Error evaluating password strength");
        PasswordStrength::WEAK
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_blacklist_section_with_common_password() {
        // Initialize the blacklist for testing
        let _ = COMMON_PASSWORDS.get_or_init(|| {
            vec!["password".to_string(), "123456".to_string()]
                .into_iter()
                .collect()
        });

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
}
