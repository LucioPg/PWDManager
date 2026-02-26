# pwd-types Library Extraction Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Estrarre il crate `pwd-types` contenente tipi puri per la gestione delle password dal progetto PWDManager.

**Architecture:** Creazione crate indipendente con feature flags per dipendenze opzionali (sqlx, generator, dioxus). Il progetto padre userà il crate via path dependency.

**Tech Stack:** Rust, secrecy, sqlx, sqlx-template, aegis-password-generator

**Reference Documents:**
- `docs/plans/2026-02-26-library-extraction-orchestrator.md` (stato e prerequisiti)
- `docs/library-extraction-analysis.md` (dettagli tecnici living)

---

## Task 1: Creare Struttura Crate

**Files:**
- Create: `pwd-types/Cargo.toml`
- Create: `pwd-types/src/lib.rs`

**Step 1.1: Creare directory e Cargo.toml**

Eseguire:
```bash
mkdir -p pwd-types/src
```

Creare `pwd-types/Cargo.toml`:
```toml
[package]
name = "pwd-types"
version = "0.1.0"
edition = "2024"

[features]
default = ["secrecy"]

# Core features
secrecy = ["dep:secrecy"]

# Database support - abilita tipi per SQLx
sqlx = ["dep:sqlx", "dep:sqlx-template", "secrecy"]

# Password generation support (richiede sqlx per SqlxTemplate)
generator = ["dep:aegis-password-generator", "sqlx"]

# Dioxus form support
dioxus = ["dep:dioxus", "secrecy"]

[dependencies]
# Core
secrecy = { version = "0.10", optional = true }

# SQLx support (optional)
sqlx = { version = "0.8", features = ["sqlite", "macros"], optional = true }
sqlx-template = { version = "0.2", optional = true }

# Password generation (optional)
aegis-password-generator = { version = "0.1", optional = true }

# Dioxus support (optional)
dioxus = { version = "0.7", optional = true }
```

**Step 1.2: Creare lib.rs skeleton**

Creare `pwd-types/src/lib.rs`:
```rust
//! Tipi puri per la gestione delle password.
//!
//! Questo crate fornisce tipi condivisi per:
//! - Valutazione forza password (score, strength)
//! - Statistiche password
//! - Wrapper secrecy per SQLx
//! - Configurazione generazione password

// Core types (sempre disponibili)
mod score;
pub use score::{PasswordScore, PasswordStrength, PasswordEvaluation};

mod stats;
pub use stats::PasswordStats;

// Optional: secrecy support
#[cfg(feature = "secrecy")]
pub use secrecy::{SecretBox, SecretString};

// Optional: SQLx database types
#[cfg(feature = "sqlx")]
mod secrets;
#[cfg(feature = "sqlx")]
pub use secrets::{DbSecretString, DbSecretVec, SecretSliceU8};

#[cfg(feature = "sqlx")]
mod stored;
#[cfg(feature = "sqlx")]
pub use stored::{UserAuth, StoredPassword, StoredRawPassword};

// Optional: password generator config (richiede sqlx per SqlxTemplate)
#[cfg(all(feature = "generator", feature = "sqlx"))]
mod generator;
#[cfg(all(feature = "generator", feature = "sqlx"))]
pub use generator::{PasswordGeneratorConfig, PasswordPreset, ExcludedSymbolSet};

**Step 1.3: Verificare struttura**

```bash
ls -la pwd-types/
ls -la pwd-types/src/
```

Expected: Directory e file creati

---

## Task 2: Estrarre Core Types (score.rs, stats.rs)

**Files:**
- Create: `pwd-types/src/score.rs`
- Create: `pwd-types/src/stats.rs`
- Source: `src/backend/password_types_helper.rs:349-433`, `374-383`

**Step 2.1: Creare score.rs**

Creare `pwd-types/src/score.rs`:
```rust
//! Tipi per la valutazione della forza delle password.

use std::fmt;
use std::fmt::Display;

/// Rappresenta un punteggio password da 0 a 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type)]
#[sqlx(transparent)]
pub struct PasswordScore(u8);

impl PasswordScore {
    pub const MAX: u8 = 100;

    fn clamp(value: i64) -> u8 {
        let positive = value.max(0);
        positive.min(Self::MAX as i64) as u8
    }

