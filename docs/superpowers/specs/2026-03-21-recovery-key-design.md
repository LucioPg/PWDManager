# Recovery Key - Design Spec

## Overview

Add a recovery mechanism for the encrypted database. Currently, the encryption key is random and stored only in the OS keyring. If the keyring entry is lost or modified, the database becomes permanently inaccessible. This spec introduces a diceware-based recovery passphrase as the source of truth, with the keyring serving as a convenience cache.

## Design Decision: Diceware + Argon2id (Approach A)

The recovery passphrase is a 6-word diceware phrase in CamelCase. The actual encryption key is derived from the passphrase via Argon2id with a salt stored in a file alongside the database. This approach uses existing project dependencies (`diceware`, `argon2`) with no new crates.

**Why not BIP-39:** BIP-39 provides deterministic encoding of bytes to words and back, but requires a new crate. Diceware is already in the project, and Argon2id provides stronger key derivation than BIP-39's checksum-based approach.

**Why not raw diceware as key (no KDF):** SQLCipher's built-in passphrase mode uses PBKDF2, which is weaker than Argon2id against brute-force attacks.

**Why not deterministic diceware encoding:** The `diceware` crate uses `OsRng` internally and has no seeded/deterministic mode. The passphrase is generated once and shown to the user; determinism is provided by Argon2 (same passphrase + same salt = same key).

## Cryptographic Flow

### Key Derivation

- **KDF:** Argon2id (variant `Argon2::default()`)
- **Salt:** 16 random bytes, stored in `database.db.salt`
- **Output:** 32 bytes (256 bits), hex-encoded to 64 characters
- **Usage:** Raw key via `PRAGMA key = "x'...'"` (same as current)
- **Parameters:** `m=19456, t=2, p=1` (Argon2id defaults)

### Salt File Format

Path: `{db_path}.salt` (e.g., `database.db.salt`)
Content: 32-character hex string (16 bytes), not secret.

```
a3f1b2c4d5e6f789012345678abcdef0
```

### Recovery Passphrase

- **Format:** 6 diceware words, CamelCase (e.g., `Word1Word2Word3Word4Word5Word6`)
- **Language:** System language via `sys-locale`, defaults to EN
- **Generation:** `diceware::make_passphrase()` with `Config::new().with_embedded(lang).with_words(6).with_camel_case(true)`

### First Setup Flow

```
1. Generate salt (16 random bytes) → save to database.db.salt
2. Generate diceware passphrase (6 words, system language, CamelCase)
3. Derive key: Argon2id(passphrase, salt) → [u8; 32] → hex 64 char
4. Store hex key in keyring (cache for daily use)
5. Open new DB with key, create tables
6. Return InitResult::FirstSetup { pool, recovery_phrase }
```

### Normal Startup Flow

```
1. Read salt from database.db.salt
2. Retrieve key from keyring
3. Try to open DB with keyring key
4. Success → return InitResult::Ready(pool)
5. Failure → return Err(DBError::DBKeyMissingWithDb)
```

### Recovery Flow (keyring lost or wrong)

```
1. Read salt from database.db.salt
2. UI shows RecoveryKeyInputDialog
3. User enters passphrase
4. Derive key: Argon2id(passphrase, salt)
5. Try to open DB with derived key
6. Success → store key in keyring → return InitResult::Ready(pool)
7. Failure → return Err(DBKeyError::RecoveryKeyInvalid) → retry (infinite)
```

### Regeneration Flow (from Settings)

```
1. User clicks "Rigenera recovery key" in settings
2. Confirmation dialog
3. Generate new salt + new diceware passphrase
4. Re-encrypt DB with new derived key (ATTACH + sqlcipher_export + replace)
5. Update keyring with new derived key
6. Update database.db.salt with new salt
7. Show RecoveryKeyRegeneratedDialog with new passphrase
```

### Database Reset Flow

```
1. User clicks "Ripristina database" from RecoveryKeyInputDialog
2. Confirmation dialog (DatabaseResetDialog)
3. Delete database.db + database.db.salt
4. Restart init_db() → triggers First Setup flow
```

## Data Types

### InitResult (new)

```rust
pub enum InitResult {
    Ready(SqlitePool),
    FirstSetup { pool: SqlitePool, recovery_phrase: String },
}
```

### DBKeyError (modified)

```rust
pub enum DBKeyError {
    NoEntry,                        // Credential not in keyring
    KeyringError(String),           // System keyring error
    MissingKeyWithDb,               // DB exists but open fails → recovery needed
    RecoveryKeyInvalid,             // Passphrase does not open the DB
    SaltFileError(String),          // Error reading/writing salt file
}
```

`MissingKeyWithDb` covers both "keyring empty" and "keyring has wrong key" — both cause `SqlitePool::connect_with()` to fail.

### DBError (new variants, in custom_errors crate)

```rust
// New variants added to DBError:
DBKeyMissingWithDb,             // Maps from DBKeyError::MissingKeyWithDb
DBRecoveryKeyInvalid,           // Maps from DBKeyError::RecoveryKeyInvalid
DBSaltFileError(String),        // Maps from DBKeyError::SaltFileError
```

## Error Mapping Chain

```
db_key.rs                  db_backend.rs                main.rs (UI)
─────────                  ─────────────                ──────────
MissingKeyWithDb ───────→ DBError::DBKeyMissingWithDb ──match──→ RecoveryKeyInputDialog
RecoveryKeyInvalid ─────→ DBError::DBRecoveryKeyInvalid─match──→ Error + retry in dialog
SaltFileError(msg) ─────→ DBError::DBSaltFileError(msg)─match──→ Generic error + retry
```

