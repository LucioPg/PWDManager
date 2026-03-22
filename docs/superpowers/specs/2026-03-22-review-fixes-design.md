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

### `read_salt` return type change

Return `[u8; 16]` instead of `Vec<u8>`. Callers no longer need conversion.

### `generate_and_store_key` doc

Add `/// CPU-bound: call via spawn_blocking` to doc comment.

### `get_or_create_db_key` deprecation

Replace `@deprecated` in doc comment with `#[deprecated]` attribute.

## 2. init_db (Task 4)

### Salt file orphan cleanup (I-2)

In `generate_and_store_key`, if key derivation fails after `write_salt`, attempt to remove the orphaned salt file. If removal also fails, log with `tracing::warn!` and include in error message.

### Connect error disambiguation (I-3)

When `SqlitePool::connect_with` fails in normal startup:

- Keyring returned a key but DB won't open -> `DBKeyMissingWithDb` (wrong key)
- Keyring itself failed (not `NoEntry`) -> `DBGeneralError` with original error
- Keyring returned `NoEntry` -> `DBKeyMissingWithDb`

### Variable shadowing (M-3)

Rename local variable `db_key` to `db_key_value` in `init_db`.

## 3. Dialog Components (Task 5)

### RecoveryKeyInputDialog non-dismissable (I-1, I-2)

Remove the ability to dismiss via X button or backdrop click. The dialog should only close through explicit user actions (Recover or Reset).

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

Extract `fn get_db_path() -> PathBuf` in `db_backend.rs`, used by `init_db` and exposed for `main.rs`.

### Match arm collapsing (M-5)

Collapse `Ready(pool)` and `FirstSetup { pool, .. }` into a single arm using `|`.

## 5. Test Fixes

### Task 2 M-1

Use `tempfile::tempdir()` instead of fixed path `pwd_test_salt`.

### Task 3 I-2

Add comment explaining WAL/SHM files are intentionally ignored.

### Task 3 I-3

Fix test to use `SERVICE_NAME` constant instead of `TEST_SERVICE`.
