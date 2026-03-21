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
- **Performance:** ~50ms per derivation on modern hardware. All derivation calls MUST run via `tokio::task::spawn_blocking` to avoid blocking the Dioxus UI thread (Argon2id is CPU-bound).

### Salt File Format

Path: `{db_path}.salt` (e.g., `database.db.salt`)
Content: 32-character hex string (16 bytes), not secret.

```
a3f1b2c4d5e6f789012345678abcdef0
```

### Recovery Passphrase

- **Format:** 6 diceware words, CamelCase (e.g., `Word1Word2Word3Word4Word5Word6`)
- **Language:** System language via `sys-locale`, defaults to EN
- **Generation:** Uses `detect_system_language()` → `DicewareLanguage` → `EmbeddedList` (existing `From` impl in `settings_types.rs`), then `diceware::make_passphrase()`. Note: `make_passphrase()` returns `Result<String>`, errors should be mapped to `DBKeyError::KeyringError` since word lists are embedded at compile time and failures indicate corruption.
- **Entropy:** ~77 bits (6 words from 7776-word list), making brute-force impractical.

### First Setup Flow

```
1. Generate salt (16 random bytes) → save to database.db.salt
2. Generate diceware passphrase (6 words, system language, CamelCase)
3. Derive key: Argon2id(passphrase, salt) → [u8; 32] → hex 64 char  [spawn_blocking]
4. Store hex key in keyring (cache for daily use)
5. Open new DB with key, create tables
6. Return InitResult::FirstSetup { pool, recovery_phrase }
```

### Normal Startup Flow

```
1. Read salt from database.db.salt
   - If salt file missing → return Err(DBError::DBSaltFileError) → DB reset required
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
4. Derive key: Argon2id(passphrase, salt)  [spawn_blocking]
5. Try to open DB with derived key
6. Success → store key in keyring → return pool to UI
7. Failure → return Err(DBKeyError::RecoveryKeyInvalid) → retry (infinite)
```

**Security note:** No rate limiting is applied because the recovery flow requires physical access to the machine. The 6-word diceware passphrase provides ~77 bits of entropy, making brute-force impractical.

### Regeneration Flow (from Settings)

Uses the same backup-restore pattern as `migrate_to_encrypted()` to ensure atomicity.

```
1. User clicks "Rigenera recovery key" in settings
2. Confirmation dialog (RecoveryKeyRegenerateDialog)
3. Generate new salt + new diceware passphrase
4. Create backup of database.db (as migrate_to_encrypted does)
5. Re-encrypt DB with new derived key (ATTACH + sqlcipher_export + replace)
6. On failure at any point after step 5 → restore from backup
7. On success:
   a. Update keyring with new derived key
   b. Update database.db.salt with new salt
   c. Clean up backup
8. Show RecoveryKeyRegeneratedDialog with new passphrase
```

**Crash recovery matrix:**

| State after crash | Result |
|-------------------|--------|
| Before step 5 (backup exists, DB unchanged) | Old setup still works |
| After step 5, before step 7a (DB re-encrypted, keyring/salt unchanged) | User recovers via new passphrase + new salt shown in dialog |
| After step 7a, before step 7b (keyring updated, salt unchanged) | User recovers via new passphrase + new salt shown in dialog |
| After step 7b (all updated) | New setup works normally |

The new passphrase is shown to the user only AFTER all writes succeed (step 8), ensuring the user always has a working recovery path.

### Salt File Missing — Edge Case

If `database.db` exists but `database.db.salt` is missing or corrupted:
- Cannot derive any key from a passphrase without the salt
- `init_db()` returns `Err(DBError::DBSaltFileError)`
- UI shows error: "File salt corrotto o mancante. Il database deve essere ripristinato."
- Only option: database reset (same as Database Reset Flow)

### Database Reset Flow

```
1. User clicks "Ripristina database" from RecoveryKeyInputDialog
2. Confirmation dialog (DatabaseResetDialog)
3. Delete database.db + database.db.salt
4. Reset db_init_notified flag (so success toast shows after fresh setup)
5. Restart init_db() via db_resource.restart() → triggers First Setup flow
```

## Data Types

### InitResult (new)

```rust
pub enum InitResult {
    Ready(SqlitePool),
    FirstSetup { pool: SqlitePool, recovery_phrase: SecretString },
}
```

The passphrase uses `SecretString` (from the `secrecy` crate, already used throughout the project) for defense-in-depth. The UI component calls `.expose_secret()` only when rendering.

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
SaltFileError(msg) ─────→ DBError::DBSaltFileError(msg)─match──→ "Salt corrotto" + reset
```

## UI Components

All dialogs follow the existing `BaseModal` + `ActionButton` pattern.

### 1. RecoveryKeySetupDialog

Shown on first setup. Displays the generated passphrase.

**Props:** `open: Signal<bool>`, `passphrase: String`, `on_confirm: EventHandler<()>`

**Behavior:** Non-dismissable — no X button, no cancel. The user MUST acknowledge the recovery phrase before proceeding. Dismissing without saving the phrase is equivalent to data loss.

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

Shown after successful regeneration. Same layout as RecoveryKeySetupDialog but for the new passphrase. Also non-dismissable.

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
    Some(Err(DBError::DBSaltFileError(_))) =>
        // Show error "Salt file corrotto o mancante" + show DatabaseResetDialog
    Some(Err(_)) =>
        // Generic error + retry button (unchanged)
    None =>
        // Spinner (unchanged)
}
```

**RecoveryKeyInputDialog callbacks:**
- `on_recover(passphrase)` → async via `spawn_blocking`: derive key → try open DB → on success: close dialog, provide pool, render app; on failure: set error signal
- `on_reset()` → show DatabaseResetDialog → on confirm: delete files → reset `db_init_notified` → `db_resource.restart()`

## Module Changes

### `db_key.rs` — Rework

All new functions remain `#[cfg(feature = "desktop")]`, consistent with the existing module gate in `src/backend/mod.rs`.

| Before | After |
|--------|-------|
| `get_or_create_db_key()` manages all state | `retrieve_db_key_if_exists()` — retrieve only |
| `generate_key()` — 32 random bytes hex | Keep for salt generation |
| `store_db_key()` | Unchanged |
| `retrieve_db_key()` | Simplified, renamed |
| `delete_db_key()` | Unchanged |
| — | `derive_key(passphrase, salt) -> Result<String, DBKeyError>` — Argon2id → hex |
| — | `derive_key_from_passphrase(passphrase, db_path) -> Result<String, DBKeyError>` — reads salt file |
| — | `generate_recovery_passphrase() -> Result<String, DBKeyError>` — 6 diceware words |
| — | `generate_and_store_key(passphrase, salt) -> Result<String, DBKeyError>` — derive + store in keyring |
| — | `recover_db_key(passphrase, db_path) -> Result<String, DBKeyError>` — full recovery |
| — | `reset_database(db_path)` — delete DB + salt file |

### `db_backend.rs` — `init_db()` rewritten

All changes remain `#[cfg(feature = "desktop")]`.
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
