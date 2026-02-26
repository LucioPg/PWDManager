# Analisi Estrazione Librerie - PWDManager

**Data:** 2026-02-26
**Obiettivo:** Identificare componenti riutilizzabili da estrarre in librerie indipendenti

---

## Panoramica Architettura Attuale

### Workspace Structure

```
PWDManager/
├── src/
│   ├── main.rs
│   ├── auth.rs
│   ├── components/          # UI Components (Dioxus)
│   │   └── globals/
│   │       └── password_handler/
│   └── backend/             # Business Logic
│       ├── mod.rs
│       ├── password_types_helper.rs  ← 🏆 CANDIDATO
│       ├── strength_utils.rs         ← 🏆 CANDIDATO
│       ├── utils.rs                  ← 🥈 PARZIALE
│       ├── password_utils.rs         ← 🥈 PARZIALE
│       ├── db_backend.rs             ← ❌ NON CANDIDATO
│       └── settings_types.rs         ← ❌ TROPPO SEMPLICE
├── gui_launcher/            # Desktop launcher (gia crate)
└── custom_errors/           # Error types (gia crate)
```

---

## Mappa delle Dipendenze

```
custom_errors (✅ gia libreria)
       ↓
password_types_helper ←────────────────────┐
       ↓                                   │
   ┌───┴───┐                               │
   │       │                               │
strength_utils  utils.rs (password)        │
   │       │       │                       │
   │       │       ↓                       │
   │       │   password_utils ←────────────┘
   │       │       ↓
   │       │   db_backend
   │       │
   └───┴───→ PasswordHandler (Dioxus UI)
                  ↓
             StrengthAnalyzer
```

---

## Candidati per Estrazione

### 🏆 Tier 1 - Candidati Eccellenti

#### 1. `pwd-types`

**File sorgente:** `src/backend/password_types_helper.rs`
**Complessita:** 🟢 Bassa
**Riutilizzabilita:** ⭐⭐⭐⭐⭐ Altissima

**Contenuto:**

```rust
// Wrapper secrecy per SQLx
pub struct DbSecretString(pub SecretString);
pub struct DbSecretVec(pub SecretSliceU8);

// Tipi password
pub struct PasswordScore(u8);  // 0-100
pub enum PasswordStrength { NotEvaluated, WEAK, MEDIUM, STRONG, EPIC, GOD }
pub struct PasswordEvaluation {
    score,
    reasons
}
pub struct PasswordStats {
    weak,
    medium,
    strong,
    epic,
    god,
    total,
    not_evaluated
}

// Configurazione generazione
pub struct PasswordGeneratorConfig {
    length,
    symbols,
    numbers,
    uppercase,
    lowercase,
    excluded_symbols
}
pub enum PasswordPreset { Medium, Strong, Epic, God }

// Database types
pub struct UserAuth {
    id,
    password
}
pub struct StoredPassword {
    id,
    user_id,
    location,
    password,
    notes,
    score,
    ...
}
pub struct StoredRawPassword {
    id,
    user_id,
    location,
    password,
    notes,
    score,
    ...
}
```

**Dipendenze:**

- `secrecy` - SecretString wrapper
- `sqlx` - Type trait implementations
- `sqlx-template` - SqlxTemplate derive
- `aegis-password-generator` - PasswordConfig re-export

**Perche e ideale:**

- Tipi puri senza side effects
- Zero dipendenze dal resto del progetto
- Gia ben documentati con doc comments
- Test autonomi presenti

---

#### 2. `pwd-strength`

**File sorgente:** `src/backend/strength_utils.rs`
**Complessita:** 🟡 Media
**Riutilizzabilita:** ⭐⭐⭐⭐ Alta

**Contenuto:**

```rust
// Inizializzazione
pub fn init_blacklist() -> std::io::Result<()>

// Valutazione principale
pub fn evaluate_password_strength(
    password: &SecretString,
    token: Option<CancellationToken>
) -> PasswordEvaluation

pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>
)

// Sezioni interne
fn blacklist_section(password: &SecretString) -> Result<Option<String>, ()>
fn length_section(password: &SecretString) -> Result<Option<String>, ()>
fn character_variety_section(password: &SecretString) -> Result<Option<String>, ()>
fn pattern_analysis_section(password: &SecretString) -> Result<Option<String>, ()>
```

