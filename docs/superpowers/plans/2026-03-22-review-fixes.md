# Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 21 code review issues across db_key, init_db, dialog components, and main.rs.

**Architecture:** Sequential backend-first approach — fix error types and core functions (db_key.rs), then init_db (db_backend.rs), then dialog components, then main.rs integration. Each task is self-contained and independently committable.

**Tech Stack:** Rust, Dioxus 0.7, SQLCipher, keyring crate, Argon2id

**Spec:** `docs/superpowers/specs/2026-03-22-review-fixes-design.md`
**Issues:** `docs/22-03-2026-issues.md`

---

### Task 1: Add new DBKeyError variants and fix error mapping

**Files:**
- Modify: `src/backend/db_key.rs:16-41`

- [ ] **Step 1: Add `DerivationError` and `DatabaseCleanupError` variants to `DBKeyError`**

Add after line 27 (`SaltFileError(String),`):

```rust
    /// CPU-bound derivation failure (Argon2, diceware) — not a keyring error
    DerivationError(String),
    /// File deletion failure during database reset
    DatabaseCleanupError(String),
```

- [ ] **Step 2: Add Display arms for new variants**

Add in the `Display` impl (after line 36):

```rust
            DBKeyError::DerivationError(msg) => write!(f, "Derivation error: {}", msg),
            DBKeyError::DatabaseCleanupError(msg) => write!(f, "Database cleanup error: {}", msg),
```

- [ ] **Step 3: Fix `derive_key` error mapping (Task 2 I-1)**

Change line 88 from:
```rust
        .map_err(|e| DBKeyError::KeyringError(format!("Key derivation failed: {}", e)))?;
```
to:
```rust
        .map_err(|e| DBKeyError::DerivationError(format!("Key derivation failed: {}", e)))?;
```

- [ ] **Step 4: Fix `generate_recovery_passphrase` error mapping (Task 2 I-2)**

Change line 144 from:
```rust
        .map_err(|e| DBKeyError::KeyringError(format!("Failed to generate passphrase: {}", e)))
```
to:
```rust
        .map_err(|e| DBKeyError::DerivationError(format!("Failed to generate passphrase: {}", e)))
```

- [ ] **Step 5: Fix `reset_database` error mapping for DB file deletion (Task 3 I-1)**

Change line 195 from:
```rust
        Err(DBKeyError::SaltFileError(errors.join("; ")))
```
to:
```rust
        Err(DBKeyError::DatabaseCleanupError(errors.join("; ")))
```

- [ ] **Step 6: Run tests to verify changes**

Run: `cargo test --features desktop -- db_key`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "fix: add DerivationError/DatabaseCleanupError variants, fix error mapping"
```

---

### Task 2: Change `read_salt` return type to `[u8; 16]`

**Files:**
- Modify: `src/backend/db_key.rs:98-117`

- [ ] **Step 1: Update `read_salt` signature and return type (Task 2 I-3)**

Replace lines 98-117 with:

```rust
/// Reads 16 bytes from the salt file and returns them as a fixed-size array.
pub fn read_salt(db_path: &str) -> Result<[u8; 16], DBKeyError> {
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
    let bytes: Vec<u8> = (0..16)
        .map(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| {
                DBKeyError::SaltFileError(format!("Invalid hex in salt file: {}", e))
            })
        })
        .collect::<Result<Vec<u8>, _>>()?;

    bytes.try_into()
        .map_err(|_| DBKeyError::SaltFileError("Salt must be exactly 16 bytes".into()))
}
```

Note: `derive_key_from_passphrase` (line 150) works automatically since `&[u8; 16]` coerces to `&[u8]`.

- [ ] **Step 2: Update test assertions to use array instead of vec (Task 2 I-3 side-effect)**

Change line 287 from:
```rust
        assert_eq!(salt.to_vec(), read_back);
```
to:
```rust
        assert_eq!(salt, read_back);
```

- [ ] **Step 3: Run tests**

Run: `cargo test --features desktop -- db_key`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "fix: change read_salt return type from Vec<u8> to [u8; 16]"
```

---

### Task 3: Salt orphan cleanup in `generate_and_store_key`

**Files:**
- Modify: `src/backend/db_key.rs:156-165`

