//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::{FromRow, Sqlite, Type};
use sqlx_template::SqlxTemplate;
use std::fmt;
use std::ops::Deref;

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
    pub auto_update: AutoUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Default)]
#[sqlx(transparent)]
pub struct AutoUpdate(pub bool);

impl Deref for AutoUpdate {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<bool> for AutoUpdate {
    fn from(value: bool) -> Self {
        AutoUpdate(value)
    }
}

impl From<AutoUpdate> for bool {
    fn from(value: AutoUpdate) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
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

/// Supported languages for Diceware passphrase generation.
/// Stored as TEXT in SQLite: 'EN', 'IT', 'FR'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "UPPERCASE")]
pub enum DicewareLanguage {
    #[default]
    EN,
    FR,
    IT,
}

impl From<DicewareLanguage> for diceware::EmbeddedList {
    fn from(lang: DicewareLanguage) -> Self {
        match lang {
            DicewareLanguage::EN => diceware::EmbeddedList::EN,
            DicewareLanguage::FR => diceware::EmbeddedList::FR,
            DicewareLanguage::IT => diceware::EmbeddedList::IT,
        }
    }
}

impl From<diceware::EmbeddedList> for DicewareLanguage {
    fn from(list: diceware::EmbeddedList) -> Self {
        match list {
            diceware::EmbeddedList::EN => DicewareLanguage::EN,
            diceware::EmbeddedList::FR => DicewareLanguage::FR,
            diceware::EmbeddedList::IT => DicewareLanguage::IT,
        }
    }
}

#[allow(clippy::needless_question_mark)]
#[derive(sqlx::FromRow, Debug, Clone, Default, sqlx_template::SqlxTemplate, PartialEq)]
#[db("sqlite")]
#[table("diceware_generation_settings")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct DicewareGenerationSettings {
    pub id: Option<i64>,
    pub settings_id: i64,
    pub word_count: i32,
    pub special_chars: i32,
    pub force_special_chars: bool,
    pub numbers: i32,
    pub language: DicewareLanguage,
}
