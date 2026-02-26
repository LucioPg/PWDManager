//! Configurazione per la generazione di password.

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub use aegis_password_generator::types::PasswordConfig as AegisPasswordConfig;
use sqlx::{sqlite::Sqlite, Type};

/// Set di simboli esclusi dalla generazione.
#[derive(Debug, Clone)]
pub struct ExcludedSymbolSet(HashSet<char>);

impl Type<Sqlite> for ExcludedSymbolSet {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, Sqlite> for ExcludedSymbolSet {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s: String = self.0.iter().collect();
        <String as sqlx::Encode<'q, Sqlite>>::encode(s, args)
    }
}

impl<'r> sqlx::Decode<'r, Sqlite> for ExcludedSymbolSet {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let excluded_symb_string = <String as sqlx::Decode<'r, Sqlite>>::decode(value)?;
        Ok(ExcludedSymbolSet::from(excluded_symb_string))
    }
}

impl From<String> for ExcludedSymbolSet {
    fn from(s: String) -> Self {
        Self(s.chars().filter(|c| !c.is_alphanumeric()).collect())
    }
}

impl std::ops::Deref for ExcludedSymbolSet {
    type Target = HashSet<char>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for ExcludedSymbolSet {
    fn default() -> Self {
        Self(HashSet::new())
    }
}

impl PartialEq for ExcludedSymbolSet {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Configurazione per la generazione di password.
#[derive(sqlx::FromRow, Debug, Clone, Default, sqlx_template::SqlxTemplate, PartialEq)]
#[table("passwords_generation_settings")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct PasswordGeneratorConfig {
    #[allow(unused)]
    pub id: Option<i64>,
    pub settings_id: i64,
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub excluded_symbols: ExcludedSymbolSet,
}

/// Preset per la generazione password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPreset {
    Medium,
    Strong,
    Epic,
    God,
}

impl PasswordPreset {
    /// Restituisce la configurazione per questo preset.
    pub fn to_config(&self, settings_id: i64) -> PasswordGeneratorConfig {
        match self {
            Self::Medium => PasswordGeneratorConfig {
                id: Some(settings_id),
                settings_id,
                length: 8,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
                excluded_symbols: ExcludedSymbolSet::default(),
            },
            Self::Strong => PasswordGeneratorConfig {
                id: Some(settings_id),
                settings_id,
                length: 12,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
                excluded_symbols: ExcludedSymbolSet::default(),
            },
            Self::Epic => PasswordGeneratorConfig {
                id: Some(settings_id),
                settings_id,
                length: 17,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
                excluded_symbols: ExcludedSymbolSet::default(),
            },
            Self::God => PasswordGeneratorConfig {
                id: Some(settings_id),
                settings_id,
                length: 26,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
                excluded_symbols: ExcludedSymbolSet::default(),
            },
        }
    }
}

impl Display for PasswordPreset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<PasswordGeneratorConfig> for AegisPasswordConfig {
    fn from(config: PasswordGeneratorConfig) -> Self {
        AegisPasswordConfig::default()
            .with_length(config.length as usize)
            .with_symbols(config.symbols > 0)
            .with_numbers(config.numbers)
            .with_uppercase(config.uppercase)
            .with_lowercase(config.lowercase)
    }
}
