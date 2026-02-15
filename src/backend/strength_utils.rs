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

pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordStrength>,
) {
    let password_clone = password.clone();

    tokio::task::spawn_blocking(move || {
        if token.is_cancelled() {
            return;
        }

        let pass_ref = password_clone.expose_secret();

        // 1. Controllo Blacklist (Sola lettura, thread-safe)
        if let Some(blacklist) = COMMON_PASSWORDS.get() {
            if blacklist.contains(&pass_ref.to_lowercase()) {
                let _ = tx.blocking_send(PasswordStrength::WEAK);
                return;
            }
        } else {
            println!("Attenzione Blacklist: NON CARICATA");
        }

        let chars: Vec<char> = pass_ref.chars().collect();
        let len = chars.len();

        if len == 0 {
            let _ = tx.blocking_send(PasswordStrength::WEAK);
            return;
        }

        let strength = calculate_internal_score(chars);

        let _ = tx.blocking_send(strength);
    });
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
