// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#![allow(clippy::needless_question_mark, clippy::len_without_is_empty)]
//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::{FromRow, Sqlite, Type};
use sqlx_template::SqlxTemplate;
use std::fmt;
use std::ops::Deref;
use std::time::Duration;

/// Settings generali utente.
///
/// Mappa la tabella `user_settings` del database.
#[allow(clippy::needless_question_mark, clippy::len_without_is_empty)]
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
    pub auto_logout_settings: Option<AutoLogoutSettings>,
    pub active_vault_id: Option<i64>,
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
pub enum AutoLogoutSettings {
    #[default]
    TenMinutes,
    OneHour,
    FiveHours,
}

// Serve per l'encode
impl fmt::Display for AutoLogoutSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Type<Sqlite> for AutoLogoutSettings {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for AutoLogoutSettings {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.to_string(), args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for AutoLogoutSettings {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <String as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
        Ok(AutoLogoutSettings::from(s))
    }
}

impl From<String> for AutoLogoutSettings {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "onehour" => AutoLogoutSettings::OneHour,
            "fivehours" => AutoLogoutSettings::FiveHours,
            _ => AutoLogoutSettings::TenMinutes,
        }
    }
}

impl AutoLogoutSettings {
    pub fn duration(&self) -> Duration {
        match self {
            AutoLogoutSettings::TenMinutes => Duration::from_secs(10 * 60),
            AutoLogoutSettings::OneHour => Duration::from_secs(60 * 60),
            AutoLogoutSettings::FiveHours => Duration::from_secs(5 * 60 * 60),
        }
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

#[allow(clippy::needless_question_mark, clippy::len_without_is_empty)]
#[derive(sqlx::FromRow, Debug, Clone, Default, sqlx_template::SqlxTemplate, PartialEq)]
#[db("sqlite")]
#[table("diceware_generation_settings")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct DicewareGenerationSettings {
    pub id: Option<i64>,
    pub settings_id: i64,
    pub word_count: i32,
    pub add_special_char: bool,
    pub numbers: i32,
    pub language: DicewareLanguage,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AutoLogoutSettings ---

    #[test]
    fn test_auto_logout_settings_from_string_ten_minutes() {
        assert_eq!(
            AutoLogoutSettings::from("TenMinutes".to_string()),
            AutoLogoutSettings::TenMinutes
        );
        assert_eq!(
            AutoLogoutSettings::from("tenminutes".to_string()),
            AutoLogoutSettings::TenMinutes
        );
        assert_eq!(
            AutoLogoutSettings::from("TENMINUTES".to_string()),
            AutoLogoutSettings::TenMinutes
        );
        assert_eq!(
            AutoLogoutSettings::from("unknown".to_string()),
            AutoLogoutSettings::TenMinutes
        );
        assert_eq!(
            AutoLogoutSettings::from("".to_string()),
            AutoLogoutSettings::TenMinutes
        );
    }

    #[test]
    fn test_auto_logout_settings_from_string_one_hour() {
        assert_eq!(
            AutoLogoutSettings::from("OneHour".to_string()),
            AutoLogoutSettings::OneHour
        );
        assert_eq!(
            AutoLogoutSettings::from("onehour".to_string()),
            AutoLogoutSettings::OneHour
        );
    }

    #[test]
    fn test_auto_logout_settings_from_string_five_hours() {
        assert_eq!(
            AutoLogoutSettings::from("FiveHours".to_string()),
            AutoLogoutSettings::FiveHours
        );
        assert_eq!(
            AutoLogoutSettings::from("fivehours".to_string()),
            AutoLogoutSettings::FiveHours
        );
    }

    #[test]
    fn test_auto_logout_settings_durations() {
        assert_eq!(
            AutoLogoutSettings::TenMinutes.duration(),
            Duration::from_secs(600)
        );
        assert_eq!(
            AutoLogoutSettings::OneHour.duration(),
            Duration::from_secs(3600)
        );
        assert_eq!(
            AutoLogoutSettings::FiveHours.duration(),
            Duration::from_secs(18000)
        );
    }

    // --- Theme ---

    #[test]
    fn test_theme_from_string_light() {
        assert_eq!(Theme::from("Light".to_string()), Theme::Light);
        assert_eq!(Theme::from("light".to_string()), Theme::Light);
        assert_eq!(Theme::from("unknown".to_string()), Theme::Light);
        assert_eq!(Theme::from("".to_string()), Theme::Light);
    }

    #[test]
    fn test_theme_from_string_dark() {
        assert_eq!(Theme::from("Dark".to_string()), Theme::Dark);
        assert_eq!(Theme::from("dark".to_string()), Theme::Dark);
    }

    #[test]
    fn test_theme_defaults() {
        assert_eq!(Theme::default(), Theme::Light);
        assert_eq!(AutoLogoutSettings::default(), AutoLogoutSettings::TenMinutes);
    }

    // --- DicewareLanguage ---

    #[test]
    fn test_diceware_language_roundtrip() {
        for lang in [DicewareLanguage::EN, DicewareLanguage::FR, DicewareLanguage::IT] {
            let embedded: diceware::EmbeddedList = lang.into();
            let back: DicewareLanguage = embedded.into();
            assert_eq!(lang, back);
        }
    }

    #[test]
    fn test_diceware_language_default() {
        assert_eq!(DicewareLanguage::default(), DicewareLanguage::EN);
    }

    // --- AutoUpdate ---

    #[test]
    fn test_auto_update_from_bool() {
        assert!(AutoUpdate::from(true).0);
        assert!(!AutoUpdate::from(false).0);
    }

    #[test]
    fn test_auto_update_deref() {
        let au = AutoUpdate(true);
        assert!(*au);
        let au = AutoUpdate(false);
        assert!(!*au);
    }
}
