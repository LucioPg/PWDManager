# Recovery Key Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add diceware-based database recovery mechanism with Argon2id key derivation, replacing the random-key-only approach.

**Architecture:** The recovery passphrase (6 diceware words) is the source of truth. The encryption key is derived via Argon2id from the passphrase + a salt file stored alongside the DB. The OS keyring serves as a convenience cache. If the keyring is lost/wrong, the user enters their passphrase to recover access.

**Tech Stack:** Rust, Dioxus 0.7, SQLCipher (via libsqlite3-sys), argon2 0.5.3, diceware crate, keyring 3, DaisyUI 5

**Spec:** `docs/superpowers/specs/2026-03-21-recovery-key-design.md`

**Note:** Regeneration from Settings (RecoveryKeyRegeneratedDialog) is deferred to a follow-up plan.

---

## File Map

| File | Responsibility |
|------|---------------|
| `custom_errors/src/lib.rs` (external crate) | New `DBError` variants |
| `src/backend/db_key.rs` | Key derivation, salt I/O, recovery passphrase, recovery/reset functions |
| `src/backend/db_backend.rs` | `InitResult` enum, rewritten `init_db()` |
| `src/main.rs` | Match on `InitResult`, recovery dialog state management |
| `src/components/globals/dialogs/recovery_key_setup.rs` | First-setup passphrase display dialog |
| `src/components/globals/dialogs/recovery_key_input.rs` | Recovery passphrase input dialog |
| `src/components/globals/dialogs/database_reset.rs` | Database reset confirmation dialog |
| `src/components/globals/dialogs/recovery_key_regenerate.rs` | Regeneration confirmation dialog (for Settings) |
| `src/components/globals/dialogs/mod.rs` | Register new dialog modules |

---

### Task 1: Add DBError variants to custom_errors crate

**Files:**
- Modify: `C:/Users/Lucio/RustroverProjects/custom_errors/src/lib.rs`

- [ ] **Step 1: Add new error variants to DBError enum**

Add these three variants after `DBRegistrationError` (line 42):

```rust
    #[error("Database key missing: recovery key required")]
    DBKeyMissingWithDb,
    #[error("Invalid recovery key")]
    DBRecoveryKeyInvalid,
    #[error("Salt file error: {0}")]
    DBSaltFileError(String),
```

- [ ] **Step 2: Add constructor helpers**

Add after `new_registration_error` (line 115):

```rust
    pub fn new_key_missing_with_db() -> Self {
        DBError::DBKeyMissingWithDb
    }

    pub fn new_recovery_key_invalid() -> Self {
        DBError::DBRecoveryKeyInvalid
    }

    pub fn new_salt_file_error(msg: String) -> Self {
        DBError::DBSaltFileError(msg)
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cd "C:/Users/Lucio/RustroverProjects/custom_errors" && cargo build`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
cd "C:/Users/Lucio/RustroverProjects/custom_errors"
git add src/lib.rs
git commit -m "feat: add DBError variants for recovery key (missing, invalid, salt)"
```

---

### Task 2: Expand DBKeyError and add core functions to db_key.rs

**Files:**
- Modify: `src/backend/db_key.rs`

This task adds the new `DBKeyError` variants, the `derive_key` function, salt file I/O, and `generate_recovery_passphrase`.

- [ ] **Step 1: Expand DBKeyError enum**

Replace the current `DBKeyError` enum (lines 11-17) with:

```rust
#[derive(Debug)]
pub enum DBKeyError {
    /// The requested credential does not exist in the keyring.
    NoEntry,
    /// A keyring system error (service unavailable, access denied, etc.)
    KeyringError(String),
    /// DB exists but open fails (keyring empty or has wrong key) → recovery needed
    MissingKeyWithDb,
    /// Passphrase does not open the DB
    RecoveryKeyInvalid,
    /// Error reading/writing salt file
    SaltFileError(String),
}
```

- [ ] **Step 2: Update Display impl**

Replace the `Display` impl (lines 19-26) with:

```rust
impl std::fmt::Display for DBKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBKeyError::NoEntry => write!(f, "Keyring entry not found"),
            DBKeyError::KeyringError(msg) => write!(f, "Keyring error: {}", msg),
            DBKeyError::MissingKeyWithDb => write!(f, "Database encryption key missing or invalid"),
            DBKeyError::RecoveryKeyInvalid => write!(f, "Invalid recovery key"),
            DBKeyError::SaltFileError(msg) => write!(f, "Salt file error: {}", msg),
        }
    }
}
```

- [ ] **Step 3: Add imports for Argon2 and diceware**

Add these imports at the top of the file (after the existing `use` statements on lines 1-2):

```rust
use argon2::Argon2;
use crate::backend::password_utils::detect_system_language;
use crate::backend::settings_types::DicewareLanguage;
```

- [ ] **Step 4: Add `derive_key` function**

Add after the `delete_db_key` function (after line 61):

```rust
/// Derives a 64-char hex key from a passphrase and salt using Argon2id.
/// MUST be called via `spawn_blocking` — CPU-bound operation (~50ms).
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<String, DBKeyError> {
    if salt.len() != 16 {
        return Err(DBKeyError::SaltFileError(
            format!("Salt must be 16 bytes, got {}", salt.len()),
        ));
    }
    let argon2 = Argon2::default();
    let mut output = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut output)
        .map_err(|e| DBKeyError::KeyringError(format!("Key derivation failed: {}", e)))?;
    Ok(output.iter().map(|b| format!("{:02x}", b)).collect())
}
```

- [ ] **Step 5: Add salt file I/O functions**

```rust
/// Returns the salt file path for a given DB path.
pub fn salt_file_path(db_path: &str) -> String {
    format!("{}.salt", db_path)
}

