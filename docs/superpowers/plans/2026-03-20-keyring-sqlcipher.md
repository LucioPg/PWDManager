# Keyring + SQLCipher Database Encryption

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Encrypt the SQLite database at rest using SQLCipher with a per-installation random key stored in the OS keyring (Windows Credential Manager).

**Architecture:**
- A `db_key` module handles key generation (32 random bytes, hex-encoded) and OS keyring storage via the `keyring` crate.
- `init_db()` retrieves (or creates) the key on startup, then opens the database with SQLCipher via `PRAGMA key = "x'...'"`.
- Existing unencrypted databases are automatically detected (via file header check) and migrated in-place using `sqlcipher_export`.
- The existing export/import feature serves as the backup/recovery mechanism — no additional key backup needed.

**Tech Stack:** keyring v3 (windows-native), SQLCipher (libsqlite3-sys/bundled-sqlcipher), sqlx 0.8, rand 0.9

**IMPORTANT:** This work must be executed in an isolated git worktree. See Task 0.

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/backend/db_key.rs` | OS keyring operations: generate, store, retrieve DB encryption key |
| Modify | `Cargo.toml:14-47` | Add keyring, rand, libsqlite3-sys dependencies |
| Modify | `src/backend/db_backend.rs:81-100` | SQLCipher integration + migration in `init_db()` |
| Modify | `src/backend/mod.rs:1` | Export `db_key` module |

---

## Task 0: Create Worktree

> **Tutte le task successive devono essere eseguite all'interno del worktree.**

- [ ] **Step 1: Create isolated worktree**

```bash
git worktree add .claude/worktrees/keyring-sqlcipher -b feature/keyring-sqlcipher
```

- [ ] **Step 2: Switch working directory**

```bash
cd .claude/worktrees/keyring-sqlcipher
```

Verify you're on the right branch:
```bash
git branch --show-current
```
Expected: `feature/keyring-sqlcipher`

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add keyring and rand to `[dependencies]`**

Add after the existing `# Dipendenze esterne` section (after line 47):

```toml
# DB Encryption
keyring = { version = "3", features = ["windows-native"] }
rand = "0.9"
```

- [ ] **Step 2: Add SQLCipher via libsqlite3-sys**

Modify the existing `sqlx` dependency (line 30) and add `libsqlite3-sys` in the same section:

```toml
sqlx = { version = "0.8.6", default-features = false, features = ["runtime-tokio", "sqlite", "macros"] }
libsqlite3-sys = { version = "0.30", default-features = false, features = ["bundled-sqlcipher"] }
```

> **RISK:** `bundled-sqlcipher` may require OpenSSL. If `cargo check` fails with a missing OpenSSL error, also add:
> ```toml
> openssl-sys = { version = "0.9", features = ["vendored"] }
> ```
> Note: `vendored` OpenSSL requires a C compiler (MSVC/Build Tools) and adds several minutes to first build.

> **RISK:** `default-features = false` on sqlx may disable needed features. If `cargo check` fails, check the error and re-enable specific features as needed (e.g. `tls` if sqlx depends on it).

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

Expected: Compiles successfully. First build will be slow (several minutes) due to SQLCipher/OpenSSL C compilation.

If it fails:
1. Check if `bundled` and `bundled-sqlcipher` conflict on `libsqlite3-sys` — the `default-features = false` should prevent this
2. Add `openssl-sys` with `vendored` if OpenSSL is missing
3. Read the error carefully — SQLCipher build errors usually point to missing crypto libraries

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add keyring, rand, and SQLCipher (bundled-sqlcipher)"
```

---

## Task 2: Create db_key Module (TDD)

**Files:**
- Create: `src/backend/db_key.rs`
- Modify: `src/backend/mod.rs:1`

- [ ] **Step 1: Write the implementation**

Create `src/backend/db_key.rs`:

```rust
use keyring::Entry;
use rand::Rng;

/// Service name used in the OS keyring (Windows Credential Manager).
pub const SERVICE_NAME: &str = "PWDManager";

/// Username/identifier for the DB encryption key in the keyring.
pub const KEY_USERNAME: &str = "db_encryption_key";

/// Error type for keyring operations.
#[derive(Debug)]
pub enum DBKeyError {
    /// The requested credential does not exist in the keyring.
    NoEntry,
    /// A keyring system error (service unavailable, access denied, etc.)
    KeyringError(String),
}

impl std::fmt::Display for DBKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBKeyError::NoEntry => write!(f, "Keyring entry not found"),
            DBKeyError::KeyringError(msg) => write!(f, "Keyring error: {}", msg),
        }
    }
}

impl std::error::Error for DBKeyError {}

/// Generates a 64-character hex string (32 random bytes, hex-encoded).
/// Used as raw key material via `PRAGMA key = "x'...'"`.
fn generate_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill(&mut key_bytes);
    key_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Stores a key in the OS keyring under the given service/username.
fn store_db_key(service: &str, username: &str, key: &str) -> Result<(), DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.set_password(key)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))
}

