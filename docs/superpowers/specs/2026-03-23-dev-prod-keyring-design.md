# Dev/Prod Keyring Separation - Design Spec

## Overview

Separate the keyring and database setup logic between development and production environments. In production, the NSIS installer handles database initialization after the user accepts a privacy notice. In development, a fixed recovery passphrase is used with a separate keyring entry, keeping the flow identical to production for full testability.

## Problem

Currently, the DB encryption key is created at first app launch (FirstSetup flow). This is architecturally wrong: in production, the key should be created during installation, after the user accepts a privacy notice. The challenge is maintaining a functional development environment without an installer process.

## Design Decision: Fixed Passphrase + Separate Keyring Entry (Approach B)

In dev, a fixed diceware passphrase is used (hard-coded constant) with a separate keyring entry (`PWDManager-dev`). Salt is still generated randomly. Key derivation via Argon2id is identical to production. The recovery flow is fully testable because the passphrase is always known.

**Why not Approach A (same flow, different entry name only):** In dev, losing the keyring entry would require re-running FirstSetup with a new random passphrase, making testing cumbersome.

**Why not Approach C (fixed passphrase + fixed salt):** A fixed salt diverges from production behavior and makes DB reset semantics confusing.

## Architecture

### Keyring Entries

| Environment | Service Name | Username | Passphrase Source |
|---|---|---|---|
| Dev | `PWDManager-dev` | `db_encryption_key` | Fixed (`DEV_RECOVERY_PASSPHRASE`) |
| Prod | `PWDManager` | `db_encryption_key` | Random (diceware, system language) |

### Constants (`src/backend/db_key.rs`)

```rust
// Shared
pub const SERVICE_NAME: &str = "PWDManager";
pub const KEY_USERNAME: &str = "db_encryption_key";

// Dev-only (compile-time gated — not accessible in release builds)
#[cfg(debug_assertions)]
pub const DEV_SERVICE_NAME: &str = "PWDManager-dev";
#[cfg(debug_assertions)]
pub const DEV_RECOVERY_PASSPHRASE: &str = "CorrectHorseBatteryStaple";
```

> **Note on format:** `DEV_RECOVERY_PASSPHRASE` uses CamelCase to match the diceware format (`Word1Word2...`), but is a human-readable English phrase rather than random diceware words. This is intentional — it makes the passphrase memorable during development while keeping the format consistent.

### Helper Function

```rust
pub fn keyring_service_name() -> &'static str {
    if cfg!(debug_assertions) {
        DEV_SERVICE_NAME  // compile-time guaranteed to exist in debug
    } else {
        SERVICE_NAME
    }
}
```

All keyring operations (`store_db_key`, `retrieve_db_key`, `delete_db_key`) must use `keyring_service_name()` instead of `SERVICE_NAME` directly.

## Flows

### Complete Flow Matrix