## UI Components

All dialogs follow the existing `BaseModal` + `ActionButton` pattern.

### 1. RecoveryKeySetupDialog

Shown on first setup. Displays the generated passphrase.

**Props:** `open: Signal<bool>`, `passphrase: String`, `on_confirm: EventHandler<()>`

**Content:**
- Title: "Recovery Key"
- Info text explaining what to do
- Read-only display of the 6-word passphrase
- Warning about data loss if passphrase is lost
- Confirm button: "Ho salvato la recovery key"

### 2. RecoveryKeyInputDialog

Shown when `init_db()` returns `DBError::DBKeyMissingWithDb`.

**Props:** `open: Signal<bool>`, `error: Signal<bool>`, `on_recover: EventHandler<String>`, `on_reset: EventHandler<()>`

**Content:**
- Title: "Recovery Key"
- Info text explaining the situation
- Text input for passphrase
- Conditional error message ("Recovery key non valida")
- Two buttons: "Ripristina database" (secondary) and "Recupera" (primary)
- Infinite retry: dialog stays open on error

### 3. DatabaseResetDialog

Confirmation dialog for database reset.

**Props:** `open: Signal<bool>`, `on_confirm: EventHandler<()>`, `on_cancel: EventHandler<()>`

**Content:**
- Alert-error icon
- Title: "Ripristinare il database?"
- Warning text: data will be irreversibly deleted
- Two buttons: "Annulla" (secondary) and "Ripristina" (danger)
- Pattern: identical to existing deletion dialogs (UserDeletionDialog, etc.)

### 4. RecoveryKeyRegenerateDialog

Confirmation dialog for passphrase regeneration (triggered from Settings).

**Props:** `open: Signal<bool>`, `on_confirm: EventHandler<()>`, `on_cancel: EventHandler<()>`

**Content:**
- Alert-error icon
- Title: "Rigenerare la recovery key?"
- Warning text: new passphrase generated, old one won't work, data preserved
- Two buttons: "Annulla" (secondary) and "Rigenera" (danger)

### 5. RecoveryKeyRegeneratedDialog

Shown after successful regeneration. Same layout as RecoveryKeySetupDialog but for the new passphrase.

**Props:** `open: Signal<bool>`, `passphrase: String`, `on_confirm: EventHandler<()>`

## main.rs Integration

```rust
let mut db_resource = use_resource(move || async move { init_db().await });

match db_resource.value() {
    Some(Ok(InitResult::Ready(pool))) =>
        // Normal startup: provide pool context, render Router
    Some(Ok(InitResult::FirstSetup { pool, recovery_phrase })) =>
        // First setup: provide pool context, render Router, show RecoveryKeySetupDialog
    Some(Err(DBError::DBKeyMissingWithDb)) =>
        // Show RecoveryKeyInputDialog
    Some(Err(_)) =>
        // Generic error + retry button (unchanged)
    None =>
        // Spinner (unchanged)
}
```

**RecoveryKeyInputDialog callbacks:**
- `on_recover(passphrase)` → async: derive key → try open DB → on success: close dialog, provide pool, render app; on failure: set error signal
- `on_reset()` → show DatabaseResetDialog → on confirm: delete files → `db_resource.restart()`

## Module Changes

### `db_key.rs` — Rework

| Before | After |
|--------|-------|
| `get_or_create_db_key()` manages all state | `retrieve_db_key_if_exists()` — retrieve only |
| `generate_key()` — 32 random bytes hex | Keep for salt generation |
| `store_db_key()` | Unchanged |
| `retrieve_db_key()` | Simplified, renamed |
| `delete_db_key()` | Unchanged |
| — | `derive_key(passphrase, salt) -> String` — Argon2id → hex |
| — | `derive_key_from_passphrase(passphrase, db_path) -> String` — reads salt file |
| — | `generate_recovery_passphrase() -> String` — 6 diceware words |
| — | `generate_and_store_key(passphrase, salt) -> String` — derive + store in keyring |
| — | `recover_db_key(passphrase, db_path) -> Result<String, DBKeyError>` — full recovery |
| — | `reset_database(db_path)` — delete DB + salt file |

### `db_backend.rs` — `init_db()` rewritten

- Returns `Result<InitResult, DBError>` instead of `Result<SqlitePool, DBError>`
- Removes `create_if_missing(true)` (TOCTOU risk from edge cases plan)
- DB creation only happens in the First Setup branch
- Recovery logic delegated to separate functions called from UI callbacks

### `main.rs`

- Matches on `InitResult` variants instead of simple `Ok(pool)`
- Manages recovery dialog state (open/close/error signals)

### `custom_errors` crate

- New `DBError` variants: `DBKeyMissingWithDb`, `DBRecoveryKeyInvalid`, `DBSaltFileError`

## New Files

| File | Content |
|------|---------|
| `src/components/globals/dialogs/recovery_key_setup.rs` | First setup dialog |
| `src/components/globals/dialogs/recovery_key_input.rs` | Recovery input dialog |
| `src/components/globals/dialogs/recovery_key_regenerate.rs` | Regeneration confirm dialog |
| `src/components/globals/dialogs/database_reset.rs` | Database reset confirm dialog |
| `src/components/globals/dialogs/mod.rs` | Updated with new modules |

## No Migration Needed

The app is in pre-launch. No existing users have encrypted databases. This feature is available from the first release.