    pub fn new<T: Into<i64>>(value: T) -> Self {
        let v = value.into();
        Self(PasswordScore::clamp(v))
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn get_strength(score: Option<i64>) -> PasswordStrength {
        match score {
            Some(s) if s > 95 => PasswordStrength::GOD,
            Some(s) if s >= 85 => PasswordStrength::EPIC,
            Some(s) if s >= 70 => PasswordStrength::STRONG,
            Some(s) if s >= 50 => PasswordStrength::MEDIUM,
            Some(_) => PasswordStrength::WEAK,
            None => PasswordStrength::NotEvaluated,
        }
    }
}

impl PartialEq<u8> for PasswordScore {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for PasswordScore {
    fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for PasswordScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Enum che rappresenta la forza della password.
///
/// Viene salvata nel database come testo ('not evaluated','weak', ecc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum PasswordStrength {
    NotEvaluated,
    WEAK,
    MEDIUM,
    STRONG,
    EPIC,
    GOD,
}

/// Risultato della valutazione di una password.
#[derive(Debug, Clone, PartialEq)]
pub struct PasswordEvaluation {
    pub score: Option<PasswordScore>,
    pub reasons: Vec<String>,
}

impl From<PasswordScore> for PasswordEvaluation {
    fn from(score: PasswordScore) -> Self {
        PasswordEvaluation {
            score: Some(score),
            reasons: vec![],
        }
    }
}

impl PasswordEvaluation {
    pub fn strength(&self) -> PasswordStrength {
        match self.score {
            Some(s) => {
                let value = s.value() as i64;
                PasswordScore::get_strength(Some(value))
            }
            None => PasswordStrength::NotEvaluated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_score_clamp() {
        assert_eq!(PasswordScore::MAX, PasswordScore::new(100).value());
        assert_eq!(PasswordScore::MAX, PasswordScore::new(101).value());
        assert_eq!(0, PasswordScore::new(-100).value());
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(PasswordStrength::GOD, PasswordScore::get_strength(Some(96)));
        assert_eq!(PasswordStrength::EPIC, PasswordScore::get_strength(Some(85)));
        assert_eq!(PasswordStrength::STRONG, PasswordScore::get_strength(Some(70)));
        assert_eq!(PasswordStrength::MEDIUM, PasswordScore::get_strength(Some(50)));
        assert_eq!(PasswordStrength::WEAK, PasswordScore::get_strength(Some(10)));
        assert_eq!(PasswordStrength::NotEvaluated, PasswordScore::get_strength(None));
    }
}
```

**Step 2.2: Creare stats.rs**

Creare `pwd-types/src/stats.rs`:
```rust
//! Statistiche sulle password salvate.

/// Enum per tenere traccia delle statistiche delle password (usato nel frontend).
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PasswordStats {
    pub weak: usize,
    pub medium: usize,
    pub strong: usize,
    pub epic: usize,
    pub god: usize,
    pub total: usize,
    pub not_evaluated: usize,
}
```

**Step 2.3: Verificare compilazione parziale**

```bash
cd pwd-types && cargo check --features secrecy
```

Expected: Compilazione OK (con warnings per unused imports)

---

## Task 3: Estrarre SQLx Types (secrets.rs, stored.rs)

**Files:**
- Create: `pwd-types/src/secrets.rs`
- Create: `pwd-types/src/stored.rs`
- Source: `src/backend/password_types_helper.rs:13-150`, `152-321`

**Step 3.1: Creare secrets.rs**

Creare `pwd-types/src/secrets.rs`:
```rust
//! Wrapper secrecy per SQLx/SQLite.
//!
//! Questi wrapper rendono `SecretString` e `SecretBox<[u8]>` compatibili con SQLx.

use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::{sqlite::Sqlite, Type};

/// Type alias per `SecretBox<[u8]>`.
pub type SecretSliceU8 = SecretBox<[u8]>;

/// Wrapper per [`SecretString`] compatibile con SQLx/SQLite.
#[derive(Debug, Clone)]
pub struct DbSecretString(pub SecretString);

impl Type<Sqlite> for DbSecretString {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DbSecretString {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let val = self.0.expose_secret().to_string();
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretString {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <String as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
        Ok(DbSecretString(SecretString::from(s)))
    }
}

impl From<SecretString> for DbSecretString {
    fn from(secret: SecretString) -> Self {
        Self(secret)
    }
}

impl std::ops::Deref for DbSecretString {
    type Target = SecretString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wrapper per [`SecretBox<[u8]>`] compatibile con SQLx/SQLite.
#[derive(Debug, Clone)]
pub struct DbSecretVec(pub SecretSliceU8);

impl Type<Sqlite> for DbSecretVec {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DbSecretVec {
    fn encode_by_ref(
        &self,
        args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let slice = self.0.expose_secret();
        let val: Vec<u8> = slice.to_vec();
        <Vec<u8> as sqlx::Encode<'q, sqlx::Sqlite>>::encode(val, args)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DbSecretVec {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let vec = <Vec<u8> as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
        Ok(DbSecretVec(SecretBox::from(vec)))
    }
}

impl From<Vec<u8>> for DbSecretVec {
    fn from(vec: Vec<u8>) -> Self {
        Self(SecretBox::from(vec))
    }
}

impl From<SecretSliceU8> for DbSecretVec {
    fn from(secret: SecretSliceU8) -> Self {
        Self(secret)
    }
}

impl std::ops::Deref for DbSecretVec {
    type Target = SecretSliceU8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

**Step 3.2: Creare stored.rs**

Creare `pwd-types/src/stored.rs`:
```rust
//! Tipi database per password salvate.

use crate::{PasswordScore, SecretBox, SecretString};
use secrecy::ExposeSecret;
use sqlx::FromRow;
use sqlx_template::SqlxTemplate;

#[cfg(feature = "sqlx")]
use crate::{DbSecretString, DbSecretVec};

/// Struct per l'autenticazione utente.
#[derive(FromRow, Debug)]
pub struct UserAuth {
    pub id: i64,
    #[cfg(feature = "sqlx")]
    pub password: DbSecretString,
}

/// Struct per una password salvata nel database.
#[derive(FromRow, Debug, Clone, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct StoredPassword {
    pub id: Option<i64>,
    pub user_id: i64,
    pub location: DbSecretVec,
    pub location_nonce: Vec<u8>,
    pub password: DbSecretVec,
    pub password_nonce: Vec<u8>,
    pub notes: Option<DbSecretVec>,
    pub notes_nonce: Option<Vec<u8>>,
    pub score: PasswordScore,
    pub created_at: Option<String>,
}

impl StoredPassword {
    /// Crea una nuova struct [`StoredPassword`].
    pub fn new(
        id: Option<i64>,
        user_id: i64,
        location: SecretBox<[u8]>,
        location_nonce: Vec<u8>,
        password: SecretBox<[u8]>,
        notes: Option<SecretBox<[u8]>>,
        notes_nonce: Option<Vec<u8>>,
        score: PasswordScore,
        created_at: Option<String>,
        password_nonce: Vec<u8>,
    ) -> Self {
        let location: DbSecretVec = location.into();
        let password: DbSecretVec = password.into();
        let notes: Option<DbSecretVec> = notes.map(|n| n.into());

        StoredPassword {
            id,
            user_id,
            location,
            location_nonce,
            password,
            password_nonce,
            notes,
            notes_nonce,
            score,
            created_at,
        }
    }
}

/// Password non criptata per uso interno.
#[derive(Clone)]
pub struct StoredRawPassword {
    pub id: Option<i64>,
    #[allow(unused)]
    pub user_id: i64,
    pub location: SecretString,
    pub password: SecretString,
    pub notes: Option<SecretString>,
    pub score: Option<PasswordScore>,
    pub created_at: Option<String>,
}

impl std::fmt::Debug for StoredRawPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredRawPassword")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("location", &"***SECRET***")
            .field("password", &"***SECRET***")
            .field("notes", &self.notes.as_ref().map(|_| "***SECRET***"))
            .field("score", &self.score)
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl StoredRawPassword {
    pub fn new() -> Self {
        StoredRawPassword {
            id: None,
            user_id: 0,
            location: SecretString::new("".into()),
            password: "".to_string().into(),
            notes: None,
            score: None,
            created_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_form_fields(
        &self,
    ) -> (
        i64,
        SecretString,
        SecretString,
        Option<SecretString>,
        Option<PasswordScore>,
    ) {
        (
            self.id.unwrap(),
            self.location.clone(),
            self.password.clone(),
            self.notes.clone(),
            self.score.clone(),
        )
    }
}

impl PartialEq for StoredRawPassword {
    fn eq(&self, other: &Self) -> bool {
        match (&self.id, &other.id) {
            (Some(id1), Some(id2)) => {
                id1 == id2
                    && self.location.expose_secret() == other.location.expose_secret()
            }
            (None, None) => true,
            _ => false,
        }
    }
}
```

**Step 3.3: Verificare compilazione con feature sqlx**

```bash
cd pwd-types && cargo check --features "secrecy,sqlx"
```

Expected: Compilazione OK

---

## Task 4: Estrarre Generator Types (generator.rs)

**Files:**
- Create: `pwd-types/src/generator.rs`
- Source: `src/backend/password_types_helper.rs:435-593`

**Step 4.1: Creare generator.rs**

Creare `pwd-types/src/generator.rs`:
```rust
//! Configurazione per la generazione di password.

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub use aegis_password_generator::types::PasswordConfig as AegisPasswordConfig;
use sqlx::{sqlite::Sqlite, Type};

use crate::PasswordScore;

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
#[derive(sqlx::FromRow, Debug, Clone, Default, SqlxTemplate, PartialEq)]
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
```

**Step 4.2: Verificare compilazione con feature generator**

```bash
cd pwd-types && cargo check --features "secrecy,sqlx,generator"
```

Expected: Compilazione OK

---

## Task 5: Aggiornare Workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml` (root, linee 36-37)

**Step 5.1: Aggiungere pwd-types ai workspace members**

Modificare `Cargo.toml` alla sezione `[workspace]`:

```toml
[workspace]
members = ["gui_launcher", ".", "custom_errors", "pwd-types"]
```

**Step 5.2: Aggiungere dipendenza pwd-types al progetto principale**

Aggiungere in `[dependencies]`:

```toml
pwd-types = { path = "pwd-types", features = ["sqlx", "generator"] }
```

**Step 5.3: Verificare workspace**

```bash
cargo check --workspace
```

Expected: Compilazione OK (pwd-types compilato, progetto principale fallisce per use paths non aggiornati - questo è OK)

---

## Task 6: Aggiornare Use Paths nel Progetto Padre

**Files da modificare (da grep):**
1. `src/backend/db_backend.rs:3`
2. `src/backend/password_utils.rs:13`
3. `src/backend/password_utils_tests.rs:5`
4. `src/backend/strength_utils.rs:1,279`
5. `src/backend/db_settings_tests.rs:21`
6. `src/components/features/upsert_user.rs:14`
7. `src/components/features/dashboard.rs:1`
8. `src/components/globals/dialogs/stored_password_upsert.rs:2`
9. `src/components/globals/password_handler/component.rs:3`
10. `src/components/globals/password_handler/strength_analyzer.rs:3`
11. `src/components/globals/stat_card.rs:1`
12. `src/components/globals/table/table.rs:1`
13. `src/components/globals/table/table_row.rs:1`

**Step 6.1: Aggiornare db_backend.rs**

Sostituire:
```rust
use crate::backend::password_types_helper::{
```
con:
```rust
use pwd_types::{
```

**Step 6.2: Aggiornare password_utils.rs**

Sostituire:
```rust
use crate::backend::password_types_helper::{
```
con:
```rust
use pwd_types::{
```

**Step 6.3: Aggiornare password_utils_tests.rs**

Sostituire:
```rust
use crate::backend::password_types_helper::{
```
con:
```rust
use pwd_types::{
```

**Step 6.4: Aggiornare strength_utils.rs**

Sostituire:
```rust
use crate::backend::password_types_helper::{PasswordEvaluation, PasswordScore};
```
con:
```rust
use pwd_types::{PasswordEvaluation, PasswordScore};
```

E nel modulo test (linea 279), sostituire:
```rust
use crate::backend::password_types_helper::PasswordStrength;
```
con:
```rust
use pwd_types::PasswordStrength;
```

**Step 6.5: Aggiornare db_settings_tests.rs**

Sostituire:
```rust
use crate::backend::password_types_helper::PasswordPreset;
```
con:
```rust
use pwd_types::PasswordPreset;
```

**Step 6.6: Aggiornare components (upsert_user.rs, dashboard.rs, ...)**

Per ognuno dei file in `src/components/`, sostituire:
```rust
use crate::backend::password_types_helper::...
```
con:
```rust
use pwd_types::...
```

**Step 6.7: Verificare compilazione**

```bash
cargo check --workspace
```

Expected: Compilazione OK con warnings

---

## Task 7: Aggiornare mod.rs e Rimuovere Vecchio Codice

**Files:**
- Modify: `src/backend/mod.rs`
- Modify: `src/backend/password_types_helper.rs` (da trasformare in re-export)

**Step 7.1: Aggiornare password_types_helper.rs come re-export**

Trasformare `src/backend/password_types_helper.rs` in un semplice re-export per backward compatibility:

```rust
//! Re-export dei tipi da pwd-types per backward compatibility.
//!
//! Questo modulo delega tutti i tipi al crate `pwd-types`.

pub use pwd_types::*;
```

**Step 7.2: Verificare che i test passino**

```bash
cargo test --workspace
```

Expected: Tutti i test passano

---

## Task 8: Commit Finale

**Step 8.1: Verificare stato**

```bash
git status
```

**Step 8.2: Aggiungere file e commit**

```bash
git add pwd-types/
git add Cargo.toml
git add src/backend/password_types_helper.rs
git add src/backend/db_backend.rs
git add src/backend/password_utils.rs
git add src/backend/password_utils_tests.rs
git add src/backend/strength_utils.rs
git add src/backend/db_settings_tests.rs
git add src/components/features/upsert_user.rs
git add src/components/features/dashboard.rs
git add src/components/globals/dialogs/stored_password_upsert.rs
git add src/components/globals/password_handler/component.rs
git add src/components/globals/password_handler/strength_analyzer.rs
git add src/components/globals/stat_card.rs
git add src/components/globals/table/table.rs
git add src/components/globals/table/table_row.rs

git commit -m "$(cat <<'EOF'
feat: extract pwd-types library

Estratto crate pwd-types contenente tipi puri per la gestione password:
- PasswordScore, PasswordStrength, PasswordEvaluation (core)
- PasswordStats (statistiche)
- DbSecretString, DbSecretVec (SQLx wrappers)
- UserAuth, StoredPassword, StoredRawPassword (database types)
- PasswordGeneratorConfig, PasswordPreset (generator config)

Feature flags: secrecy (default), sqlx, generator, dioxus

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

**Step 8.3: Verificare commit**

```bash
git log -1 --oneline
```

Expected: Commit creato

---

## Task 9: Aggiornare Documenti Living

**Step 9.1: Aggiornare orchestratore**

In `docs/plans/2026-02-26-library-extraction-orchestrator.md`:
- Cambiare stato Step 1 da ⏳ a ✅
- Aggiungere data completamento
- Compilare "Lezioni Apprese" per Step 1

**Step 9.2: Aggiornare reference document**

In `docs/library-extraction-analysis.md`:
- Aggiornare mappa dipendenze
- Documentare eventuali problemi riscontrati
- Aggiungere note per Step 2

---

## Riepilogo Feature Flags

| Feature     | Moduli Abilitati                    | Dipendenze                    |
|-------------|-------------------------------------|-------------------------------|
| `secrecy`   | SecretString, SecretBox re-export   | secrecy                       |
| `sqlx`      | secrets, stored                     | sqlx, sqlx-template, secrecy  |
| `generator` | generator                           | aegis-password-generator      |
| `dioxus`    | (placeholder per FormSecret)        | dioxus                        |

---

## Troubleshooting

### Problema: "cannot find crate pwd_types"
Verificare che `pwd-types` sia in `workspace.members` nel `Cargo.toml` root.

### Problema: "feature `sqlx` is required"
Assicurarsi che il progetto padre abbia le features corrette in `Cargo.toml`:
```toml
pwd-types = { path = "pwd-types", features = ["sqlx", "generator"] }
```

### Problema: "unused import" warnings
I warnings per unused imports/variables sono non bloccanti e possono essere puliti successivamente.

---

**Fine Piano - Pronto per esecuzione in sessione separata**
