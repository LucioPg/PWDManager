//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::{FromRow, Sqlite, Type};
use sqlx_template::SqlxTemplate;
use std::fmt;

/// Settings generali utente.
///
/// Mappa la tabella `user_settings` del database.
#[derive(Debug, Clone, FromRow, SqlxTemplate, PartialEq, Default)]
#[db("sqlite")]
#[table("user_settings")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct UserSettings {
    pub id: Option<i64>,
    pub user_id: i64,
    pub theme: Theme,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Light
    }
}

// Serve per l'encode
impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Type<Sqlite> for Theme {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for Theme {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.to_string(), args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Theme {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <String as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
        Ok(Theme::from(s))
    }
}

impl From<String> for Theme {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }
}