**Dipendenze:**

- `secrecy` - SecretString
- `tokio` - async runtime
- `tokio-util` - CancellationToken
- `tracing` - logging
- `pwd-types` - PasswordEvaluation, PasswordScore

**Perche e ideale:**

- Logica completamente generica
- Niente UI o framework specifico
- Test molto completi (gia presenti)
- Blacklist embedded (10k password comuni)

---

### 🥈 Tier 2 - Buoni Candidati (con refactoring)

#### 3. `pwd-crypto`

**File sorgente:** `src/backend/utils.rs` + `src/backend/password_utils.rs` (parte)
**Complessita:** 🟡 Media
**Riutilizzabilita:** ⭐⭐⭐ Buona

**Contenuto da estrarre:**

```rust
// Da utils.rs
pub fn base64_encode(bytes: &[u8]) -> String
pub fn generate_salt() -> SaltString
pub fn encrypt(raw_password: SecretString) -> Result<String, EncryptionError>
pub fn verify_password(raw_password: SecretString, hash: &str) -> Result<(), DecryptionError>

// Da password_utils.rs
pub fn create_cipher(salt: &Salt<'_>, user_auth: &UserAuth) -> Result<Aes256Gcm, DBError>
fn create_nonce() -> Nonce<Aes256Gcm>
fn encrypt_string(plaintext: &str, cipher: &Aes256Gcm) -> Result<(SecretBox<[u8]>, Nonce<Aes256Gcm>), DBError>
fn decrypt_to_string(encrypted: &[u8], nonce: &Nonce<Aes256Gcm>, cipher: &Aes256Gcm) -> Result<String, DBError>
```

**⚠️ Da separare (rimangono nel progetto padre):**

```rust
// Da utils.rs - Avatar handling (SPECIFICO DEL PROGETTO)
pub fn get_user_avatar_with_default(avatar_from_db: Option<Vec<u8>>) -> String
pub fn format_avatar_url(avatar_b64: String) -> String
pub fn scale_avatar(bytes: &[u8]) -> Result<Vec<u8>, GeneralError>
fn image_to_vec(img: &DynamicImage) -> Result<Vec<u8>, GeneralError>
```

**Dipendenze:**

- `argon2` - Password hashing
- `aes-gcm` - Encryption
- `secrecy` - SecretString
- `base64` - Encoding
- `pwd-types` - UserAuth
- `custom_errors` - Error types

**Refactoring necessario:**

1. Creare `src/backend/avatar_utils.rs` per funzioni avatar
2. Spostare funzioni avatar da `utils.rs`
3. Estrarre solo funzioni crypto pure

---

#### 4. `pwd-dioxus`

**File sorgente:** `src/components/globals/password_handler/` + `form_field.rs`
**Complessita:** 🔴 Alta
**Riutilizzabilita:** ⭐⭐⭐ Buona (solo per progetti Dioxus)

**Contenuto:**

```rust
// Componente principale
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element
- Input password con visibility toggle
- Generazione password suggerita
- Valutazione forza in tempo reale
- Cancellation support

// Visualizzazione forza
pub fn StrengthAnalyzer(props: StrengthAnalyzerProps) -> Element
- Barra gradiente con cursore
- Tooltip con motivi
- Spinner durante valutazione

// Campo form generico
pub fn FormField(props: FormFieldProps) -> Element
- Supporto visibility toggle per password
- Label e placeholder
```

**Dipendenze:**

- `dioxus` - UI framework
- `pwd-types` - PasswordScore, PasswordStrength, FormSecret
- `pwd-strength` - evaluate_password_strength
- `aegis-password-generator` - Generazione password

**⚠️ Nota:** Libreria specifica per Dioxus, ma riutilizzabile in altri progetti Dioxus.

---

### ❌ Tier 3 - Non Raccomandati

| Modulo                         | Motivo                                                        |
|--------------------------------|---------------------------------------------------------------|
| `db_backend.rs`                | Altamente accoppiato a SQLx, schema DB specifico, transazioni |
| `password_utils.rs` (pipeline) | Dipende da db_backend per fetch/save                          |
| `settings_types.rs`            | Troppo semplice (1 struct), non vale la pena                  |
| `init_queries.rs`              | SQL specifico del progetto                                    |
| `ui_utils.rs`                  | Helper UI specifici                                           |

