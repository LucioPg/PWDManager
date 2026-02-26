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

### Feature Flags Consigliati

```toml
# pwd-types/Cargo.toml
[features]
default = []
sqlx = ["dep:sqlx", "dep:sqlx-template"]
secrecy = ["dep:secrecy"]
```

### Versioning

- Iniziare con version `0.1.0` per tutte le nuove librerie
- Mantenere compatibilita semantica durante sviluppo iniziale
- NON pubblicare su crates.io

---

**Fine Analisi**
