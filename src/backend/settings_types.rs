//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::FromRow;
use sqlx_template::{SqliteTemplate, SqlxTemplate};

/// Preset per la generazione password.
///
/// I valori sono calcolati per garantire che la password generata
/// rientri nella fascia di strength corretta secondo `strength_utils`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPreset {
    Medium,
    Strong,
    Epic,
    God,
}

impl PasswordPreset {
    /// Restituisce la configurazione per questo preset.
    ///
    /// # Valori calcolati da strength_utils
    ///
    /// | Preset | length | symbols | Score |
    /// |--------|--------|---------|-------|
    /// | Medium | 8 | 2 | 69 |
    /// | Strong | 12 | 2 | 81 |
    /// | Epic | 16 | 2 | 93 |
    /// | God | 26 | 2 | 98 |
    pub fn to_config(&self) -> PasswordGenConfig {
        match self {
            Self::Medium => PasswordGenConfig {
                length: 8,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::Strong => PasswordGenConfig {
                length: 12,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::Epic => PasswordGenConfig {
                length: 16,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::God => PasswordGenConfig {
                length: 26,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
        }
    }
}

/// Configurazione per la generazione password (in memoria).
///
/// Usata per passare i parametri di configurazione senza
/// dipendere dal database.
///
/// Nota: usa i64 per coerenza con il resto del codebase.
#[derive(Debug, Clone, Copy)]
pub struct PasswordGenConfig {
    pub length: i64,
    pub symbols: i64,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
}

/// Settings generali utente.
///
/// Mappa la tabella `user_settings` del database.
#[derive(Debug, Clone, FromRow, SqlxTemplate)]
#[db("sqlite")]
#[table("user_settings")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct UserSettings {
    pub id: Option<i64>,
    pub user_id: i64,
}

/// Settings per la generazione password.
///
/// Mappa la tabella `passwords_generation_settings` del database.
///
/// Nota: usa i64 per length e symbols per coerenza con il resto del codebase.
#[derive(Debug, Clone, FromRow, SqliteTemplate)]
#[table("passwords_generation_settings")]
#[tp_upsert(by = "id")]
pub struct PasswordsGenSettings {
    pub id: Option<i64>,
    pub settings_id: i64,
    pub length: i64,
    pub symbols: i64,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub excluded_symbols: Option<String>,
}
