# Security Architecture

This document describes the cryptographic layers that protect stored passwords, the database, and the recovery mechanism.

## Overview

PWDManager uses a layered encryption model. Each layer is independent: compromising one does not expose data protected by another.

```
Stored passwords
  |-- AES-256-GCM (per-field encryption with unique nonces)
  |
Database file
  |-- SQLCipher (AES-256, transparent page-level encryption)
  |
Database key
  |-- Windows Credential Manager (OS-provided storage)
  |-- Derived from recovery passphrase via Argon2id (backup path)
  |
User authentication
  |-- Argon2 (password hashing with zeroize)
```

## Layer 1: Database Encryption (SQLCipher)

The SQLite database is encrypted at rest using SQLCipher with the following pinned parameters:

| Parameter | Value | Purpose |
|---|---|---|
| `cipher_page_size` | 4096 | Disk page layout |
| `cipher_hmac_algorithm` | HMAC_SHA512 | Page integrity verification |
| `kdf_iter` | 256000 | Key derivation iterations (pinned for consistency) |

The key is a 64-character hex string (32 random bytes) passed via `PRAGMA key = "x'...'"`. It is not derived from a user password -- it is randomly generated at setup time.

SQLCipher encrypts every page before writing to disk and decrypts on read. Without the key, the database file is indistinguishable from random data. WAL mode is enabled for concurrent read/write safety.

**Source:** `src/backend/db_backend.rs` (`build_sqlcipher_options`)

## Layer 2: Per-Field Encryption (AES-256-GCM)

Each credential stored in the database is individually encrypted before insertion. The following fields are encrypted with AES-256-GCM using unique nonces:

- `username`
- `url`
- `password`
- `notes` (optional)

The AES cipher key is derived from the user's login password using Argon2id. The salt is extracted from the Argon2 hash stored in the `users` table. This means each user has their own encryption domain: two users with the same password store different ciphertext for the same credential.

**Nonce management:** Each encrypted field uses a cryptographically random 12-byte nonce, generated per-field at encryption time via `aes-gcm::create_nonce()`. Nonces are stored alongside the ciphertext in the database.

**Bulk operations:** Encryption and decryption of large credential sets run in parallel via `rayon::par_iter`, offloaded to a blocking thread pool to avoid starving the async runtime.

**Source:** `src/backend/password_utils.rs` (`create_stored_data_records`, `decrypt_bulk_stored_data`)

### Password Change and Re-encryption

When a user changes their password, all stored credentials must be re-encrypted with the new cipher key. The migration pipeline:

1. Decrypt all credentials with the old password's cipher
2. Re-encrypt with the new password's cipher
3. Batch-update the database
4. Remove the temporary old password hash

This runs as a background task with progress reporting through the UI.

**Source:** `src/backend/password_utils.rs` (`stored_passwords_migration_pipeline_with_progress`)

## Layer 3: Authentication (Argon2)

User passwords are hashed with Argon2 before storage. The crate configuration:

- Standard Argon2id parameters
- `zeroize` feature enabled: password memory is zeroed after hashing

The Argon2 hash string is stored in the `users.password` column. The embedded salt within the hash is also extracted at runtime to serve as the AES key derivation salt, avoiding a separate salt storage for the cipher.

**Source:** `pwd-crypto` crate (external, `https://github.com/LucioPg/pwd-crypto`)

## Layer 4: Database Key Management

### Normal Operation

On first setup, a random 32-byte key is generated and stored in Windows Credential Manager under the service name `PWDManager`. This key is used directly as the SQLCipher encryption key. Every subsequent startup retrieves the key from the keyring and opens the database.

The keyring provides OS-level protection: credentials are encrypted by Windows DPAPI and accessible only to the current user.

### First Setup (Release)

In release builds, the NSIS installer runs the application with a `--setup` flag before first launch. This triggers a headless setup that:

1. Generates a random 6-word Diceware passphrase (CamelCase, system language)
2. Derives a 256-bit key via Argon2id with a random 16-byte salt
3. Stores the derived key in Windows Credential Manager
4. Writes the salt to `database.db.salt` (hex-encoded, 32 characters)
5. Creates the encrypted database and runs initialization queries
6. Prints the recovery passphrase to stdout (captured by the NSIS installer and displayed to the user)

The passphrase is shown exactly once. If the user loses it, database recovery is not possible.

### First Setup (Development)

In debug builds, a fixed passphrase (`CorrectHorseBatteryStaple`) is used with a separate keyring entry (`PWDManager-dev`) to avoid conflicts with production data.

**Source:** `src/backend/db_key.rs`, `src/backend/setup.rs`

## Recovery Key