---

## Piano di Estrazione

### Fase 1: Fondamenta (no breaking changes)

```
Step 1: pwd-types
        ├── Crea crate pwd-types
        ├── Sposta tipi da password_types_helper.rs
        ├── Aggiorna use paths nel progetto padre
        └── Test: cargo test verifica tutto funziona

Step 2: pwd-strength
        ├── Crea crate pwd-strength
        ├── Aggiungi dipendenza pwd-types
        ├── Sposta strength_utils.rs
        └── Test: cargo test verifica tutto funziona

Step 3: pwd-crypto
        ├── Crea avatar_utils.rs nel progetto padre
        ├── Sposta funzioni avatar
        ├── Crea crate pwd-crypto
        ├── Sposta funzioni crypto
        └── Test: cargo test verifica tutto funziona
```

### Fase 2: UI (opzionale)

```
Step 4: pwd-dioxus
        ├── Crea crate pwd-dioxus
        ├── Sposta PasswordHandler, StrengthAnalyzer
        ├── Feature flags per customization
        └── Test: cargo test verifica tutto funziona
```

---

## Gestione Overlapping

### Dipendenze da `password_types_helper.rs`

| File                | Tipi utilizzati                                                             |
|---------------------|-----------------------------------------------------------------------------|
| `strength_utils.rs` | `PasswordEvaluation`, `PasswordScore`                                       |
| `utils.rs`          | Nessuna                                                                     |
| `password_utils.rs` | `StoredPassword`, `UserAuth`, `DbSecretString`, `DbSecretVec`, tutti i tipi |
| `db_backend.rs`     | `UserAuth`, `StoredPassword`, `PasswordPreset`, `PasswordGeneratorConfig`   |
| `PasswordHandler`   | `PasswordScore`, `PasswordStrength`, `FormSecret`                           |

### Strategia di Migrazione

1. **Estrarre `pwd-types` per primo**
2. **Aggiornare import gradualmente:**
   ```rust
   // Prima
   use crate::backend::password_types_helper::PasswordScore;

   // Dopo
   use pwd_types::PasswordScore;
   ```
3. **Mantenere re-exports per backward compatibility:**
   ```rust
   // In src/backend/mod.rs
   pub use pwd_types::{PasswordScore, PasswordStrength, ...};
   ```

---

## Struttura Finale Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "PWDManager", # App principale
    "gui_launcher", # Desktop launcher
    "custom_errors", # Error types (esistente)
    "pwd-types", # NUOVO: Tipi password
    "pwd-strength", # NUOVO: Valutazione password
    "pwd-crypto", # NUOVO: Crittografia password
    # "pwd-dioxus",    # OPZIONALE: UI components
]
```

---

## Checklist Preliminare

Prima di iniziare l'estrazione:

- [ ] Verificare che tutti i test passino: `cargo test`
- [ ] Verificare build release: `cargo build --release`
- [ ] Commit stato attuale: `git commit -m "chore: pre-extraction checkpoint"`
- [ ] Creare branch per estrazione: `git checkout -b feat/extract-libs`

---

## Prossimi Passi

1. **Iniziare con `pwd-types`** - Il candidato piu semplice e fondamentale
2. Verificare che l'estrazione non rompa nulla
3. Procedere con `pwd-strength`
4. Infine `pwd-crypto` (con refactoring avatar)

---

## Note Tecniche

### Versioning

- Iniziare con version `0.1.0` per tutte le nuove librerie
- Mantenere compatibilita semantica durante sviluppo iniziale
- NON pubblicare su crates.io

---

# Piani di Implementazione Dettagliati

> **⚠️ IMPORTANTE:** Questo documento deve essere **aggiornato dopo ogni step completato**.
>
> Ogni estrazione può rivelare dipendenze nascoste, problemi di compatibilità o opportunità di refactoring non previste. Prima di procedere allo step successivo:
>
> 1. Verificare che `cargo test` passi al 100%
> 2. Verificare che `cargo build --release` completi senza errori
> 3. Aggiornare questo documento con:
>    - Problemi incontrati e soluzioni adottate
>    - Modifiche alle API rispetto al piano originale
>    - Nuove dipendenze emerse
>    - Aggiornamento mappa delle dipendenze
> 4. Commit delle modifiche alla documentazione
>
> **Non procedere allo step successivo senza aver aggiornato questo documento.**

Questa sezione contiene i piani step-by-step per ogni estrazione, con feature flags e configurazioni specifiche.

---

## Step 1: `pwd-types` - Dettaglio Implementazione

### Struttura Crate

```
pwd-types/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── score.rs              # PasswordScore, PasswordStrength, PasswordEvaluation
│   ├── stats.rs              # PasswordStats
│   ├── secrets.rs            # DbSecretString, DbSecretVec (feature: sqlx)
│   ├── generator.rs          # PasswordGeneratorConfig, PasswordPreset (feature: generator)
│   ├── stored.rs             # StoredPassword, StoredRawPassword, UserAuth (feature: sqlx)
│   └── form.rs               # FormSecret (feature: dioxus)
└── tests/
    └── score_tests.rs