- [ ] **Step 1: Add cleanup logic to `generate_and_store_key` (Task 4 I-2)**

Replace lines 156-165 with:

```rust
/// Generates a new salt, derives key from passphrase, stores in keyring.
/// Returns the derived key hex string.
/// CPU-bound: call via `spawn_blocking`.
pub fn generate_and_store_key(
    passphrase: &str,
    db_path: &str,
) -> Result<String, DBKeyError> {
    let salt = generate_db_salt();
    write_salt(db_path, &salt)?;

    let result = derive_key(passphrase, &salt)
        .and_then(|key| {
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        });

    match result {
        Ok(key) => Ok(key),
        Err(e) => {
            // Cleanup orphaned salt file on failure
            let salt_path = salt_file_path(db_path);
            if let Err(cleanup_err) = std::fs::remove_file(&salt_path) {
                tracing::warn!("Failed to clean up orphaned salt file {}: {}", salt_path, cleanup_err);
            }
            Err(e)
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features desktop -- db_key`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "fix: add salt file cleanup on generate_and_store_key failure"
```

---

### Task 4: Fix `reset_database` — comments and error variant update

**Files:**
- Modify: `src/backend/db_key.rs:168-197`

- [ ] **Step 1: Add comment for WAL/SHM files (Task 3 I-2)**

Change lines 177-183 from:
```rust
    // Remove WAL/SHM files if present
    for suffix in &["-wal", "-shm"] {
        let path = format!("{}{}", db_path, suffix);
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
```
to:
```rust
    // WAL/SHM files are intentionally ignored on failure:
    // they are transient SQLite files that may or may not exist,
    // and failing to delete them is not a critical error.
    for suffix in &["-wal", "-shm"] {
        let path = format!("{}{}", db_path, suffix);
        if std::path::Path::new(&path).exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test --features desktop -- db_key::tests::test_reset_database`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "fix: add comment explaining WAL/SHM silent ignore in reset_database"
```

---

### Task 5: Fix `get_or_create_db_key` deprecation and doc

**Files:**
- Modify: `src/backend/db_key.rs:199-208`

- [ ] **Step 1: Replace `@deprecated` with `#[deprecated]` attribute (Task 3 M-3)**

Replace lines 199-201 from:
```rust
/// @deprecated: Replaced by the recovery key flow in init_db().
/// Kept temporarily for compilation. Will be removed in the init_db rewrite.
#[allow(dead_code)]
```
to:
```rust
/// Replaced by the recovery key flow in init_db().
/// Kept temporarily for compilation. Will be removed in the init_db rewrite.
#[allow(dead_code)]
#[deprecated]
```

- [ ] **Step 2: Run `cargo check` to verify compilation**

Run: `cargo check --features desktop`
Expected: May show deprecation warnings (expected), no errors.

- [ ] **Step 3: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "fix: use #[deprecated] attribute instead of @deprecated doc comment"
```

---

### Task 6: Fix test — tempdir and keyring cleanup

**Files:**
- Modify: `src/backend/db_key.rs:210-369` (test module)

- [ ] **Step 1: Check if `tempfile` is in Cargo.toml dependencies**

Run: `grep tempfile Cargo.toml`
If not present, add `tempfile = "3"` to `[dev-dependencies]`.

- [ ] **Step 2: Fix `test_salt_file_roundtrip` to use tempdir (Task 2 M-1)**

Replace lines 279-291 from:
```rust
    fn test_salt_file_roundtrip() {
        let dir = std::env::temp_dir().join("pwd_test_salt");
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("test.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        write_salt(&db_path, &salt).unwrap();
        let read_back = read_salt(&db_path).unwrap();
        assert_eq!(salt, read_back);

        let _ = std::fs::remove_file(salt_file_path(&db_path));
        let _ = std::fs::remove_dir(&dir);
    }
```
to:
```rust
    fn test_salt_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        write_salt(&db_path, &salt).unwrap();
        let read_back = read_salt(&db_path).unwrap();
        assert_eq!(salt, read_back);
    }
```

- [ ] **Step 3: Fix `test_reset_database_removes_files` to use tempdir (Task 2 M-1)**

Replace lines 328-346 from:
```rust
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
```
to:
```rust
    fn test_reset_database_removes_files() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("database.db").to_str().unwrap().to_string();
        let salt = generate_db_salt();

        // Create files
        std::fs::write(&db_path, "test data").unwrap();
        write_salt(&db_path, &salt).unwrap();

        assert!(std::path::Path::new(&db_path).exists());

        reset_database(&db_path).unwrap();

        assert!(!std::path::Path::new(&db_path).exists());
        assert!(!std::path::Path::new(&salt_file_path(&db_path)).exists());
    }
```

- [ ] **Step 4: Fix `test_generate_and_store_key` cleanup (Task 3 I-3)**

Replace lines 348-368 from:
```rust
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
to:
```rust
    fn test_generate_and_store_key() {
        cleanup();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("database.db").to_str().unwrap().to_string();

        let key = generate_and_store_key("MyTestPassphrase123", &db_path).unwrap();

        assert_eq!(key.len(), 64);
        assert!(std::path::Path::new(&salt_file_path(&db_path)).exists());

        // Verify the stored key matches what derive_key produces
        let salt = read_salt(&db_path).unwrap();
        let derived = derive_key("MyTestPassphrase123", &salt).unwrap();
        assert_eq!(key, derived);

        // Cleanup: the test writes to SERVICE_NAME (real keyring), not TEST_SERVICE
        delete_db_key(SERVICE_NAME, KEY_USERNAME);
    }
```

- [ ] **Step 5: Run all db_key tests**

Run: `cargo test --features desktop -- db_key`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/backend/db_key.rs Cargo.toml Cargo.lock
git commit -m "fix: use tempfile for tests, fix keyring cleanup in test_generate_and_store_key"
```

---

### Task 7: Extract `get_db_path()` helper and fix `init_db`

**Files:**
- Modify: `src/backend/db_backend.rs:83-167`

- [ ] **Step 1: Add `get_db_path()` helper function (Task 6 M-1)**

Add before `init_db` function (before line 83), gated with `#[cfg(feature = "desktop")]`:

```rust
/// Returns the database file path (CWD-relative).
#[cfg(feature = "desktop")]
pub fn get_db_path() -> Result<String, DBError> {
    std::env::current_dir()
        .unwrap_or_default()
        .join("database.db")
        .to_str()
        .ok_or_else(|| DBError::new_general_error("Invalid DB path".into()))
        .map(|s| s.to_string())
}
```

- [ ] **Step 2: Simplify `init_db` to use `get_db_path()` (Task 6 M-1)**

Replace lines 84-89 from:
```rust
    let db_path = std::env::current_dir()
        .unwrap_or_default()
        .join("database.db");
    let db_path = db_path
        .to_str()
        .ok_or_else(|| DBError::new_general_error("Invalid DB path".into()))?;
```
to:
```rust
    let db_path = get_db_path()?;
```

- [ ] **Step 3: Fix variable shadowing — rename `db_key` to `db_key_value` (Task 4 M-3)**

Change line 104 from:
```rust
        let db_key = tokio::task::spawn_blocking({
```
to:
```rust
        let db_key_value = tokio::task::spawn_blocking({
```

And line 113 from:
```rust
        let pragma_key_value = format!("\"x'{}'\"", db_key);
```
to:
```rust
        let pragma_key_value = format!("\"x'{}'\"", db_key_value);
```

- [ ] **Step 4: Fix connect error disambiguation (Task 4 I-3)**

Replace lines 149-166 (the `match keyring_result` block) with:

```rust
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
        Err(db_key::DBKeyError::NoEntry) => Err(DBError::new_key_missing_with_db()),
        Err(e) => Err(DBError::new_general_error(&format!("Keyring error: {}", e))),
    }
```

- [ ] **Step 5: Run `cargo check --features desktop`**

Run: `cargo check --features desktop`
Expected: Compilation succeeds

- [ ] **Step 6: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "fix: extract get_db_path helper, fix connect error disambiguation, rename db_key shadow"
```

---

### Task 8: Fix RecoveryKeyInputDialog — non-dismissable + accessibility

**Files:**
- Modify: `src/components/globals/dialogs/recovery_key_input.rs:1-97`

- [ ] **Step 1: Make dialog non-dismissable (Task 5 I-2)**

Replace lines 22-24 from:
```rust
            on_close: move |_| {
                open_clone.set(false);
            },
```
to:
```rust
            on_close: move |_| {},
```

Remove the now-unused `open_clone` variable (line 14):
```rust
    // Remove: let mut open_clone = open.clone();
```

- [ ] **Step 2: Add `autocomplete="off"` and `aria-label` (Task 5 M-4, M-5)**

Change the `input` element (lines 40-58) to include the new attributes. Add after `r#type: "text",`:

```rust
                autocomplete: "off",
                aria_label: "Recovery key",
```

- [ ] **Step 3: Extract recovery logic into shared closure (Task 5 I-1)**

The `on_click` handler of the "Recover" button (lines 86-93) and the `onkeydown` handler (lines 49-57) contain duplicate logic. Extract it:

Replace the `onkeydown` handler (lines 49-57):
```rust
                onkeydown: move |e: KeyboardEvent| {
                    if e.code() == Code::Enter {
                        let passphrase = input_value_clone.read().clone();
                        if !passphrase.trim().is_empty() {
                            on_recover_clone.call(passphrase);
                            input_value_clone.set(String::new());
                        }
                    }
                },
```
Note: The "Recover" button handler (lines 86-93) uses `input_value` directly while the keydown uses `input_value_clone`. Both do the same thing. Refactor to use the `on_recover_clone` closure for both.

Replace the Recover button `on_click` handler (lines 86-93) to match the keydown pattern:
```rust
                    on_click: move |_| {
                        let passphrase = input_value_clone.read().clone();
                        if !passphrase.trim().is_empty() {
                            on_recover_clone.call(passphrase);
                            input_value_clone.set(String::new());
                        }
                    },
```

- [ ] **Step 4: Run `cargo check --features desktop`**

Run: `cargo check --features desktop`
Expected: Compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add src/components/globals/dialogs/recovery_key_input.rs
git commit -m "fix: make recovery dialog non-dismissable, add accessibility attrs, dedup logic"
```

---

### Task 9: Fix main.rs — all remaining issues

**Files:**
- Modify: `src/main.rs:1-403`

- [ ] **Step 1: Add import for `get_db_path` (Task 6 M-1)**

Add to imports (around line 20):
```rust
use backend::db_backend::get_db_path;
```

- [ ] **Step 2: Fix FirstSetup effect — add guard against overwrite (Task 6 I-2)**

Replace lines 166-172 from:
```rust
    // Effect: detect FirstSetup and show dialog
    use_effect(move || {
        let resource = db_resource.read();
        if let Some(Ok(InitResult::FirstSetup { recovery_phrase, .. })) = &*resource {
            setup_passphrase.set(recovery_phrase.expose_secret().to_string());
            show_setup_dialog.set(true);
        }
    });
```
to:
```rust
    // Effect: detect FirstSetup and show dialog
    use_effect(move || {
        let resource = db_resource.read();
        if let Some(Ok(InitResult::FirstSetup { recovery_phrase, .. })) = &*resource {
            if !show_setup_dialog.get() {
                setup_passphrase.set(recovery_phrase.expose_secret().to_string());
                show_setup_dialog.set(true);
            }
        }
    });
```

- [ ] **Step 3: Collapse Ready/FirstSetup match arms (Task 6 M-5)**

Replace lines 174-182 from:
```rust
    match &*db_resource.read() {
        Some(Ok(InitResult::Ready(pool))) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
        Some(Ok(InitResult::FirstSetup { pool, .. })) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
```
to:
```rust
    match &*db_resource.read() {
        Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
```

- [ ] **Step 4: Fix `handle_recover` — use `get_db_path()` and handle JoinError (Task 6 I-3, M-1)**

Replace lines 246-302 from:
```rust
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
            let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
                .unwrap()
                .pragma("key", pragma)
                .pragma("foreign_keys", "ON")
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);

            match sqlx::SqlitePool::connect_with(opts).await {
                Ok(_pool) => {
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
```
to:
```rust
    let handle_recover = move |passphrase: String| {
        let passphrase = passphrase.clone();
        spawn(async move {
            let db_path = match get_db_path() {
                Ok(p) => p,
                Err(_) => {
                    recovery_error.set(true);
                    return;
                }
            };

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
                Ok(Err(_)) => {
                    recovery_error.set(true);
                    return;
                }
                Err(join_err) => {
                    tracing::error!("Recovery derivation panicked: {}", join_err);
                    recovery_error.set(true);
                    return;
                }
            };

            // Try to open DB with derived key
            let pragma = format!("\"x'{}'\"", key);
            let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
                .unwrap()
                .pragma("key", pragma)
                .pragma("foreign_keys", "ON")
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);

            match sqlx::SqlitePool::connect_with(opts).await {
                Ok(_pool) => {
                    // Store key in keyring
                    let _ = crate::backend::db_key::store_db_key(
                        crate::backend::db_key::SERVICE_NAME,
                        crate::backend::db_key::KEY_USERNAME,
                        &key,
                    );
                    recovery_error.set(false);
                    show_recovery_dialog.set(false);
                    db_init_notified.set(false);
                    db_resource.restart();
                }
                Err(_) => {
                    recovery_error.set(true);
                }
            }
        });
    };
```

- [ ] **Step 5: Fix `handle_reset` in `render_recovery_ui` — use `get_db_path()` and handle result (Task 6 I-4, M-1)**

Replace lines 304-315 from:
```rust
    let handle_reset = move |_: ()| {
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
```
to:
```rust
    let handle_reset = move |_: ()| {
        let db_path = match get_db_path() {
            Ok(p) => p,
            Err(_) => return,
        };

        match crate::backend::db_key::reset_database(&db_path) {
            Ok(()) => {
                db_init_notified.set(false);
                db_resource.restart();
            }
            Err(e) => {
                show_toast_error(
                    format!("Failed to reset database: {}", e),
                    toast_state,
                );
            }
        }
    };
```

Note: This requires `toast_state` to be available in `render_recovery_ui`. Add it as a parameter.

Update `render_recovery_ui` signature (line 239) to accept `toast_state`:
```rust
fn render_recovery_ui(
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_recovery_dialog: Signal<bool>,
    mut recovery_error: Signal<bool>,
    mut show_reset_dialog: Signal<bool>,
    mut db_init_notified: Signal<bool>,
    mut toast_state: Signal<ToastHubState>,
) -> Element {
```

Update the call site (line 184-191) to pass `toast_state`:
```rust
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
```

- [ ] **Step 6: Fix `handle_reset` in `render_salt_error_ui` — same changes (Task 6 I-4, M-1)**

Update `render_salt_error_ui` signature (line 340) to accept `toast_state`:
```rust
fn render_salt_error_ui(
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_reset_dialog: Signal<bool>,
    error_msg: String,
    mut db_init_notified: Signal<bool>,
    mut toast_state: Signal<ToastHubState>,
) -> Element {
```

Replace its `handle_reset` (lines 346-357) with the same pattern as step 5.

Update the call site (line 192-194):
```rust
        Some(Err(custom_errors::DBError::DBSaltFileError(msg))) => {
            render_salt_error_ui(db_resource, show_reset_dialog, msg.clone(), db_init_notified, toast_state)
        }
```

- [ ] **Step 7: Run `cargo check --features desktop`**

Run: `cargo check --features desktop`
Expected: Compilation succeeds (this also resolves Task 4 I-1 — main.rs now compiles)

- [ ] **Step 8: Run all tests**

Run: `cargo test --features desktop`
Expected: All tests pass

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "fix: main.rs — guard FirstSetup, handle JoinError/reset errors, DRY db_path, collapse match"
```

---

### Task 10: Verify toast spam prevention (Task 6 I-1)

**Files:**
- Verify: `src/main.rs:99-126`

- [ ] **Step 1: Verify `db_init_notified` signal correctly prevents toast spam**

The existing code at lines 106-109 and 112-115 already guards with `if !db_init_notified()`. The DBKeyMissingWithDb arm at line 117-120 intentionally does NOT have this guard (the dialog should always re-open if init fails). This is correct behavior.

No code changes needed — this was a verification task.

- [ ] **Step 2: Run full test suite as final verification**

Run: `cargo test --features desktop`
Expected: All tests pass