The recovery key is a 6-word Diceware passphrase generated from the BEALE wordlist (English), the EFF Italian wordlist, or the EFF French wordlist, depending on the system locale at generation time. Words are concatenated in CamelCase with no separators.

The recovery key serves as a backup path to derive the database encryption key when the keyring entry is lost or corrupted. The derivation process:

1. User enters the recovery passphrase in the recovery dialog
2. The salt is read from `database.db.salt`
3. Argon2id derives the 256-bit key from passphrase + salt
4. SQLCipher opens the database with the derived key
5. On success, the key is stored back in the keyring

### Recovery Key Regeneration

The user can regenerate the recovery key from the Settings page. This triggers a full database rekey:

1. Backup the current salt file
2. Generate a new Diceware passphrase + salt + derived key
3. Write the new salt file
4. Execute `PRAGMA rekey` to re-encrypt the entire database with the new key
5. On success: update the keyring and return the new passphrase
6. On failure: restore the old salt file

The old passphrase becomes invalid after successful regeneration.

**Source:** `src/backend/db_backend.rs` (`rekey_database`)

## Password Strength Evaluation

Passwords are evaluated using a scoring system from the `pwd-strength` crate:

| Score Range | Level |
|---|---|
| 0-49 | WEAK |
| 50-69 | MEDIUM |
| 70-84 | STRONG |
| 85-95 | EPIC |
| 96-100 | GOD |

The evaluation includes a blacklist of common passwords (bundled as `assets/blacklist.txt`). Passwords appearing in the blacklist receive a score penalty.

Imported passwords have their score cleared (`score = None`) and re-evaluated on import, ensuring the blacklist check is applied retroactively.

**Source:** `pwd-strength` crate (external, `https://github.com/LucioPg/pwd-strength`), `docs/password_strength_levels.md`

## Password Generation

### Random Passwords

Generated using the `pwd-types` `AegisPasswordConfig` adapter. Configuration includes:

- Length
- Character sets: uppercase, lowercase, digits, symbols
- Minimum count constraints per character type
- Excluded symbols

The generator retries until all constraints are satisfied simultaneously.

### Diceware Passphrases

Generated using the `diceware` crate with embedded wordlists. Configuration:

- Word count
- Language: English (BEALE), Italian (EFF), French (EFF)
- Special characters (appended or inserted between words)
- Numeric words (replace some words with random digits)
- CamelCase formatting

**Source:** `src/backend/password_utils.rs`, `diceware` crate (external)

## Sensitive Data Handling

### SecretString

The `secrecy` crate provides `SecretString`, which wraps strings in a `SecretBox` that is zeroized on drop. All passwords and passphrases in memory use `SecretString`. The `ExposeSecret` trait is required to access the underlying value, making accidental logging or leaking harder.

### Export Data

Exported files contain decrypted credentials in plaintext. The user is warned via a confirmation dialog before export. Export formats are JSON (pretty-printed), CSV, and XML.

### Clipboard

When the user copies a password to clipboard, the password is exposed in the system clipboard. The application does not clear the clipboard after a timeout -- this is the user's responsibility.

## Threat Model

### In scope

- Local attacker with filesystem access: protected by SQLCipher encryption and per-field AES-256-GCM
- Stolen database file: useless without the key from Windows Credential Manager or the recovery passphrase
- Stolen laptop: requires Windows login to access the keyring entry
- Malware running as the current user: can access the keyring and the database. This is a fundamental OS-level limitation.

### Out of scope

- Physical access to an unlocked machine with an active session: the database key is in memory and credentials are decrypted on demand. Locking the workstation is the user's responsibility.
- Supply chain attacks on Rust crates: mitigated by vendoring critical crypto dependencies and reviewing their source.
- Memory forensics: SecretString provides zeroize-on-drop, but a sufficiently privileged attacker can inspect process memory at runtime.

## Auto-Logout

The application supports configurable session timeouts (10 minutes, 1 hour, 5 hours). When the timer expires, the user is logged out and all decrypted data is dropped from memory. The database connection remains open but requires re-authentication to access stored credentials.

**Source:** `src/backend/settings_types.rs` (`AutoLogoutSettings`)

## Update Integrity

Application updates are verified with minisign before installation. The public key is embedded at compile time (`keys/update-public.key`). The verification flow:

1. Download the update archive and the signature from the release endpoint
2. Base64-decode the signature
3. Verify the signature against the archive using the embedded public key
4. Extract and launch the NSIS installer only if verification succeeds

This prevents man-in-the-middle attacks on the update channel.

**Source:** `src/backend/updater.rs`, `docs/AUTO_UPDATE_PATTERN.md`