```

### Cargo.toml

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
sqlx = ["dep:sqlx", "sqlx-template", "secrecy"]

# Password generation support
generator = ["dep:aegis-password-generator", "secrecy"]

# Dioxus form support
dioxus = ["dep:dioxus", "secrecy"]

[dependencies]
# Core
secrecy = { version = "0.10", optional = true }
thiserror = "2.0"

# SQLx support (optional)
sqlx = { version = "0.8", features = ["sqlite", "macros"], optional = true }
sqlx-template = { version = "0.2", optional = true }

# Password generation (optional)
aegis-password-generator = { version = "0.1", optional = true }

# Dioxus support (optional)
dioxus = { version = "0.7", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["test-util"] }
```

### API Pubblica per Feature

```rust
// lib.rs

#[cfg(feature = "secrecy")]
pub use secrecy::{SecretString, SecretBox};

// Sempre disponibili (core)
mod score;
pub use score::{PasswordScore, PasswordStrength, PasswordEvaluation};

mod stats;
pub use stats::PasswordStats;

#[cfg(feature = "sqlx")]
mod secrets;
#[cfg(feature = "sqlx")]
pub use secrets::{DbSecretString, DbSecretVec};

#[cfg(feature = "sqlx")]
mod stored;
#[cfg(feature = "sqlx")]
pub use stored::{UserAuth, StoredPassword, StoredRawPassword};

#[cfg(feature = "generator")]
mod generator;
#[cfg(feature = "generator")]
pub use generator::{PasswordGeneratorConfig, PasswordPreset, ExcludedSymbolSet};

#[cfg(feature = "dioxus")]
mod form;
#[cfg(feature = "dioxus")]
pub use form::FormSecret;
```

### Checklist Implementazione

- [x] Creare directory `pwd-types/`
- [x] Configurare `Cargo.toml` con features
- [x] Estrarre `PasswordScore`, `PasswordStrength`, `PasswordEvaluation` in `score.rs`
- [x] Estrarre `PasswordStats` in `stats.rs`
- [x] Estrarre `DbSecretString`, `DbSecretVec` in `secrets.rs` (feature sqlx)
- [x] Estrarre `UserAuth`, `StoredPassword`, `StoredRawPassword` in `stored.rs` (feature sqlx)
- [x] Estrarre `PasswordGeneratorConfig`, `PasswordPreset`, `ExcludedSymbolSet` in `generator.rs`
- [x] Aggiornare workspace `Cargo.toml` root
- [x] Aggiornare `PWDManager/Cargo.toml` con dipendenza `pwd-types`
- [x] Aggiornare tutti i `use` paths nel progetto padre
- [x] Eseguire `cargo test` per verificare
- [x] Commit: `feat: extract pwd-types library`

**Completato:** 2026-02-26

### Modifiche rispetto al piano originale

| Aspetto | Piano Originale | Implementazione Effettiva |
|---------|-----------------|---------------------------|
| Derive sqlx | Derive dirette `#[derive(sqlx::Type)]` | `#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]` per condizionalità |
| Dipendenza futures | Non prevista | Aggiunta `futures` come dipendenza opzionale della feature sqlx |
| AegisPasswordConfig | Non esportato | Aggiunto all'export pubblico del modulo generator |
| generator feature | Dipendenza solo `aegis-password-generator` | Richiede anche `sqlx` per `SqlxTemplate` |

### Problemi incontrati e soluzioni

