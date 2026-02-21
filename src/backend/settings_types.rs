//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::FromRow;
use sqlx_template::{SqliteTemplate, SqlxTemplate};

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