/// Reads 16 bytes from the salt file and returns them.
pub fn read_salt(db_path: &str) -> Result<Vec<u8>, DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let hex = std::fs::read_to_string(&salt_path).map_err(|e| {
        DBKeyError::SaltFileError(format!("Cannot read salt file '{}': {}", salt_path, e))
    })?;
    let hex = hex.trim();
    if hex.len() != 32 {
        return Err(DBKeyError::SaltFileError(format!(
            "Invalid salt length: expected 32 hex chars, got {}",
            hex.len()
        )));
    }
    (0..16)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| {
                DBKeyError::SaltFileError(format!("Invalid hex in salt file: {}", e))
            })
        })
        .collect()
}

/// Writes 16 salt bytes to the salt file as 32 hex characters.
pub fn write_salt(db_path: &str, salt: &[u8; 16]) -> Result<(), DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let hex: String = salt.iter().map(|b| format!("{:02x}", b)).collect();
    std::fs::write(&salt_path, &hex).map_err(|e| {
        DBKeyError::SaltFileError(format!("Cannot write salt file '{}': {}", salt_path, e))
    })
}
```

- [ ] **Step 6: Add `generate_db_salt` function**

Named `generate_db_salt` to avoid collision with `generate_salt` from `utils.rs` (re-exported in `mod.rs`).

```rust
/// Generates 16 random bytes for use as Argon2 salt.
pub fn generate_db_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::rng().fill(&mut salt);
    salt
}
```

- [ ] **Step 7: Add `generate_recovery_passphrase` function**

```rust
/// Generates a 6-word diceware passphrase in CamelCase using the system language.
pub fn generate_recovery_passphrase() -> Result<String, DBKeyError> {
    let lang: DicewareLanguage = detect_system_language();
    let embedded_lang: diceware::EmbeddedList = lang.into();
    let config = diceware::Config::new()
        .with_embedded(embedded_lang)
        .with_words(6)
        .with_camel_case(true);
    diceware::make_passphrase(config)
        .map_err(|e| DBKeyError::KeyringError(format!("Failed to generate passphrase: {}", e)))
}
```

- [ ] **Step 8: Write tests for core functions**

Replace the existing `#[cfg(test)] mod tests` block (lines 105-161) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SERVICE: &str = "PWDManager-test";
    const TEST_USER: &str = "test-key";

    fn cleanup() {
        if let Ok(entry) = Entry::new(TEST_SERVICE, TEST_USER) {
            let _ = entry.delete_credential();
        }
    }

    #[test]
    fn test_generate_key_returns_64_char_hex_string() {
        let key = generate_key();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_key_is_unique() {
        let key1 = generate_key();
        let key2 = generate_key();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_store_and_retrieve_key() {
        cleanup();
        let key = generate_key();
        store_db_key(TEST_SERVICE, TEST_USER, &key).unwrap();
        let retrieved = retrieve_db_key(TEST_SERVICE, TEST_USER).unwrap();
        assert_eq!(key, retrieved);
        cleanup();
    }

    #[test]
    fn test_retrieve_nonexistent_key_returns_no_entry() {
        cleanup();
        let result = retrieve_db_key(TEST_SERVICE, TEST_USER);
        assert!(matches!(result, Err(DBKeyError::NoEntry)));
        cleanup();
    }

    #[test]
    fn test_derive_key_is_deterministic() {
        let salt = generate_db_salt();
        let key1 = derive_key("test passphrase", &salt).unwrap();
        let key2 = derive_key("test passphrase", &salt).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 64);
    }

    #[test]
    fn test_derive_key_different_passphrases_produce_different_keys() {
        let salt = generate_db_salt();
        let key1 = derive_key("passphrase one", &salt).unwrap();
        let key2 = derive_key("passphrase two", &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_rejects_wrong_salt_length() {
        let result = derive_key("test", &[0u8; 8]);
        assert!(matches!(result, Err(DBKeyError::SaltFileError(_))));
    }

    #[test]
    fn test_salt_file_roundtrip() {
        let dir = std::env::temp_dir().join("pwd_test_salt");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("test.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        write_salt(&db_path, &salt).unwrap();
        let read_back = read_salt(&db_path).unwrap();
        assert_eq!(salt.to_vec(), read_back);

        let _ = std::fs::remove_file(salt_file_path(&db_path));
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_read_salt_missing_file() {
        let result = read_salt("/nonexistent/path/database.db");
        assert!(matches!(result, Err(DBKeyError::SaltFileError(_))));
    }

    #[test]
    fn test_generate_db_salt_is_unique() {
        let salt1 = generate_db_salt();
        let salt2 = generate_db_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_generate_recovery_passphrase_format() {
        let passphrase = generate_recovery_passphrase().unwrap();
        // CamelCase words: no spaces, starts with uppercase
        assert!(!passphrase.contains(' '), "Should not contain spaces");
        assert!(
            passphrase.chars().next().unwrap().is_uppercase(),
            "Should start with uppercase"
        );
        // Should have multiple words (CamelCase boundaries)
        let word_count = passphrase.chars().filter(|c| c.is_uppercase()).count();
        assert!(word_count >= 6, "Should have at least 6 uppercase chars (6 words)");
    }

    #[test]
    fn test_generate_recovery_passphrase_is_unique() {
        let p1 = generate_recovery_passphrase().unwrap();
        let p2 = generate_recovery_passphrase().unwrap();
        assert_ne!(p1, p2);
    }
}
```

- [ ] **Step 9: Run tests**

Run: `cargo test --lib backend::db_key -- --nocapture`
Expected: All 12 tests pass

- [ ] **Step 10: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "feat: expand db_key with recovery key derivation and salt file I/O"
```

---

### Task 3: Add recovery and reset functions to db_key.rs

**Files:**
- Modify: `src/backend/db_key.rs`

- [ ] **Step 1: Add `derive_key_from_passphrase` convenience function**

Add after `generate_recovery_passphrase`:

```rust
/// Derives the DB key from a user-entered recovery passphrase.
/// Reads the salt file automatically. MUST be called via `spawn_blocking`.
pub fn derive_key_from_passphrase(passphrase: &str, db_path: &str) -> Result<String, DBKeyError> {
    let salt = read_salt(db_path)?;
    derive_key(passphrase, &salt)
}
```

- [ ] **Step 2: Add `generate_and_store_key` function**

```rust
/// Generates a new salt, derives key from passphrase, stores in keyring.
/// Returns the derived key hex string.
pub fn generate_and_store_key(
    passphrase: &str,
    db_path: &str,
) -> Result<String, DBKeyError> {
    let salt = generate_db_salt();
    write_salt(db_path, &salt)?;
    let key = derive_key(passphrase, &salt)?;
    store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
    Ok(key)
}
```

- [ ] **Step 3: Add `reset_database` function**

```rust
/// Deletes the database file and salt file for a fresh start.
pub fn reset_database(db_path: &str) -> Result<(), DBKeyError> {
    let salt_path = salt_file_path(db_path);
    let mut errors = Vec::new();

    if std::path::Path::new(db_path).exists() {
        if let Err(e) = std::fs::remove_file(db_path) {
            errors.push(format!("Failed to delete DB: {}", e));
        }
    }
    // Remove WAL/SHM files if present
    for suffix in &["-wal", "-shm"] {
        let path = format!("{}{}", db_path, suffix);
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
    if std::path::Path::new(&salt_path).exists() {
        if let Err(e) = std::fs::remove_file(&salt_path) {
            errors.push(format!("Failed to delete salt: {}", e));
        }
    }
    // delete_db_key returns () — just call it, ignore any error
    delete_db_key(SERVICE_NAME, KEY_USERNAME);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(DBKeyError::KeyringError(errors.join("; ")))
    }
}
```

- [ ] **Step 4: Keep `get_or_create_db_key` but mark deprecated**

Replace the existing `get_or_create_db_key` function with a simplified version. Kept temporarily so the project still compiles (it will be replaced by `init_db` in Task 4):

```rust
/// @deprecated: Replaced by the recovery key flow in init_db().
/// Kept temporarily for compilation. Will be removed in the init_db rewrite.
#[allow(dead_code)]
pub fn get_or_create_db_key(db_path: &str) -> Result<String, DBKeyError> {
    match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
        Ok(key) => Ok(key),
        Err(DBKeyError::NoEntry) => Err(DBKeyError::MissingKeyWithDb),
        Err(e) => Err(e),
    }
}
```

- [ ] **Step 5: Add tests for new functions**

Add these tests inside the `mod tests` block (before the closing `}`):

```rust
    #[test]
    fn test_reset_database_removes_files() {
        let dir = std::env::temp_dir().join("pwd_test_reset");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("database.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        // Create files
        std::fs::write(&db_path, "test data").unwrap();
        write_salt(&db_path, &salt).unwrap();

        assert!(std::path::Path::new(&db_path).exists());

        reset_database(&db_path).unwrap();

        assert!(!std::path::Path::new(&db_path).exists());
        assert!(!std::path::Path::new(&salt_file_path(&db_path)).exists());

        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_generate_and_store_key() {
        cleanup();
        let dir = std::env::temp_dir().join("pwd_test_gen_store");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("database.db").to_str().unwrap().to_string();

        let key = generate_and_store_key("MyTestPassphrase123", &db_path).unwrap();

        assert_eq!(key.len(), 64);
        assert!(std::path::Path::new(&salt_file_path(&db_path)).exists());

        // Verify the stored key matches what derive_key produces
        let salt = read_salt(&db_path).unwrap();
        let derived = derive_key("MyTestPassphrase123", &salt).unwrap();
        assert_eq!(key, derived);

        cleanup();
        let _ = std::fs::remove_file(salt_file_path(&db_path));
        let _ = std::fs::remove_dir(&dir);
    }
```

- [ ] **Step 6: Run tests**

Run: `cargo test --lib backend::db_key -- --nocapture`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "feat: add recovery, reset, and key store functions to db_key"
```

---

### Task 4: Rewrite init_db() in db_backend.rs

**Files:**
- Modify: `src/backend/db_backend.rs`

**Note:** The removal of `create_if_missing(true)` may affect existing tests in `db_backend_tests.rs`. Check and update them if needed.

- [ ] **Step 1: Add InitResult enum**

Add the enum before `init_db()` (after the `is_database_unencrypted` function, around line 77):

```rust
/// Result of database initialization.
#[cfg(feature = "desktop")]
pub enum InitResult {
    /// Normal startup or recovery completed successfully.
    Ready(SqlitePool),
    /// First setup: DB created, passphrase generated, show to user.
    FirstSetup {
        pool: SqlitePool,
        recovery_phrase: SecretString,
    },
}
```

`SecretString` is already imported on line 13.

- [ ] **Step 2: Rewrite `init_db()` function**

Replace the existing `init_db()` function (lines 182-223) with:

```rust
#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<InitResult, DBError> {
    let db_path = std::env::current_dir()
        .unwrap_or_default()
        .join("database.db");
    let db_path = db_path
        .to_str()
        .ok_or_else(|| DBError::new_general_error("Invalid DB path".into()))?;

    let db_exists = std::path::Path::new(db_path).exists();
    let salt_path = db_key::salt_file_path(db_path);
    let salt_exists = std::path::Path::new(&salt_path).exists();

    if !db_exists && !salt_exists {
        // FIRST SETUP
        info!("First setup: generating recovery key and creating database");

        let passphrase = db_key::generate_recovery_passphrase()
            .map_err(|e| DBError::new_general_error(format!("Passphrase generation: {}", e)))?;

        let passphrase_secret = SecretString::new(passphrase.clone().into());

        let db_key = tokio::task::spawn_blocking({
            let passphrase = passphrase.clone();
            let db_path = db_path.to_string();
            move || db_key::generate_and_store_key(&passphrase, &db_path)
        })
        .await
        .map_err(|e| DBError::new_general_error(format!("Key derivation task failed: {}", e)))?
        .map_err(|e| DBError::new_general_error(format!("Key setup failed: {}", e)))?;

        let pragma_key_value = format!("\"x'{}'\"", db_key);

        let connect_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
            .map_err(|e| DBError::new_general_error(e.to_string()))?
            .pragma("key", pragma_key_value)
            .pragma("foreign_keys", "ON")
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(connect_options)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to create database: {}", e)))?;

        for init_query in QUERIES {
            query(init_query)
                .execute(&pool)
                .await
                .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
        }

        return Ok(InitResult::FirstSetup {
            pool,
            recovery_phrase: passphrase_secret,
        });
    }

    // DB exists or salt exists → try normal startup
    if !salt_exists {
        return Err(DBError::new_salt_file_error(
            "Salt file missing or corrupted. Database reset is required.".into(),
        ));
    }

    // Try to get key from keyring
    let keyring_result = db_key::retrieve_db_key(db_key::SERVICE_NAME, db_key::KEY_USERNAME);

    match keyring_result {
        Ok(key) => {
            // Try to open DB with keyring key
            let pragma_key_value = format!("\"x'{}'\"", key);
            let connect_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
                .map_err(|e| DBError::new_general_error(e.to_string()))?
                .pragma("key", pragma_key_value)
                .pragma("foreign_keys", "ON")
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);

            match SqlitePool::connect_with(connect_options).await {
                Ok(pool) => Ok(InitResult::Ready(pool)),
                Err(_) => Err(DBError::new_key_missing_with_db()),
            }
        }
        Err(_) => Err(DBError::new_key_missing_with_db()),
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: compiles without errors. Note: `main.rs` will have a type mismatch — that's fixed in Task 6.

- [ ] **Step 4: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: rewrite init_db with recovery key flow and InitResult"
```

---

### Task 5: Create dialog components

**Files:**
- Create: `src/components/globals/dialogs/recovery_key_setup.rs`
- Create: `src/components/globals/dialogs/recovery_key_input.rs`
- Create: `src/components/globals/dialogs/database_reset.rs`
- Create: `src/components/globals/dialogs/recovery_key_regenerate.rs`
- Modify: `src/components/globals/dialogs/mod.rs`

Reference pattern: `src/components/globals/dialogs/user_deletion.rs`

All UI text is in English.

- [ ] **Step 1: Create RecoveryKeySetupDialog**

Create `src/components/globals/dialogs/recovery_key_setup.rs`:

```rust
use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeySetupDialog(
    open: Signal<bool>,
    passphrase: String,
    on_confirm: EventHandler<()>,
) -> Element {
    // Non-dismissable: no X button, no cancel
    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {}, // Intentionally empty — non-dismissable
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Title
            h3 { class: "font-bold text-lg mb-4 text-center", "Recovery Key" }

            // Info text
            p { class: "py-2 text-center",
                "Save these words in a safe place."
            }
            p { class: "py-2 text-center",
                "You will need them if the encryption key is lost."
            }

            // Passphrase display
            div { class: "bg-base-300 rounded-lg p-4 my-4 mx-4 text-center font-mono text-lg break-all select-all",
                "{passphrase}"
            }

            // Warning
            p { class: "text-warning py-2 text-center text-sm",
                strong { "Warning: " }
                "Without this recovery key, your data will be permanently lost if the encryption key is lost."
            }

            // Action button
            div { class: "modal-action",
                ActionButton {
                    text: "I have saved the recovery key".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_confirm.call(());
                        open.set(false);
                    },
                }
            }
        }
    }
}
```

- [ ] **Step 2: Create RecoveryKeyInputDialog**

Create `src/components/globals/dialogs/recovery_key_input.rs`:

```rust
use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeyInputDialog(
    open: Signal<bool>,
    error: Signal<bool>,
    on_recover: EventHandler<String>,
    on_reset: EventHandler<()>,
) -> Element {
    let mut input_value = use_signal(|| String::new());
    let mut open_clone = open.clone();

    let handle_recover = move |_| {
        let passphrase = input_value.read().clone();
        if !passphrase.trim().is_empty() {
            on_recover.call(passphrase);
            input_value.set(String::new());
        }
    };

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Title
            h3 { class: "font-bold text-lg mb-2", "Recovery Key" }

            // Info text
            p { class: "py-2",
                "The encryption key is not available or invalid."
            }
            p { class: "py-2",
                "Enter your recovery key to restore access."
            }

            // Text input
            input {
                class: "input input-bordered w-full my-4 font-mono",
                r#type: "text",
                placeholder: "Enter your recovery key...",
                value: "{input_value}",
                oninput: move |e| {
                    input_value.set(e.value());
                    error.set(false);
                },
                onkeydown: move |e: KeyboardEvent| {
                    if e.code() == Code::Enter {
                        handle_recover.call(());
                    }
                },
            }

            // Error message (conditional)
            if error() {
                p { class: "text-error py-2 text-sm",
                    "Invalid recovery key. Please try again."
                }
            }

            // Action buttons
            div { class: "modal-action",

                ActionButton {
                    text: "Reset database".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error hover:bg-error/10".to_string(),
                    on_click: move |_| {
                        on_reset.call(());
                    },
                }

                ActionButton {
                    text: "Recover".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: handle_recover,
                }
            }
        }
    }
}
```

- [ ] **Step 3: Create DatabaseResetDialog**

Create `src/components/globals/dialogs/database_reset.rs`:

```rust
use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