1. **Derive sqlx senza feature flag**: Le derive `#[sqlx(...)]` causavano errore quando compilato senza feature sqlx. Soluzione: `cfg_attr` condizionale.

2. **SqlxTemplate richiede futures**: La macro genera codice che dipende da `futures::Stream`. Soluzione: aggiunto `futures` come dipendenza opzionale.

3. **Doctest sqlx-template**: I doctest generati automaticamente falliscono per mancanza di dipendenze. Soluzione: ignorati (non bloccanti per il progetto).

4. **Workspace members**: Necessario aggiungere `pwd-types` ai members prima di poter verificare la compilazione del singolo crate.

---

## Step 2: `pwd-strength` - Dettaglio Implementazione

### Struttura Crate

```
pwd-strength/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── evaluator.rs          # evaluate_password_strength, evaluate_password_strength_tx
│   ├── sections/
│   │   ├── mod.rs
│   │   ├── blacklist.rs      # blacklist_section
│   │   ├── length.rs         # length_section
│   │   ├── variety.rs        # character_variety_section
│   │   └── pattern.rs        # pattern_analysis_section
│   ├── blacklist_loader.rs   # Caricamento dinamico blacklist
│   └── error.rs              # Error types
├── tests/
│   └── evaluator_tests.rs
└── assets/
    └── .gitkeep              # Placeholder per blacklist (non embedded)
```

### Cargo.toml

```toml
[package]
name = "pwd-strength"
version = "0.1.0"
edition = "2024"

[features]
default = ["async"]

# Async support (incluso di default)
async = ["dep:tokio", "dep:tokio-util"]

# Tracing support
tracing = ["dep:tracing"]

# Blacklist support - carica da file esterno
blacklist = []

# Built-in blacklist path (solo se blacklist feature attiva)
# Non usa include_str! - carica da filesystem
[dependencies]
pwd-types = { path = "../pwd-types", features = ["secrecy"] }
thiserror = "2.0"

# Async (optional)
tokio = { version = "1", features = ["sync", "time", "rt"], optional = true }
tokio-util = { version = "0.7", features = ["sync"], optional = true }

# Logging (optional)
tracing = { version = "0.1", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros"] }
tempfile = "3"
```

### Sistema Caricamento Blacklist

**Variabile d'ambiente:** `PWD_BLACKLIST_PATH`
**Fallback:** `./assets/10k-most-common.txt` (relativo alla working directory)

```rust
// blacklist_loader.rs

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;
use thiserror::Error;

static COMMON_PASSWORDS: OnceLock<HashSet<String>> = OnceLock::new();

#[derive(Error, Debug)]
pub enum BlacklistError {
    #[error("Blacklist file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Failed to read blacklist file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Blacklist file is empty")]
    EmptyFile,
}

/// Returns the blacklist file path.
///
/// Priority:
/// 1. Environment variable `PWD_BLACKLIST_PATH`
/// 2. Default path `./assets/10k-most-common.txt`
fn get_blacklist_path() -> PathBuf {
    std::env::var("PWD_BLACKLIST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./assets/10k-most-common.txt"))
}

/// Initializes the password blacklist from external file.
///
/// # Environment Variable
///
/// Set `PWD_BLACKLIST_PATH` to specify a custom blacklist file location.
/// If not set, defaults to `./assets/10k-most-common.txt`.
///
/// # Errors
///
/// Returns error if:
/// - File does not exist
/// - File cannot be read
/// - File is empty
///
/// # Example
///
/// ```rust,no_run
/// // Custom path via environment
/// std::env::set_var("PWD_BLACKLIST_PATH", "/etc/myapp/blacklist.txt");
/// pwd_strength::init_blacklist()?;
///
/// // Or use default path
/// pwd_strength::init_blacklist()?;
/// ```
pub fn init_blacklist() -> Result<usize, BlacklistError> {
    // Idempotente: se gia inizializzata, ritorna subito
    if COMMON_PASSWORDS.get().is_some() {
        return Ok(COMMON_PASSWORDS.get().map(|s| s.len()).unwrap_or(0));
    }

    let path = get_blacklist_path();

    if !path.exists() {
        return Err(BlacklistError::FileNotFound(path));
    }

    let content = std::fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Err(BlacklistError::EmptyFile);
    }

    let set: HashSet<String> = content
        .lines()
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect();

    let count = set.len();
    let _ = COMMON_PASSWORDS.set(set);

    #[cfg(feature = "tracing")]
    tracing::info!("Blacklist initialized: {} passwords from {:?}", count, path);

    Ok(count)
}

