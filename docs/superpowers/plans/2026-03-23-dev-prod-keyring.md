# Dev/Prod Keyring Separation - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Separate keyring entries between dev and prod environments, add `--setup` CLI command for NSIS installer integration.

**Architecture:** A `keyring_service_name()` helper returns the correct OS keyring entry based on `cfg!(debug_assertions)`. In dev, a fixed passphrase is used with `PWDManager-dev` keyring entry. A `--setup` CLI headless mode creates the prod DB for the NSIS installer. All keyring operations route through the helper.

**Tech Stack:** Rust, Dioxus 0.7, sqlx, keyring crate v3, NSIS (via Dioxus `dx bundle`), Argon2id

**Spec:** `docs/superpowers/specs/2026-03-23-dev-prod-keyring-design.md`

---

### Task 1: Add dev constants and `keyring_service_name()` helper

**Files:**
- Modify: `src/backend/db_key.rs:8-12`

- [ ] **Step 1: Add dev constants after existing `KEY_USERNAME` (line 12)**

After `pub const KEY_USERNAME: &str = "db_encryption_key";` add:

```rust
/// Dev-only keyring service name — separate from prod to avoid conflicts.
#[cfg(debug_assertions)]
pub const DEV_SERVICE_NAME: &str = "PWDManager-dev";

/// Dev-only fixed recovery passphrase (hard-coded, CamelCase to match diceware format).
/// Acceptable because debug builds are never distributed and dev DB contains no real data.
#[cfg(debug_assertions)]
pub const DEV_RECOVERY_PASSPHRASE: &str = "CorrectHorseBatteryStaple";
```

- [ ] **Step 2: Add `keyring_service_name()` helper after the constants**

```rust
/// Returns the correct keyring service name based on build configuration.
/// Dev builds use a separate keyring entry to avoid conflicts with prod.
pub fn keyring_service_name() -> &'static str {
    if cfg!(debug_assertions) {
        DEV_SERVICE_NAME
    } else {
        SERVICE_NAME
    }
}
```

- [ ] **Step 3: Write test for `keyring_service_name()`**