/// Retrieves a key from the OS keyring.
fn retrieve_db_key(service: &str, username: &str) -> Result<String, DBKeyError> {
    let entry = Entry::new(service, username)
        .map_err(|e| DBKeyError::KeyringError(e.to_string()))?;
    entry.get_password().map_err(|e| {
        if e.to_string().contains("NoEntry") || e.to_string().contains("no entry") {
            DBKeyError::NoEntry
        } else {
            DBKeyError::KeyringError(e.to_string())
        }
    })
}

/// Gets the existing key from keyring, or generates and stores a new one.
/// This is the main entry point called by `init_db()`.
///
/// - `Ok(key)` if a key exists or was successfully created
/// - `Err(DBKeyError::NoEntry)` is never returned (we create on miss)
/// - `Err(DBKeyError::KeyringError)` if the keyring is unavailable
pub fn get_or_create_db_key() -> Result<String, DBKeyError> {
    match retrieve_db_key(SERVICE_NAME, KEY_USERNAME) {
        Ok(key) => Ok(key),
        Err(DBKeyError::NoEntry) => {
            let key = generate_key();
            store_db_key(SERVICE_NAME, KEY_USERNAME, &key)?;
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

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
    fn test_get_or_create_always_returns_valid_key() {
        // get_or_create_db_key uses the real SERVICE_NAME/KEY_USERNAME.
        // It should always succeed (create if missing).
        let result = get_or_create_db_key();
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.len(), 64);
    }
}
```

- [ ] **Step 2: Add module to `mod.rs`**

Add at the top of `src/backend/mod.rs` (after `pub mod settings_types;`):

```rust
#[cfg(feature = "desktop")]
pub mod db_key;
```

> **Note:** The `#[cfg(feature = "desktop")]` guard prevents compilation errors on non-desktop targets where `keyring` with `windows-native` feature would fail.

- [ ] **Step 3: Run tests**

```bash
cargo test --lib backend::db_key::tests
```

Expected: All 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/backend/db_key.rs src/backend/mod.rs
git commit -m "feat: add db_key module for OS keyring integration"
```

---

## Task 3: SQLCipher Integration in init_db()

**Files:**
- Modify: `src/backend/db_backend.rs:1-100`

This task modifies `init_db()` to:
1. Get/create the DB encryption key from the OS keyring
2. Detect if an existing database is unencrypted (migration needed)
3. Migrate unencrypted databases to SQLCipher
4. Open the database with SQLCipher encryption

- [ ] **Step 1: Add imports**

At the top of `db_backend.rs`, add to the existing imports:

```rust
use crate::backend::db_key;
```

> `SqliteConnectOptions`, `SqliteJournalMode`, `SqlitePool`, and `FromStr` are already imported (line 12-14).

- [ ] **Step 2: Add unencrypted DB detection function**

Add before `init_db()` (around line 59):

```rust
/// Checks if a SQLite file is unencrypted by reading its magic header.
/// Regular SQLite files start with `"SQLite format 3\0"`.
/// SQLCipher encrypted files start with random bytes.
#[cfg(feature = "desktop")]
fn is_database_unencrypted(path: &str) -> bool {
    match std::fs::File::open(path) {
        Ok(mut file) => {
            let mut header = [0u8; 16];
            match std::io::Read::read_exact(&mut file, &mut header) {
                Ok(()) => header == *b"SQLite format 3\0",
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}
```

- [ ] **Step 3: Add migration function**

Add after the detection function, before `init_db()`:

```rust
/// Migrates an unencrypted SQLite database to SQLCipher format.
///
/// Flow:
/// 1. Backup original file to `database.db.pre-encryption-backup`
/// 2. Open the unencrypted DB without a key (plaintext mode in SQLCipher)
/// 3. Acquire a **single connection** (ATTACH is per-connection, not per-pool)
/// 4. Attach a new encrypted temp DB with the keyring key
/// 5. Use `sqlcipher_export` to copy all data
/// 6. Replace the original file with the encrypted version
/// 7. Clean up backup and old WAL/SHM files on success
#[cfg(feature = "desktop")]
async fn migrate_to_encrypted(path: &str, key: &str) -> Result<(), DBError> {
    let backup_path = format!("{}.pre-encryption-backup", path);
    let temp_path = format!("{}.encrypted_tmp", path);

    // Backup original
    std::fs::copy(path, &backup_path)
        .map_err(|e| DBError::new_general_error(format!("Backup failed: {}", e)))?;

    // Remove stale temp file if present
    let _ = std::fs::remove_file(&temp_path);

    // Open unencrypted source DB (no PRAGMA key = plaintext mode in SQLCipher)
    let source_opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", path))
        .map_err(|e| DBError::new_general_error(e.to_string()))?;
    let pool = SqlitePool::connect_with(source_opts)
        .await
        .map_err(|e| DBError::new_general_error(format!("Cannot open source DB: {}", e)))?;

    // CRITICAL: acquire a single connection — ATTACH/DETACH/sqlcipher_export
    // are all per-connection operations and MUST run on the same connection.
    let mut conn = pool.acquire()
        .await
        .map_err(|e| DBError::new_general_error(format!("Cannot acquire connection: {}", e)))?;

    // Attach encrypted target DB
    let attach_sql = format!(
        "ATTACH DATABASE '{}' AS encrypted KEY \"x'{}'\"",
        temp_path, key
    );
    sqlx::query(&attach_sql)
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            DBError::new_general_error(format!("ATTACH failed: {}", e))
        })?;

    // Export all data from unencrypted source to encrypted target
    sqlx::query("SELECT sqlcipher_export('encrypted')")
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            DBError::new_general_error(format!("sqlcipher_export failed: {}", e))
        })?;

    // Detach
    sqlx::query("DETACH DATABASE encrypted")
        .execute(&mut *conn)
        .await
        .map_err(|e| DBError::new_general_error(format!("DETACH failed: {}", e)))?;

    // Release connection and close pool (release file locks)
    drop(conn);
    pool.close().await;

    // Replace original with encrypted version
    std::fs::rename(&temp_path, path)
        .map_err(|e| {
            // Try to restore backup on failure
            let _ = std::fs::copy(&backup_path, path);
            DBError::new_general_error(format!("Replace failed: {}", e))
        })?;

    // Remove backup on success
    let _ = std::fs::remove_file(&backup_path);

    // Remove old WAL/SHM files from the unencrypted database
    let _ = std::fs::remove_file(&format!("{}-wal", path));
    let _ = std::fs::remove_file(&format!("{}-shm", path));

    Ok(())
}
```

- [ ] **Step 4: Replace `init_db()` function**

Replace the existing `init_db()` (lines 81-100) with:

```rust
#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<SqlitePool, DBError> {
    let db_key = db_key::get_or_create_db_key()
        .map_err(|e| DBError::new_general_error(format!("Keyring error: {}", e)))?;

    let db_path = "database.db";

    // Migrate existing unencrypted database if detected
    if is_database_unencrypted(db_path) {
        tracing::warn!("Detected unencrypted database — migrating to SQLCipher");
        migrate_to_encrypted(db_path, &db_key).await?;
        tracing::info!("Database migration to SQLCipher complete");
    }

    // Build PRAGMA key command using raw hex key material
    let pragma_key = format!("PRAGMA key = \"x'{}'\"", db_key);

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .pragma("foreign_keys", "ON")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true)
        .after_connect(move |conn, _meta| {
            let pragma = pragma_key.clone();
            Box::pin(async move {
                sqlx::query(&pragma)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
                // Verify decryption works (PRAGMA key itself never fails)
                sqlx::query("SELECT count(*) FROM sqlite_master")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
                Ok(())
            })
        });

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| DBError::new_general_error(
            format!("Failed to open encrypted database. \
                     If you reinstalled the app without exporting passwords first, \
                     the database is unrecoverable. ({})", e)
        ))?;

    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    }

    Ok(pool)
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check
```

Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: integrate SQLCipher with OS keyring key in init_db"
```

