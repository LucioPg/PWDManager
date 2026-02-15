use ::std::panic;
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq)]
pub struct PasswordEvaluation {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}

// Definisci l'asset della blacklist usando il sistema manganis di Dioxus 0.7.3
// L'attributo #[used] forza l'inclusione dell'asset anche se non referenziato direttamente nel RSX
#[used]
static BLACKLIST_ASSET: Asset = asset!(
    "/assets/10k-most-common.txt",
    AssetOptions::builder().with_hash_suffix(false)
);

// Caricamento pigro della blacklist in memoria
static COMMON_PASSWORDS: OnceLock<HashSet<String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PasswordStrength {
    NotEvaluated,
    WEAK,
    MEDIUM,
    STRONG,
}

/// Inizializza la blacklist dall'asset Dioxus
/// L'asset viene automaticamente copiato nella cartella assets/ accanto all'eseguibile durante il bundle
pub fn init_blacklist() -> std::io::Result<()> {
    println!("Caricamento blacklist dall'asset Dioxus...");

    // Ottieni il percorso dell'asset
    // In desktop, gli asset sono copiati nella cartella "assets/" accanto all'eseguibile
    let asset_path_str = BLACKLIST_ASSET.to_string();
    let asset_path = PathBuf::from(asset_path_str.trim_start_matches('/'));

    println!("Percorso asset: {:?}", asset_path);

    // Leggi il contenuto del file
    let content = std::fs::read_to_string(&asset_path).map_err(|e| {
        eprintln!("Errore lettura blacklist da {:?}: {}", asset_path, e);
        e
    })?;

    // Crea l'HashSet delle password blacklistate
    let set: HashSet<String> = content.lines().map(|l| l.trim().to_lowercase()).collect();

    let count = set.len();
    let _ = COMMON_PASSWORDS.set(set);
    println!("Blacklist caricata con successo! ({} password)", count);
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
            return Ok(Some("Password is in the top 10,000 most common".to_string()));
        }
    }
    Ok(None)
}

/// Verifica lunghezza minima password
fn length_section(password: &SecretString) -> Result<Option<String>, ()> {
    const MIN_LENGTH: usize = 8;
    if password.expose_secret().len() < MIN_LENGTH {
        return Ok(Some(format!("Password must be at least {} characters", MIN_LENGTH)));
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
        if !has_special { Some("special characters") } else { None },
    ].into_iter().flatten().collect();

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
    use tracing::error;

    let mut reasons = Vec::new();
    let mut strength = PasswordStrength::NotEvaluated;

    // Orchestrator: esegui sezioni in sequenza
    let sections: Vec<(&str, fn(&SecretString) -> Result<Option<String>, ()>)> = vec![
        ("blacklist", blacklist_section),
        ("length", length_section),
        ("variety", character_variety_section),
        ("pattern", pattern_analysis_section),
    ];

    for (section_name, section_fn) in sections {
        // Check cancellation prima di ogni sezione
        if token.is_cancelled() {
            strength = PasswordStrength::NotEvaluated;
            reasons.push("Evaluation cancelled".to_string());
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
                strength = PasswordStrength::NotEvaluated;
                break;
            }
        }
    }

    // Calcola strength finale basata su reasons
    if strength != PasswordStrength::NotEvaluated {
        strength = if reasons.is_empty() {
            PasswordStrength::STRONG
        } else if reasons.len() <= 2 {
            PasswordStrength::MEDIUM
        } else {
            PasswordStrength::WEAK
        };
    }

    let evaluation = PasswordEvaluation { strength, reasons };

    // Invia risultato
    if let Err(e) = tx.send(evaluation).await {
        error!(error = %e, "Failed to send password evaluation result");
    }
}

pub async fn evaluate_password_strength(
    password: &SecretString,
    token: CancellationToken,
) -> PasswordStrength {
    let password_clone = password.clone();

    let strength = tokio::task::spawn_blocking(move || {
        if token.is_cancelled() {
            return Err(PasswordStrength::WEAK);
        }

        let pass_ref = password_clone.expose_secret();

        // 1. Controllo Blacklist (Sola lettura, thread-safe)
        if let Some(blacklist) = COMMON_PASSWORDS.get() {
            if blacklist.contains(&pass_ref.to_lowercase()) {
                return Err(PasswordStrength::WEAK);
            }
        } else {
            println!("Attenzione Blacklist: NON CARICATA");
        }

        let chars: Vec<char> = pass_ref.chars().collect();
        let len = chars.len();

        if len == 0 {
            return Err(PasswordStrength::WEAK);
        }

        Ok(calculate_internal_score(chars))
    })
    .await;
    match strength {
        Ok(Ok(s)) => s,
        Ok(Err(s)) => s,
        Err(_) => PasswordStrength::WEAK,
    }
}

fn calculate_internal_score(chars: Vec<char>) -> PasswordStrength {
    let result = panic::catch_unwind(move || {
        let mut score: i32 = 0;
        let len = chars.len();
        // 2. Analisi Varietà
        let has_upper = chars.iter().any(|c| c.is_uppercase());
        let has_lower = chars.iter().any(|c| c.is_lowercase());
        let has_digit = chars.iter().any(|c| c.is_digit(10));
        let has_special = chars.iter().any(|c| !c.is_alphanumeric());

        score += (len as i32) * 4;
        if has_upper {
            score += 10;
        }
        if has_lower {
            score += 10;
        }
        if has_digit {
            score += 10;
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

        // 5. Valutazione Finale
        let types_count = [has_upper, has_lower, has_digit, has_special]
            .iter()
            .filter(|&&b| b)
            .count();

        // Se troppo corta o solo un tipo di carattere, non può essere Medium o Strong
        if len < 8 || types_count < 2 {
            score = score.min(45);
        }

        match score {
            s if s < 50 => PasswordStrength::WEAK,
            s if s < 85 => PasswordStrength::MEDIUM,
            _ => PasswordStrength::STRONG,
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
            vec!["password".to_string(), "123456".to_string()].into_iter().collect()
        });

        let pwd = SecretString::new("password".to_string().into());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(Some("Password is in the top 10,000 most common".to_string())));
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
        assert_eq!(result, Ok(Some("Password must be at least 8 characters".to_string())));
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