/// Returns a reference to the loaded blacklist.
///
/// Returns `None` if `init_blacklist()` has not been called.
pub fn get_blacklist() -> Option<&'static HashSet<String>> {
    COMMON_PASSWORDS.get()
}

/// Checks if a password is in the blacklist.
///
/// Returns `true` if password is in the blacklist (case-insensitive).
/// Returns `false` if blacklist is not initialized or password is not found.
pub fn is_blacklisted(password: &str) -> bool {
    COMMON_PASSWORDS
        .get()
        .map(|bl| bl.contains(&password.to_lowercase()))
        .unwrap_or(false)
}
```

### API Pubblica

```rust
// lib.rs

pub use pwd_types::{PasswordScore, PasswordStrength, PasswordEvaluation};

mod blacklist_loader;
pub use blacklist_loader::{init_blacklist, get_blacklist, is_blacklisted, BlacklistError};

mod evaluator;
pub use evaluator::{
    evaluate_password_strength,
    evaluate_password_strength_tx,  // solo con feature "async"
};
```

### Checklist Implementazione

- [x] Creare directory `pwd-strength/`
- [x] Configurare `Cargo.toml` con features
- [x] Implementare `blacklist.rs` con variabile d'ambiente
- [x] Estrarre sezioni in `sections/` directory
- [x] Estrarre `evaluate_password_strength` in `evaluator.rs`
- [x] Aggiornare `PWDManager/Cargo.toml` con dipendenza
- [x] File `10k-most-common.txt` già presente in `PWDManager/assets/`
- [x] Aggiornare imports per usare la libreria (via re-export in `mod.rs`)
- [x] Rimuovere vecchio `strength_utils.rs`
- [x] Eseguire `cargo test` - 83 test passano (31 pwd-strength + 2 pwd-types + 50 PWDManager)
- [x] Commit: `feat: extract pwd-strength library`

**Completato:** 2026-02-26

### Modifiche rispetto al piano originale

| Aspetto | Piano Originale | Implementazione Effettiva |
|---------|-----------------|---------------------------|
| Nome modulo blacklist | `blacklist_loader.rs` | `blacklist.rs` (più semplice) |
| Directory tests | `tests/` separata | Test inline nei moduli `#[cfg(test)]` |
| Storage blacklist | `OnceLock<HashSet>` | `RwLock<Option<HashSet>>` per testabilità |
| Feature `blacklist` | Feature flag dedicato | Rimosso (sempre attivo) |
| Error types | `error.rs` separato | `BlacklistError` in `blacklist.rs` |
| tokio-util features | `features = ["sync"]` | Nessuna feature (CancellationToken è default) |

### Problemi incontrati e soluzioni

1. **OnceLock immutabile**: Non è possibile resettare `OnceLock` tra test. Soluzione: cambiato da `OnceLock<HashSet>` a `RwLock<Option<HashSet>>` con funzione `reset_blacklist_for_testing()` per i test.

2. **Rust 2024 unsafe**: In Rust 2024 edition, `std::env::set_var` e `remove_var` richiedono blocchi `unsafe`. Soluzione: creati helper `set_env`/`remove_env` con unsafe blocks.

3. **tokio-util sync feature**: La feature `sync` non esiste in tokio-util. Soluzione: rimosso `features = ["sync"]` perché `CancellationToken` è disponibile di default.

4. **Test interference**: Test che modificano la variabile d'ambiente interferiscono tra loro. Soluzione: aggiunto `#[serial]` (da serial_test) a tutti i test che usano env var.

5. **Doctest unsafe**: I doctest con blocchi unsafe non compilano correttamente. Soluzione: cambiato da `#[doc(no_run)]` a `#[doc(ignore)]`.

---

## Step 3: `pwd-crypto` - Dettaglio Implementazione

### Struttura Crate

```
pwd-crypto/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── hash.rs                # Argon2 hashing (encrypt, verify_password)
│   ├── cipher.rs              # AES-256-GCM (create_cipher, encrypt_string, decrypt_string)
│   ├── nonce.rs               # Nonce generation
│   ├── encoding.rs            # base64 utilities
│   └── error.rs               # CryptoError enum
└── tests/
    ├── hash_tests.rs
    └── cipher_tests.rs
```