| Scenario | Dev | Prod |
|---|---|---|
| DB+salt missing (first setup or re-init) | FirstSetup with fixed passphrase → RecoveryKeySetupDialog | FirstSetup with random passphrase → RecoveryKeySetupDialog (fallback if installer didn't run) |
| Recovery (keyring empty, DB exists) | RecoveryKeyInputDialog (fixed passphrase) | RecoveryKeyInputDialog (random passphrase) |
| Normal startup | Opens DB with key from `PWDManager-dev` | Opens DB with key from `PWDManager` |
| Installer setup | N/A | NSIS runs `--setup` → creates key, salt, DB, shows passphrase |

### Dev FirstSetup Flow

Identical to current flow except:
1. Passphrase is read from `DEV_RECOVERY_PASSPHRASE` constant instead of being generated via diceware
2. Keyring entry uses `DEV_SERVICE_NAME` (`PWDManager-dev`)
3. Salt is still randomly generated
4. Key derivation via Argon2id is identical
5. RecoveryKeySetupDialog is shown (user sees the fixed passphrase)

### Prod FirstSetup Flow (Re-initialization)

Reached only if DB+salt are missing (e.g., user deleted files). Same as current flow:
1. Random diceware passphrase (language detection)
2. Random salt
3. Argon2id derivation
4. Keyring entry uses `SERVICE_NAME` (`PWDManager`)
5. RecoveryKeySetupDialog shown with generated passphrase

### `--setup` CLI Command (Production Installer)

A new headless CLI mode invoked by the NSIS installer. No Dioxus GUI is launched.

```
pwdmanager.exe --setup
```

**Exit codes:** `0` = success (passphrase printed on stdout), `1` = failure (error on stderr)

**No other arguments are expected or supported.**

**Behavior:**
1. Generate random diceware passphrase (language detection)
2. Generate random salt (16 bytes), write to `{db_path}.salt`
3. Derive key via Argon2id (spawn_blocking)
4. Store key in keyring under `SERVICE_NAME` (`PWDManager`)
5. Create encrypted DB with all tables
6. Print passphrase to stdout (NSIS captures this)
7. Exit with code 0

**Important:** This command always uses production keyring (`SERVICE_NAME`), regardless of build configuration. It is only invoked from the release installer. To prevent accidental invocation in debug builds (which would create a prod keyring entry), a runtime guard should be added:

```rust
if args.contains(&"--setup".to_string()) {
    if cfg!(debug_assertions) {
        eprintln!("Error: --setup is not available in debug builds");
        std::process::exit(1);
    }
    // ... proceed with setup
}
```

**Tokio runtime:** Since `--setup` exits before `launch_desktop!` (which provides the Dioxus/tokio runtime), `run_setup()` must create its own tokio runtime. Use a current-thread runtime:

```rust
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;
let result = rt.block_on(run_setup());
```

### Recovery Flow (Both Environments)

Unchanged from current implementation. The only difference is the keyring service name:
- Dev: reads from `PWDManager-dev`
- Prod: reads from `PWDManager`

Derivation logic is identical: passphrase + salt → Argon2id → hex key.

### Database Reset

`reset_database()` must use `keyring_service_name()` to delete the correct keyring entry.

## Code Changes

### `src/main.rs`

Add CLI argument parsing before `launch_desktop!`. The `--setup` branch must create its own tokio runtime since `launch_desktop!` is not yet invoked:

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--setup".to_string()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        match rt.block_on(run_setup()) {
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

    launch_desktop!(App, APP_VERSION);
}
```

### `src/main.rs` — Recovery Flow

Update `store_db_key` calls in the recovery UI handler (around line 295-298) to use `keyring_service_name()`:

```rust
// Before:
store_db_key(crate::backend::db_key::SERVICE_NAME, ...)

// After:
store_db_key(keyring_service_name(), ...)
```

### `src/backend/db_key.rs`

1. Add dev constants (`DEV_SERVICE_NAME`, `DEV_RECOVERY_PASSPHRASE`) with `#[cfg(debug_assertions)]`
2. Add `keyring_service_name()` helper
3. Replace all direct `SERVICE_NAME` usage with `keyring_service_name()`
4. **Modify `generate_and_store_key()` to accept a `service_name: &str` parameter** instead of hardcoding `SERVICE_NAME`. Currently the function internally calls `store_db_key(SERVICE_NAME, KEY_USERNAME, &key)` — this must become `store_db_key(service_name, KEY_USERNAME, &key)`. All callers must pass `keyring_service_name()` (except `run_setup()` which passes `SERVICE_NAME` explicitly).
5. Extract shared setup logic into `perform_setup()` function (used by both `init_db()` dev/prod branch and `run_setup()`)

### `src/backend/db_backend.rs`

1. In `init_db()` FirstSetup branch: use `DEV_RECOVERY_PASSPHRASE` in dev, generate random passphrase in prod
2. All keyring operations use `keyring_service_name()`
3. Extract `run_init_queries(pool)` helper from the existing `for init_query in QUERIES` loop (lines 153-158)
4. The `perform_setup()` logic (generate passphrase, salt, derive key, create DB) is extracted and shared

### `run_setup()` Function

New function (in `db_key.rs` or a new module) that encapsulates the setup logic:

```rust
pub async fn run_setup() -> Result<SecretString, DBError> {
    // 1. Generate diceware passphrase (random, system language)
    let passphrase = generate_recovery_passphrase()?;

    // 2. Derive and store key (spawn_blocking)
    let db_path = get_db_path();
    let key = tokio::task::spawn_blocking({
        let passphrase = passphrase.clone();
        move || {
            // Always uses production keyring for --setup
            generate_and_store_key(&passphrase, SERVICE_NAME, &db_path)
        }
    }).await.map_err(|e| DBError::new_general_error(e.to_string()))??;

    // 3. Create DB with tables
    let options = build_sqlcipher_options(&db_path, &key)?;
    let pool = SqlitePool::connect_with(options.create_if_missing(true)).await?;
    run_init_queries(&pool).await?;

    Ok(SecretString::new(passphrase.into()))
}
```

> **Note:** `generate_and_store_key()` takes `&str`, not `&SecretString`. The passphrase string is cloned into the `spawn_blocking` closure and exposed. This is acceptable because `spawn_blocking` runs synchronously on a separate thread and the string is not logged or persisted.

## NSIS Installer Modifications

### Configuration (`Dioxus.toml`)

Add `installer_hooks` to the `[bundle.windows.nsis]` section:

```toml
[bundle.windows]
icon_path = "icons/icon.ico"
digest_algorithm = "sha256"
# IMPORTANT: preserve existing [webview_install_mode.EmbedBootstrapper] section
[webview_install_mode.EmbedBootstrapper]
silent = true

[bundle.windows.nsis]
installer_hooks = "installer/nsis-hooks.nsh"
```

### Hook Timing: Two-Phase Approach

The NSIS hook must use a **two-phase** approach because `--setup` requires the exe to be on disk:

1. **PRE_INSTALL hook** (`NSIS_HOOK_PRE_INST`): Show the privacy notice / acceptance page. If the user declines, abort.
2. **POST_INSTALL hook** (`NSIS_HOOK_POST_INST`): After files are extracted to `$INSTDIR`, invoke `pwdmanager.exe --setup`, capture stdout, show the passphrase page.

> **Note:** The exact macro names (`NSIS_HOOK_PRE_INST`, `NSIS_HOOK_POST_INST`) must be verified against the Dioxus bundler source (`dioxus-packager` crate) before implementation. Tauri uses `PRE_INST`/`POST_INST` naming — Dioxus may differ.

### Hook Script (`installer/nsis-hooks.nsh`)

Custom NSIS script:

```
; installer/nsis-hooks.nsh (pseudocode structure)

!macro NSIS_HOOK_PRE_INST
    ; Show privacy notice page
    ; If user declines: Abort
!macroend

!macro NSIS_HOOK_POST_INST
    ; Run --setup and capture stdout into variable
    ;   nsExec::ExecToStack '"$INSTDIR\pwdmanager.exe" --setup'
    ;   Pop $0  ; exit code
    ;   Pop $1  ; stdout output (the passphrase)
    ;
    ; If $0 != 0: Abort "Setup failed"
    ; Show passphrase display page (from $1)
    ; Wait for user confirmation ("I have saved the recovery key")
!macroend
```

> **Important:** `nsExec::ExecToStack` (not `ExecToLog`) is used to capture stdout. `ExecToStack` pushes both the exit code and stdout onto the NSIS stack. `ExecToLog` sends output to the log window only and cannot be captured into a variable.

### Placeholder Pages

For the initial implementation:
- **Privacy notice page**: Static text placeholder ("Informativa sulla privacy - Placeholder")
- **Passphrase display page**: Shows the passphrase in a read-only field with warning text

## Security Considerations

- The fixed dev passphrase (`DEV_RECOVERY_PASSPHRASE`) is compiled into the debug binary. This is acceptable because debug builds are never distributed and the dev DB contains no real data.
- The `--setup` command prints the passphrase to stdout. The NSIS installer captures this internally via `ExecToStack` — it is never written to disk or logged.
- The recovery passphrase is only shown once: during setup (installer or FirstSetup dialog). If lost, the DB must be reset.