Add to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_keyring_service_name_returns_string() {
    let name = keyring_service_name();
    assert!(!name.is_empty());
    assert!(name.contains("PWDManager"));
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p pwd-dioxus --lib backend::db_key::tests -- --nocapture`
Expected: All tests pass (including new test)

- [ ] **Step 5: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "feat: add dev constants and keyring_service_name() helper"
```

---

### Task 2: Modify `generate_and_store_key()` to accept `service_name` parameter

**Files:**
- Modify: `src/backend/db_key.rs:166-190`
- Modify: `src/backend/db_key.rs:372-390` (test `test_generate_and_store_key`)

- [ ] **Step 1: Update function signature (line 166)**

Change:
```rust
pub fn generate_and_store_key(
    passphrase: &str,
    db_path: &str,
) -> Result<String, DBKeyError> {
```

To:
```rust
pub fn generate_and_store_key(
    passphrase: &str,
    service_name: &str,
    db_path: &str,
) -> Result<String, DBKeyError> {
```

- [ ] **Step 2: Update internal `store_db_key` call (line 175)**

Change:
```rust
store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
```

To:
```rust
store_db_key(service_name, KEY_USERNAME, &key)?;
```

- [ ] **Step 3: Update caller in `init_db()` (`src/backend/db_backend.rs:140`)**

Change:
```rust
move || db_key::generate_and_store_key(&passphrase, &db_path)
```

To:
```rust
move || db_key::generate_and_store_key(&passphrase, db_key::keyring_service_name(), &db_path)
```

- [ ] **Step 4: Update test `test_generate_and_store_key` (line 372)**

Change:
```rust
let key = generate_and_store_key("MyTestPassphrase123", &db_path).unwrap();
```

To:
```rust
let key = generate_and_store_key("MyTestPassphrase123", TEST_SERVICE, &db_path).unwrap();
```

And change the cleanup at the end of the test (line 389):
```rust
// Cleanup: the test now uses TEST_SERVICE, not SERVICE_NAME
delete_db_key(TEST_SERVICE, KEY_USERNAME);
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p pwd-dioxus --lib backend::db_key::tests -- --nocapture`
Expected: All tests pass

- [ ] **Step 6: Build check**

Run: `cargo check -p pwd-dioxus`
Expected: No compilation errors (all callers updated)

- [ ] **Step 7: Commit**

```bash
git add src/backend/db_key.rs src/backend/db_backend.rs
git commit -m "refactor: generate_and_store_key accepts service_name parameter"
```

---

### Task 3: Update `reset_database()` and `get_or_create_db_key()` to use `keyring_service_name()`

**Files:**
- Modify: `src/backend/db_key.rs:218` (`reset_database`)
- Modify: `src/backend/db_key.rs:234` (`get_or_create_db_key`)

- [ ] **Step 1: Update `reset_database()` (line 218)**

Change:
```rust
delete_db_key(SERVICE_NAME, KEY_USERNAME);
```

To:
```rust
delete_db_key(keyring_service_name(), KEY_USERNAME);
```

- [ ] **Step 2: Update `get_or_create_db_key()` (line 234)**

Change:
```rust
match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
```

To:
```rust
match retrieve_db_key(keyring_service_name(), KEY_USERNAME) {
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p pwd-dioxus --lib backend::db_key::tests -- --nocapture`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/backend/db_key.rs
git commit -m "refactor: use keyring_service_name() in reset_database and get_or_create_db_key"
```

---

### Task 4: Update `init_db()` FirstSetup for dev passphrase

**Files:**
- Modify: `src/backend/db_backend.rs:128-164`
- Modify: `src/backend/db_backend.rs:174`

- [ ] **Step 1: Replace passphrase generation in FirstSetup branch (lines 132-133)**

Change:
```rust
let passphrase = db_key::generate_recovery_passphrase()
    .map_err(|e| DBError::new_general_error(format!("Passphrase generation: {}", e)))?;
```

To:
```rust
let passphrase = if cfg!(debug_assertions) {
    db_key::DEV_RECOVERY_PASSPHRASE.to_string()
} else {
    db_key::generate_recovery_passphrase()
        .map_err(|e| DBError::new_general_error(format!("Passphrase generation: {}", e)))?
};
```

- [ ] **Step 2: Update keyring retrieval in normal startup (line 174)**

Change:
```rust
let keyring_result = db_key::retrieve_db_key(db_key::SERVICE_NAME, db_key::KEY_USERNAME);
```

To:
```rust
let keyring_result = db_key::retrieve_db_key(db_key::keyring_service_name(), db_key::KEY_USERNAME);
```

- [ ] **Step 3: Build check**

Run: `cargo check -p pwd-dioxus`
Expected: No compilation errors

- [ ] **Step 4: Manual test (dev)**

Run the app in debug mode:
1. Delete `database.db` and `database.db.salt` if they exist
2. Run `cargo run`
3. Verify: RecoveryKeySetupDialog appears with passphrase "CorrectHorseBatteryStaple"
4. Verify: Windows Credential Manager has entry `PWDManager-dev` (not `PWDManager`)
5. Close and restart — app should open normally (key from keyring)

- [ ] **Step 5: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: use fixed dev passphrase and keyring_service_name in init_db"
```

---

### Task 5: Update `main.rs` recovery flow to use `keyring_service_name()`

**Files:**
- Modify: `src/main.rs:295-298`

- [ ] **Step 1: Update `store_db_key` call in `handle_recover` (line 295)**

Change:
```rust
let _ = crate::backend::db_key::store_db_key(
    crate::backend::db_key::SERVICE_NAME,
    crate::backend::db_key::KEY_USERNAME,
    &key,
);
```

To:
```rust
let _ = crate::backend::db_key::store_db_key(
    crate::backend::db_key::keyring_service_name(),
    crate::backend::db_key::KEY_USERNAME,
    &key,
);
```

- [ ] **Step 2: Build check**

Run: `cargo check -p pwd-dioxus`
Expected: No compilation errors

- [ ] **Step 3: Manual test (dev recovery)**

1. Run the app in debug mode (with existing dev DB)
2. Open Windows Credential Manager → delete the `PWDManager-dev` entry
3. Restart the app
4. Verify: RecoveryKeyInputDialog appears
5. Enter "CorrectHorseBatteryStaple"
6. Verify: app opens normally, key is re-stored in keyring

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "fix: use keyring_service_name in recovery flow"
```

---

### Task 6: Add `--setup` CLI command

**Files:**
- Modify: `src/main.rs:399-404`
- Create: `src/backend/setup.rs`

- [ ] **Step 1: Create `src/backend/setup.rs`**

```rust
//! Headless setup for NSIS installer integration.
//! Creates encrypted DB with random diceware passphrase, stores key in prod keyring.

use crate::backend::db_backend::{build_sqlcipher_options, get_db_path};
use crate::backend::db_key;
use crate::backend::init_queries::QUERIES;
use custom_errors::DBError;
use secrecy::SecretString;
use sqlx::query;
use sqlx::sqlite::SqlitePool;

/// Runs headless database setup (for NSIS installer).
/// Always uses production keyring (`SERVICE_NAME`), never dev keyring.
/// Returns the generated recovery passphrase.
pub async fn run_setup() -> Result<SecretString, DBError> {
    // 1. Generate random diceware passphrase
    let passphrase = db_key::generate_recovery_passphrase()
        .map_err(|e| DBError::new_general_error(format!("Passphrase generation: {}", e)))?;

    let db_path = get_db_path()?;

    // 2. Derive key and store in prod keyring
    let key = tokio::task::spawn_blocking({
        let passphrase = passphrase.clone();
        let db_path = db_path.clone();
        move || db_key::generate_and_store_key(&passphrase, db_key::SERVICE_NAME, &db_path)
    })
    .await
    .map_err(|e| DBError::new_general_error(format!("Key derivation task failed: {}", e)))?
    .map_err(|e| DBError::new_general_error(format!("Key setup failed: {}", e)))?;

    // 3. Create encrypted DB with tables
    let connect_options = build_sqlcipher_options(&db_path, &key)?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(connect_options)
        .await
        .map_err(|e| DBError::new_general_error(format!("Failed to create database: {}", e)))?;

    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    }

    Ok(SecretString::new(passphrase.into()))
}
```

- [ ] **Step 2: Add module declaration**

In `src/backend/mod.rs`, add:
```rust
#[cfg(feature = "desktop")]
pub mod setup;
```

- [ ] **Step 3: Update `main()` function (line 399)**

Replace:
```rust
fn main() {
    // Nota: il logging viene inizializzato automaticamente nel launcher
    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("PWDManager v{}", APP_VERSION);
    launch_desktop!(App, APP_VERSION);
}
```

With:
```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--setup".to_string()) {
        if cfg!(debug_assertions) {
            eprintln!("Error: --setup is not available in debug builds");
            std::process::exit(1);
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        match rt.block_on(crate::backend::setup::run_setup()) {
            Ok(passphrase) => {
                println!("{}", passphrase.expose_secret());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Setup failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("PWDManager v{}", APP_VERSION);
    launch_desktop!(App, APP_VERSION);
}
```

- [ ] **Step 4: Build check (release)**

Run: `cargo check -p pwd-dioxus`
Expected: No compilation errors

- [ ] **Step 5: Commit**

```bash
git add src/backend/setup.rs src/backend/mod.rs src/main.rs
git commit -m "feat: add --setup CLI command for NSIS installer"
```

---

### Task 7: Create NSIS installer hooks

**Files:**
- Create: `installer/nsis-hooks.nsh`
- Modify: `Dioxus.toml:17-24`

- [ ] **Step 1: Create `installer/nsis-hooks.nsh`**

Create directory `installer/` and file `nsis-hooks.nsh`:

```nsis
; PWDManager NSIS Installer Hooks
; Two-phase approach: privacy notice before install, DB setup after install.

!macro NSIS_HOOK_PRE_INST
    ; TODO: Show privacy notice / acceptance page
    ; If user declines: Abort
    !insertmacro MUI_HEADER_TEXT "Privacy Notice" "Please read and accept"
!macroend

!macro NSIS_HOOK_POST_INST
    ; Run --setup and capture stdout (passphrase)
    nsExec::ExecToStack '"$INSTDIR\pwdmanager.exe" --setup'
    Pop $0  ; exit code
    Pop $1  ; stdout (recovery passphrase)

    ${If} $0 != 0
        MessageBox MB_ICONSTOP "Database setup failed. Installation cannot continue.$\n$\nExit code: $0" /SD IDOK
        Abort "Database setup failed"
    ${EndIf}

    ; TODO: Show passphrase display page with $1
    ; User must click "I have saved the recovery key" to continue
!macroend
```

> **Note:** The exact macro names (`NSIS_HOOK_PRE_INST`, `NSIS_HOOK_POST_INST`) must be verified against the Dioxus bundler source (`dioxus-packager` crate) before actual testing. The placeholder pages are sufficient for the initial implementation.

- [ ] **Step 2: Update `Dioxus.toml`**

Add `[bundle.windows.nsis]` section while preserving existing `webview_install_mode`:

```toml
[bundle.windows]
icon_path = "icons/icon.ico"
digest_algorithm = "sha256"
[webview_install_mode.EmbedBootstrapper]
silent = true

[bundle.windows.nsis]
installer_hooks = "installer/nsis-hooks.nsh"
```

- [ ] **Step 3: Commit**

```bash
git add installer/nsis-hooks.nsh Dioxus.toml
git commit -m "feat: add NSIS installer hooks for --setup integration"
```

---

### Task 8: Final integration test and cleanup

**Files:**
- All modified files

- [ ] **Step 1: Run full test suite**

Run: `cargo test -p pwd-dioxus`
Expected: All tests pass

- [ ] **Step 2: Verify dev mode manually**

1. Delete `database.db`, `database.db.salt` if they exist
2. Open Windows Credential Manager → verify no `PWDManager-dev` or `PWDManager` entries
3. Run `cargo run`
4. Verify: RecoveryKeySetupDialog shows "CorrectHorseBatteryStaple"
5. Verify: Windows Credential Manager has `PWDManager-dev` entry
6. Close and restart → app opens normally
7. Test recovery: delete `PWDManager-dev` from Credential Manager, restart, enter passphrase → works

- [ ] **Step 3: Verify `--setup` guard in debug**

Run: `cargo run -- --setup 2>&1`
Expected: "Error: --setup is not available in debug builds" and exit code 1

- [ ] **Step 4: Grep for remaining direct `SERVICE_NAME` usage**

Run: `grep -rn "SERVICE_NAME" src/ --include="*.rs"`
Expected: Only in `db_key.rs` constant definition and in `setup.rs` (which intentionally uses `SERVICE_NAME` for prod)

- [ ] **Step 5: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: final cleanup and integration verification"
```
