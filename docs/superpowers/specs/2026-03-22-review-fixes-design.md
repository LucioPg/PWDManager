# Review Fixes Design — db_key, init_db, Dialog, main.rs

**Date:** 2026-03-22
**Scope:** Fix all 21 issues from code review (`docs/22-03-2026-issues.md`)

## 1. Error Types (Task 2-3)

### New DBKeyError variants

```rust
pub enum DBKeyError {
    NoEntry,
    KeyringError(String),
    DerivationError(String),        // NEW — Argon2/diceware non-keyring errors
    DatabaseCleanupError(String),   // NEW — DB file deletion errors
    MissingKeyWithDb,
    RecoveryKeyInvalid,
    SaltFileError(String),
}
```

### Error mapping changes

| Function | Before | After |
|----------|--------|-------|
| `derive_key` | `KeyringError` | `DerivationError` |
| `generate_recovery_passphrase` | `KeyringError` | `DerivationError` |
| `reset_database` (DB file errors) | `SaltFileError` | `DatabaseCleanupError` |

No changes to the external `custom_errors` crate are needed. The new `DBKeyError` variants will be mapped to `DBError::DBGeneralError(msg)` via the existing `.map_err` in `init_db`.

### `read_salt` return type change

Return `[u8; 16]` instead of `Vec<u8>`. Implementation strategy: collect into `Vec<u8>` first (current code), then convert with `try_into()`:

```rust
let bytes: Vec<u8> = (0..16).map(|i| hex::decode(...)).collect::<Result<Vec<u8>, _>>()?;
let arr: [u8; 16] = bytes.try_into()
    .map_err(|_| DBKeyError::SaltFileError("Salt must be exactly 16 bytes".into()))?;
```

This change also benefits `derive_key_from_passphrase` which calls `read_salt` — no separate change needed there.

### `generate_and_store_key` doc

Add `/// CPU-bound: call via spawn_blocking` to doc comment.

### `get_or_create_db_key` deprecation

Replace `@deprecated` in doc comment with `#[deprecated]` attribute.

## 2. init_db (Task 4)

### Compilation error (I-1)

`main.rs` does not compile in the current worktree state. This is resolved as a natural consequence of the Task 6 fixes (match arm collapsing, variable renaming, new helper function usage).

### Salt file orphan cleanup (I-2)

The cleanup happens **inside `generate_and_store_key` in `db_key.rs`** (synchronous, within `spawn_blocking`), not in `init_db`. After `write_salt` succeeds, if `derive_key` or `store_db_key` fails, remove the orphaned salt file:

```rust
// Inside generate_and_store_key (sync function in db_key.rs)
let salt = generate_db_salt();
write_salt(db_path, &salt)?;

let result = derive_key(passphrase, &salt)
    .and_then(|key| { store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?; Ok(key) });

match result {
    Ok(key) => Ok(key),
    Err(e) => {
        // Cleanup orphaned salt file on failure
        let salt_path = salt_file_path(db_path);
        if let Err(cleanup_err) = std::fs::remove_file(&salt_path) {
            tracing::warn!("Failed to clean up orphaned salt file {}: {}", salt_path.display(), cleanup_err);
        }
        Err(e)
    }
}
```

If the salt file removal itself fails, the error is logged but the **original derivation error** is still returned to the caller.

### Connect error disambiguation (I-3)

Split the catch-all `Err(_)` arm into two branches:

```rust
match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
    Ok(key) => {
        match connect_with_key(&db_path, &key).await {
            Ok(pool) => Ok(InitResult::Ready(pool)),
            Err(_) => Err(DBError::new_key_missing_with_db()), // Key present but DB won't open
        }
    }
    Err(DBKeyError::NoEntry) => Err(DBError::new_key_missing_with_db()),
    Err(e) => Err(DBError::new_general_error(&format!("Keyring error: {}", e))),
}
```

### Variable shadowing (M-3)

Rename local variable `db_key` to `db_key_value` in `init_db`.

## 3. Dialog Components (Task 5)

### RecoveryKeyInputDialog non-dismissable (I-1, I-2)

Remove the ability to dismiss via X button or backdrop click. The dialog should only close through explicit user actions (Recover or Reset). Replace the `on_close` handler with a no-op `move |_| {}` (since `BaseModal` is from the external `pwd-dioxus` library, we don't control its `dismissible` prop directly).

### Input accessibility (M-4, M-5)

Add `autocomplete: "off"` and `aria_label: "Recovery key"` to the recovery key input.

### Recovery logic dedup (I-1)

Extract recovery logic into a shared closure within the component.

## 4. main.rs Integration (Task 6)

### Toast spam prevention (I-1)

Verify `db_init_notified` signal correctly prevents repeated toast triggers. Add explicit dependency tracking in `use_effect` if needed.

### FirstSetup overwrite guard (I-2)

Add guard `if !show_setup_dialog.get()` before setting `setup_passphrase` in the FirstSetup effect.

### JoinError handling (I-3)

In `handle_recover`, distinguish between `JoinError` (panic in spawn_blocking) and passphrase derivation failure:

- `JoinError` -> log with `tracing::error!`, show generic "An error occurred" to user
- Derivation failure -> show "Invalid recovery key" to user

### reset_database error handling (I-4)

Handle the `Result` of `reset_database`: on failure, show error toast to user.

### DRY: db_path helper (M-1)

Extract `fn get_db_path() -> PathBuf` in `db_backend.rs` (returns `PathBuf`). All three call sites (`init_db`, `handle_recover`, `handle_reset`) call `.to_str().unwrap().to_string()` on the result as needed. This also deduplicates the reset handler between `render_recovery_ui` and `render_salt_error_ui`.

### Match arm collapsing (M-5)

Collapse `Ready(pool)` and `FirstSetup { pool, .. }` into a single arm using `|`.

## 5. Test Fixes

### Task 2 M-1

Use `tempfile::tempdir()` instead of fixed path `pwd_test_salt`.

### Task 3 I-2

Add comment explaining WAL/SHM files are intentionally ignored.

### Task 3 I-3

The test calls `cleanup()` on `TEST_SERVICE`/`TEST_USER`, but `generate_and_store_key` writes to `SERVICE_NAME`/`KEY_USERNAME` (real keyring). Fix: add `delete_db_key(SERVICE_NAME, KEY_USERNAME)` to the test's cleanup to properly remove the real keyring entry written by the test.