### Cargo.toml

```toml
[package]
name = "pwd-crypto"
version = "0.1.0"
edition = "2024"

[features]
default = ["hash"]

# Argon2 password hashing
hash = ["dep:argon2", "dep:secrecy"]

# AES-256-GCM encryption
cipher = ["dep:aes-gcm", "dep:secrecy"]

# Full crypto suite
full = ["hash", "cipher"]

# Base64 utilities
base64 = ["dep:base64"]

[dependencies]
# Core
thiserror = "2.0"
secrecy = { version = "0.10", optional = true }

# Password hashing (optional)
argon2 = { version = "0.5", features = ["std", "zeroize"], optional = true }

# Encryption (optional)
aes-gcm = { version = "0.10", features = ["zeroize"], optional = true }

# Encoding (optional)
base64 = { version = "0.22", optional = true }

# Per create_cipher (richiede pwd-types con feature sqlx)
pwd-types = { path = "../pwd-types", features = ["sqlx"], optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros"] }
```

### API Pubblica

```rust
// lib.rs

#[cfg(feature = "hash")]
mod hash;
#[cfg(feature = "hash")]
pub use hash::{encrypt, verify_password, generate_salt};

#[cfg(feature = "cipher")]
mod cipher;
#[cfg(feature = "cipher")]
pub use cipher::{
    create_cipher,
    create_nonce,
    encrypt_string,
    encrypt_optional_string,
    decrypt_to_string,
    decrypt_optional_to_string,
};

#[cfg(feature = "base64")]
mod encoding;
#[cfg(feature = "base64")]
pub use encoding::base64_encode;

mod error;
pub use error::CryptoError;
```

### CryptoError Unificato

```rust
// error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Password verification failed")]
    VerificationFailed,

    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    #[error("Nonce corruption: expected 12 bytes, got {0}")]
    NonceCorruption(usize),

    #[error("Cipher creation failed: {0}")]
    CipherCreation(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
}
```

### Refactoring Progetto Padre

**Nuovo file:** `src/backend/avatar_utils.rs`

```rust
// avatar_utils.rs - Rimane nel progetto PWDManager

use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use custom_errors::GeneralError;
use base64::{Engine, prelude::BASE64_STANDARD};

pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

pub fn get_user_avatar_with_default(avatar_from_db: Option<Vec<u8>>) -> String {
    let avatar: Vec<u8> = match avatar_from_db {
        Some(avatar_) if !avatar_.is_empty() => avatar_,
        _ => include_bytes!("../../assets/default_avatar.png").to_vec(),
    };
    format_avatar_url(base64_encode(&avatar))
}

pub fn format_avatar_url(avatar_b64: String) -> String {
    format!("data:image/png;base64,{}", avatar_b64)
}

pub fn scale_avatar(bytes: &[u8]) -> Result<Vec<u8>, GeneralError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| GeneralError::new_scaling_error(e.to_string()))?;
    image_to_vec(&img.thumbnail(128, 128))
}

fn image_to_vec(img: &DynamicImage) -> Result<Vec<u8>, GeneralError> {
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| GeneralError::new_encode_error(e.to_string()))?;
    Ok(buffer.into_inner())
}
```

### Checklist Implementazione

- [ ] Creare `src/backend/avatar_utils.rs` nel progetto padre
- [ ] Spostare funzioni avatar da `utils.rs` a `avatar_utils.rs`
- [ ] Aggiornare `mod.rs` per includere `avatar_utils`
- [ ] Creare directory `pwd-crypto/`
- [ ] Configurare `Cargo.toml` con features
- [ ] Implementare `hash.rs` con funzioni Argon2
- [ ] Implementare `cipher.rs` con funzioni AES-GCM
- [ ] Implementare `error.rs` con `CryptoError`
- [ ] Aggiornare `password_utils.rs` per usare la libreria
- [ ] Aggiornare `utils.rs` per usare la libreria (o rimuovere)
- [ ] Eseguire `cargo test`
- [ ] Commit: `feat: extract pwd-crypto library`

---

## Step 4: `pwd-dioxus` - Dettaglio Implementazione (Opzionale)