#[component]
pub fn DatabaseResetDialog(
    open: Signal<bool>,
    on_confirm: EventHandler<()>,
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let mut open_clone = open.clone();

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            div { class: "alert alert-error mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            h3 { class: "font-bold text-lg mb-2", "Reset database?" }

            p { class: "py-4",
                "All data will be permanently deleted."
            }

            div { class: "modal-action",

                ActionButton {
                    text: "Cancel".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }

                ActionButton {
                    text: "Reset".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error hover:bg-error/10".to_string(),
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
```

- [ ] **Step 4: Create RecoveryKeyRegenerateDialog**

Create `src/components/globals/dialogs/recovery_key_regenerate.rs`:

```rust
use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeyRegenerateDialog(
    open: Signal<bool>,
    on_confirm: EventHandler<()>,
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let mut open_clone = open.clone();

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            h3 { class: "font-bold text-lg mb-2", "Regenerate recovery key?" }

            p { class: "py-4",
                "A new recovery key will be generated. Your data will not be lost, but the old recovery key will no longer work."
            }

            div { class: "modal-action",

                ActionButton {
                    text: "Cancel".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }

                ActionButton {
                    text: "Regenerate".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-warning hover:bg-warning/10".to_string(),
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
```

- [ ] **Step 5: Update dialogs/mod.rs**

Update `src/components/globals/dialogs/mod.rs` to add the new modules:

```rust
pub mod base_modal;
mod database_reset;
mod export_progress;
mod export_warning;
mod import_progress;
mod import_warning;
mod migration_progress;
mod migration_warning;
mod recovery_key_input;
mod recovery_key_regenerate;
mod recovery_key_setup;
mod stored_all_passwords_deletion;
mod stored_password_deletion;
mod stored_password_show;
mod stored_password_upsert;
pub mod user_deletion;

pub use base_modal::*;
pub use database_reset::*;
pub use recovery_key_input::*;
pub use recovery_key_regenerate::*;
pub use recovery_key_setup::*;
pub use stored_password_deletion::*;
pub use user_deletion::*;

// ide-only serve per avere highlight mentre si lavora su elementi non ancora completati.
// #[cfg(feature = "ide-only")]
pub use export_progress::*;
pub use export_warning::*;
pub use import_progress::*;
pub use import_warning::*;
pub use migration_progress::*;
pub use migration_warning::*;
pub use stored_all_passwords_deletion::*;
pub use stored_password_show::*;
pub use stored_password_upsert::*;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build`
Expected: compiles without errors

- [ ] **Step 7: Commit**

```bash
git add src/components/globals/dialogs/
git commit -m "feat: add recovery key dialog components"
```

---

### Task 6: Integrate recovery flow in main.rs

**Files:**
- Modify: `src/main.rs`

This is the most complex task. It wires everything together: matching on `InitResult`, managing dialog states, and handling the recovery/reset callbacks.

- [ ] **Step 1: Update imports**

Add to the imports in `src/main.rs`:

```rust
use crate::backend::db_backend::InitResult;
use crate::components::globals::dialogs::{
    DatabaseResetDialog, RecoveryKeyInputDialog, RecoveryKeySetupDialog,
};
```

- [ ] **Step 2: Rewrite the App component**

Replace the entire `App` component (lines 38-171) with:

```rust
#[component]
fn App() -> Element {
    let auth_state = auth::AuthState::new();
    use_context_provider(move || auth_state);
    let mut app_theme = use_signal(|| Theme::Light);
    let mut auto_update = use_signal(|| AutoUpdate::default());
    use_context_provider(move || app_theme);
    use_context_provider(|| auto_update);
    let mut update_state = use_signal(|| UpdateState::Idle);
    use_context_provider(|| update_state);
    let mut update_manifest = use_signal(|| None::<UpdateManifest>);
    use_context_provider(|| update_manifest);
    use_context_provider(|| Signal::new(ToastHubState::default()));

    let mut db_resource = use_resource(move || async move { init_db().await });
    let db_resource_clone_drop = db_resource.clone();
    #[allow(unused_mut)]
    let mut spawn_handle = use_signal(|| Option::<Task>::None);
    let mut toast_state = use_context::<Signal<ToastHubState>>();
    let mut db_init_notified = use_signal(|| false);
    let mut users_list_printed = use_signal(|| false);

    // Recovery dialog state
    let mut show_recovery_dialog = use_signal(|| false);
    let mut recovery_error = use_signal(|| false);
    let mut show_reset_dialog = use_signal(|| false);
    let mut show_setup_dialog = use_signal(|| false);
    let mut setup_passphrase = use_signal(|| String::new());

    // Cleanup del pool quando il componente viene smontato o l'app si chiude
    use_drop(move || {
        let db_resource_clone = db_resource_clone_drop.clone();
        match &*db_resource_clone.read() {
            Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
                println!("Cleanup: chiudo connessioni DB prima dell'uscita");
                let pool_clone = pool.clone();
                spawn(async move {
                    let _ = pool_clone.close().await;
                });
            }
            _ => println!("Cleanup: pool non presente"),
        }
    });

    use_effect(move || {
        let blacklist_path = BLACKLIST_ASSET.to_string();
        let blacklist_path = blacklist_path.trim_start_matches('/');
        if let Err(e) = init_blacklist_from_path(blacklist_path) {
            let error = format!("BLACKLIST Loading is Failed!: {}", e.to_string());
            show_toast_error(error, toast_state);
        }
    });

    // Effect: handle DB resource changes
    use_effect(move || {
        let db_resource_clone = db_resource.clone();
        let resource = db_resource_clone.read();

        match &*resource {
            Some(Ok(InitResult::Ready(_))) => {
                if !db_init_notified() {
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                }
            }
            Some(Ok(InitResult::FirstSetup { .. })) => {
                if !db_init_notified() {
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                }
            }
            Some(Err(custom_errors::DBError::DBKeyMissingWithDb)) => {
                show_recovery_dialog.set(true);
                show_toast_error("Recovery key required".into(), toast_state);
            }
            Some(Err(_)) => {
                show_toast_error("Database Loading failed!".into(), toast_state);
            }
            None => {}
        }
    });

    // Effect: debug user list
    use_effect(move || {
        let db_resource_clone = db_resource.clone();

        match &*db_resource_clone.read() {
            Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
                let mut spawn_handle = spawn_handle.clone();
                if let Some(new_handle) = spawn_handle.take() {
                    new_handle.cancel();
                }
                if cfg!(debug_assertions) {
                    if !users_list_printed() {
                        let pool_clone = pool.clone();
                        let handle = spawn(async move {
                            match list_users_no_avatar(&pool_clone).await {
                                Ok(users) => {
                                    println!("=== LISTA UTENTI ===");
                                    println!("ID  --  Username  --  Creation Date");
                                    for (id, username, password) in users {
                                        println!("{}\t{}\t{}", id, username, password);
                                    }
                                    println!("===================");
                                    users_list_printed.set(true);
                                }
                                Err(e) => {
                                    println!("Errore nel recupero utenti: {:?}", e);
                                }
                            }
                        });
                        spawn_handle.set(Some(handle));
                    }
                }
            }
            _ => {}
        }
    });

    // Effect: detect FirstSetup and show dialog
    use_effect(move || {
        let resource = db_resource.read();
        if let Some(Ok(InitResult::FirstSetup { recovery_phrase, .. })) = &*resource {
            setup_passphrase.set(recovery_phrase.expose_secret().to_string());
            show_setup_dialog.set(true);
        }
    });

    match &*db_resource.read() {
        Some(Ok(InitResult::Ready(pool))) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
        Some(Ok(InitResult::FirstSetup { pool, .. })) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
        Some(Err(custom_errors::DBError::DBKeyMissingWithDb)) => {
            render_recovery_ui(
                db_resource,
                show_recovery_dialog,
                recovery_error,
                show_reset_dialog,
                db_init_notified,
                toast_state,
            )
        }
        Some(Err(custom_errors::DBError::DBSaltFileError(msg))) => {
            render_salt_error_ui(db_resource, show_reset_dialog, msg.clone(), db_init_notified, toast_state)
        }
        Some(Err(e)) => {
            rsx! {
                Style {}
                div { class: "error-container",
                    h1 { "Critical Database Error" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Retry" }
                }
            }
        }
        None => {
            rsx! {
                Style {}
                div { class: "flex gap-4 justify-center items-center h-screen",
                    Spinner {
                        size: SpinnerSize::XXXXLarge,
                        color_class: "text-blue-500",
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Add helper render functions**

Add these functions before the `main()` function (before line 172):

```rust
fn render_app_with_setup(
    pool: &SqlitePool,
    show_setup_dialog: Signal<bool>,
    setup_passphrase: Signal<String>,
    update_state: Signal<UpdateState>,
) -> Element {
    rsx! {
        Style {}
        ToastContainer {}
        UpdateNotification { update_state }
        Router::<Route> {}

        RecoveryKeySetupDialog {
            open: show_setup_dialog,
            passphrase: setup_passphrase.read().clone(),
            on_confirm: move |_| {},
        }
    }
}

fn render_recovery_ui(
    db_resource: Signal<Resource<Result<InitResult, custom_errors::DBError>>>,
    show_recovery_dialog: Signal<bool>,
    recovery_error: Signal<bool>,
    show_reset_dialog: Signal<bool>,
    db_init_notified: Signal<bool>,
    toast_state: Signal<ToastHubState>,
) -> Element {
    let handle_recover = move |passphrase: String| {
        let passphrase = passphrase.clone();
        spawn(async move {
            let db_path = std::env::current_dir()
                .unwrap_or_default()
                .join("database.db")
                .to_str()
                .unwrap()
                .to_string();

            let derive_result = tokio::task::spawn_blocking({
                let p = passphrase.clone();
                let path = db_path.clone();
                move || {
                    let salt = crate::backend::db_key::read_salt(&path)?;
                    crate::backend::db_key::derive_key(&p, &salt)
                }
            })
            .await;

            let key = match derive_result {
                Ok(Ok(key)) => key,
                _ => {
                    recovery_error.set(true);
                    return;
                }
            };

            // Try to open DB with derived key
            let pragma = format!("\"x'{}'\"", key);
            let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
                .unwrap()
                .pragma("key", pragma)
                .pragma("foreign_keys", "ON")
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);

            match sqlx::SqlitePool::connect_with(opts).await {
                Ok(pool) => {
                    // Store key in keyring
                    let _ = crate::backend::db_key::store_db_key(
                        crate::backend::db_key::SERVICE_NAME,
                        crate::backend::db_key::KEY_USERNAME,
                        &key,
                    );
                    recovery_error.set(false);
                    show_recovery_dialog.set(false);
                    db_init_notified.set(false);
                    // Restart will re-init normally with the key now in keyring
                    db_resource.restart();
                }
                Err(_) => {
                    recovery_error.set(true);
                }
            }
        });
    };

    let handle_reset = move |_| {
        let db_path = std::env::current_dir()
            .unwrap_or_default()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        let _ = crate::backend::db_key::reset_database(&db_path);
        db_init_notified.set(false);
        db_resource.restart();
    };

    rsx! {
        Style {}
        div { class: "flex gap-4 justify-center items-center h-screen",
            Spinner {
                size: SpinnerSize::XXXXLarge,
                color_class: "text-blue-500",
            }
        }

        RecoveryKeyInputDialog {
            open: show_recovery_dialog,
            error: recovery_error,
            on_recover: move |p: String| handle_recover(p),
            on_reset: move |_| show_reset_dialog.set(true),
        }

        DatabaseResetDialog {
            open: show_reset_dialog,
            on_confirm: move |_| handle_reset(),
        }
    }
}

fn render_salt_error_ui(
    db_resource: Signal<Resource<Result<InitResult, custom_errors::DBError>>>,
    show_reset_dialog: Signal<bool>,
    error_msg: String,
    db_init_notified: Signal<bool>,
    toast_state: Signal<ToastHubState>,
) -> Element {
    let handle_reset = move |_| {
        let db_path = std::env::current_dir()
            .unwrap_or_default()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        let _ = crate::backend::db_key::reset_database(&db_path);
        db_init_notified.set(false);
        db_resource.restart();
    };

    rsx! {
        Style {}
        div { class: "error-container",
            h1 { "Critical Database Error" }
            p { "{error_msg}" }
            button { onclick: move |_| show_reset_dialog.set(true), "Reset database" }
        }

        DatabaseResetDialog {
            open: show_reset_dialog,
            on_confirm: move |_| handle_reset(),
        }
    }
}
```

- [ ] **Step 4: Add missing imports to main.rs**

Ensure these are in the imports:

```rust
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use std::str::FromStr;
```

Note: `SqlitePool` may need to be imported via `use backend::db_backend::init_db;` — it's already imported indirectly. Check if `pool` variable types match. The `InitResult::Ready(pool)` and `InitResult::FirstSetup { pool, .. }` both contain `SqlitePool`, which is re-exported from `db_backend`.

- [ ] **Step 5: Verify compilation**

Run: `cargo build`
Expected: compiles without errors

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: integrate recovery key flow in main.rs with dialog state"
```

---

### Task 7: Build and smoke test

**Files:** None (testing only)

- [ ] **Step 1: Full build**

Run: `cargo build`
Expected: compiles without errors

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Smoke test (manual)**

Run the app. On first launch (no database.db or database.db.salt):
1. App should create the database and show the RecoveryKeySetupDialog with a 6-word passphrase
2. Click "I have saved the recovery key" → dialog closes, app continues normally
3. Verify that `database.db.salt` exists alongside `database.db`
4. Close and reopen the app → should start normally without the setup dialog

- [ ] **Step 4: Test recovery flow (manual)**

1. Delete the keyring entry from Windows Credential Manager
2. Reopen the app → should show RecoveryKeyInputDialog
3. Enter the wrong passphrase → should show error, stay in dialog
4. Enter the correct passphrase → should open the database, show success
5. Close and reopen → should start normally (key restored in keyring)

- [ ] **Step 5: Commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: smoke test fixes for recovery key flow"
```