---

## Task 4: Manual Integration Verification

> **These steps require running the actual app. They cannot be automated in unit tests.**

- [ ] **Step 1: Test first run (fresh install scenario)**

```bash
# Remove existing database and keyring entry
rm -f database.db database.db-wal database.db-shm

# Run the app
dx serve --desktop
```

Verify:
- [ ] App starts normally
- [ ] Login works
- [ ] Password CRUD works
- [ ] Database file is not readable with standard SQLite tools

- [ ] **Step 2: Verify encryption with sqlite3**

```bash
# If you have sqlite3 CLI installed:
sqlite3 database.db "SELECT count(*) FROM sqlite_master;"
```

Expected: Error like `file is not a database` or `file is encrypted`.

- [ ] **Step 3: Verify keyring entry**

Open Windows Credential Manager (`cmdkeywiz` or Control Panel → Credential Manager → Windows Credentials).
Look for an entry with target `PWDManager`.
The password should be a 64-character hex string.

- [ ] **Step 4: Test migration scenario**

```bash
# Stop the app
# Restore an old unencrypted database.db (from git history or backup)
# Start the app again
```

Verify:
- [ ] App detects unencrypted DB (check logs for "migrating to SQLCipher")
- [ ] Migration completes successfully
- [ ] App functions normally after migration
- [ ] Backup file (`database.db.pre-encryption-backup`) is deleted

- [ ] **Step 5: Test export/import still works**

```bash
# In the app:
# 1. Export passwords to JSON/CSV/XML
# 2. Delete a stored password
# 3. Re-import the exported file
# 4. Verify the password is restored
```

- [ ] **Step 6: Run automated tests**

```bash
cargo test
```

Expected: All tests pass.

- [ ] **Step 7: Commit any fixes discovered during verification**

```bash
git add -A
git commit -m "fix: address issues found during integration testing"
```

---

## Task 5: Update TODO.md

- [ ] **Step 1: Mark keyring task as done**

In `TODO.md`, update the relevant items:

Mark as done:
```
- [x] usare il keyring del sistema operativo per generare la password per ogni installazione
```

Remove or comment out the obfstr alternative:
```
~~- [ ] la password db deve essere offuscata tramite il crate obfstr~~
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "docs: update TODO for keyring implementation"
```