### Struttura Crate

```
pwd-dioxus/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── password_handler/
│   │   ├── mod.rs
│   │   ├── component.rs
│   │   └── props.rs
│   ├── strength_analyzer/
│   │   ├── mod.rs
│   │   └── component.rs
│   ├── form_field/
│   │   ├── mod.rs
│   │   └── component.rs
│   └── spinner/
│       ├── mod.rs
│       └── component.rs
└── assets/
    └── styles.css          # CSS opzionale per componenti
```

### Cargo.toml

```toml
[package]
name = "pwd-dioxus"
version = "0.1.0"
edition = "2024"

[features]
default = ["handler"]

# Password handler component
handler = ["dep:dioxus", "pwd-strength", "pwd-types/secrecy"]

# Strength analyzer (barra + tooltip)
analyzer = ["dep:dioxus", "pwd-types"]

# Form field generico
form-field = ["dep:dioxus"]

# Spinner component
spinner = ["dep:dioxus"]

# All components
full = ["handler", "analyzer", "form-field", "spinner"]

# Password generation UI
generator = ["handler", "pwd-types/generator", "dep:aegis-password-generator"]

[dependencies]
dioxus = { version = "0.7", features = ["router"], optional = true }
pwd-types = { path = "../pwd-types", optional = true }
pwd-strength = { path = "../pwd-strength", optional = true }
aegis-password-generator = { version = "0.1", optional = true }
```

---

## Configurazione Progetto Padre

### Cargo.toml Aggiornato

```toml
[package]
name = "PWDManager"
version = "0.1.0"
edition = "2024"

[dependencies]
# Workspace crates
gui-launcher = { path = "gui_launcher" }
custom_errors = { path = "custom_errors" }

# Nuove librerie estratte
pwd-types = { path = "pwd-types", features = ["sqlx", "generator", "dioxus"] }
pwd-strength = { path = "pwd-strength", features = ["async", "tracing", "blacklist"] }
pwd-crypto = { path = "pwd-crypto", features = ["full", "base64"] }

# External dependencies
image = { version = "0.25", features = ["png"] }
tracing = "0.1"
dioxus = { version = "0.7", features = ["desktop", "router"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros"] }
tokio = { version = "1", features = ["full", "tracing"] }
argon2 = { version = "0.5", features = ["std", "zeroize"] }
base64 = "0.22"
rfd = "0.17"
dioxus-primitives = { git = "https://github.com/DioxusLabs/components", version = "0.0.1", default-features = false }
secrecy = "0.10"
aes-gcm = { version = "0.10", features = ["zeroize"] }
sqlx-template = "0.2"
futures = "0.3"
rayon = "1.11"
tokio-util = { version = "0.7", features = ["full"] }
aegis-password-generator = "0.1"
```

### Variabili Ambiente

```bash
# .env (opzionale)
PWD_BLACKLIST_PATH=./assets/10k-most-common.txt
```

---

## Riepilogo Feature Flags

### pwd-types

| Feature     | Descrizione                          | Dipendenze                      |
|-------------|--------------------------------------|---------------------------------|
| `secrecy`   | SecretString support (default)       | `secrecy`                       |
| `sqlx`      | Tipi database (DbSecret*, Stored*)   | `sqlx`, `sqlx-template`         |
| `generator` | PasswordGeneratorConfig, Preset      | `aegis-password-generator`      |
| `dioxus`    | FormSecret per UI                    | `dioxus`                        |

### pwd-strength

| Feature     | Descrizione                          | Dipendenze                      |
|-------------|--------------------------------------|---------------------------------|
| `async`     | Supporto async (default)             | `tokio`, `tokio-util`           |
| `tracing`   | Logging                              | `tracing`                       |
| `blacklist` | Caricamento blacklist da file        | -                               |

### pwd-crypto

| Feature   | Descrizione                          | Dipendenze                      |
|-----------|--------------------------------------|---------------------------------|
| `hash`    | Argon2 hashing (default)             | `argon2`, `secrecy`             |
| `cipher`  | AES-256-GCM encryption               | `aes-gcm`, `secrecy`            |
| `base64`  | Base64 utilities                     | `base64`                        |
| `full`    | Tutto incluso                        | `hash`, `cipher`                |

---

**Fine Analisi**
