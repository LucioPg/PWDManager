# User Settings Automation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automate user settings creation during registration with preset-based password generation configuration.

**Architecture:** Create `settings_types.rs` with `PasswordPreset` enum and DB structs. Modify `save_or_update_user` to return user_id. Add `create_user_settings` function with transaction. Integrate in registration flow.

**Tech Stack:** Rust, Dioxus 0.7, sqlx, sqlx-template, SQLite

---

## Task 1: Create settings_types.rs

**Files:**
- Create: `src/backend/settings_types.rs`

**Step 1: Create the file with all types**

```rust
//! Tipi per la gestione dei settings utente.
//!
//! Contiene il preset per la generazione password e le struct
//! per il mapping con le tabelle del database.

use sqlx::FromRow;
use sqlx_template::SqliteTemplate;

/// Preset per la generazione password.
///
/// I valori sono calcolati per garantire che la password generata
/// rientri nella fascia di strength corretta secondo `strength_utils`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPreset {
    Medium,
    Strong,
    Epic,
    God,
}

impl PasswordPreset {
    /// Restituisce la configurazione per questo preset.
    ///
    /// # Valori calcolati da strength_utils
    ///
    /// | Preset | length | symbols | Score |
    /// |--------|--------|---------|-------|
    /// | Medium | 8 | 2 | 69 |
    /// | Strong | 12 | 2 | 81 |
    /// | Epic | 16 | 2 | 93 |
    /// | God | 26 | 2 | 98 |
    pub fn to_config(&self) -> PasswordGenConfig {
        match self {
            Self::Medium => PasswordGenConfig {
                length: 8,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::Strong => PasswordGenConfig {
                length: 12,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::Epic => PasswordGenConfig {
                length: 16,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
            Self::God => PasswordGenConfig {
                length: 26,
                symbols: 2,
                numbers: true,
                uppercase: true,
                lowercase: true,
            },
        }
    }
}

/// Configurazione per la generazione password (in memoria).
///
/// Usata per passare i parametri di configurazione senza
/// dipendere dal database.
#[derive(Debug, Clone, Copy)]
pub struct PasswordGenConfig {
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
}

/// Settings generali utente.
///
/// Mappa la tabella `user_settings` del database.
#[derive(Debug, Clone, FromRow, SqliteTemplate)]
#[table("user_settings")]
#[tp_upsert(by = "id")]
pub struct UserSettings {
    pub id: Option<i64>,
    pub user_id: i64,
}

/// Settings per la generazione password.
///
/// Mappa la tabella `passwords_generation_settings` del database.
#[derive(Debug, Clone, FromRow, SqliteTemplate)]
#[table("passwords_generation_settings")]
#[tp_upsert(by = "id")]
pub struct PasswordsGenSettings {
    pub id: Option<i64>,
    pub settings_id: i64,
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub excluded_symbols: Option<String>,
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Success (no errors related to new file)

**Step 3: Commit**

```bash
git add src/backend/settings_types.rs
git commit -m "feat: add settings_types.rs with PasswordPreset and DB structs

- PasswordPreset enum with Medium/Strong/Epic/God variants
- PasswordGenConfig for in-memory configuration
- UserSettings and PasswordsGenSettings with sqlx-template derives

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Export settings_types from mod.rs

**Files:**
- Modify: `src/backend/mod.rs`

**Step 1: Add the module and export**

In `src/backend/mod.rs`, add after line 1:

```rust
pub mod settings_types;
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Success

**Step 3: Commit**

```bash
git add src/backend/mod.rs
git commit -m "feat: export settings_types module from backend

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Modify save_or_update_user to return user_id

**Files:**
- Modify: `src/backend/db_backend.rs` (lines 198-260)

**Step 1: Update function signature**

Change line 198-204 from:
```rust
pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i64>,
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<(), DBError> {
```

To:
```rust
pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i64>,
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<i64, DBError> {
```

**Step 2: Update UPDATE case to return user_id**

Change lines 211-238, replace the `Some(user_id)` branch with:

```rust
        Some(user_id) => {
            let update = prepare_user_update(pool, user_id, username, password, avatar).await?;

            if !update.has_updates() {
                return Ok(user_id);
            }

            let sql_fields = update.build_sql_fields();
            let sql = format!("UPDATE users SET {} WHERE id = ?", sql_fields.join(", "));

            let mut query = sqlx::query(&sql);

            if let Some(username) = update.username {
                query = query.bind(username);
            }
            if let Some(password) = update.password {
                query = query.bind(password.expose_secret().to_string());
            }
            if let Some(avatar) = update.avatar {
                query = query.bind(avatar);
            }
            query = query.bind(user_id);

            query
                .execute(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;

            Ok(user_id)
        }
```

**Step 3: Update INSERT case to use RETURNING id**

Replace lines 241-256 (`None =>` branch) with:

```rust
        None => {
            let psw = password.unwrap_or_default();
            if !psw.expose_secret().trim().is_empty() {
                let hash_password = crate::backend::utils::encrypt(psw)
                    .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;

                let user_id: i64 = sqlx::query_scalar(
                    "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id"
                )
                .bind(&username)
                .bind(&hash_password)
                .bind(&avatar)
                .fetch_one(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;

                Ok(user_id)
            } else {
                Err(DBError::new_save_error("Password cannot be empty".into()))
            }
        }
```

**Step 4: Remove the final `Ok(())`**

Remove line 259 (`Ok(())`) as each branch now returns explicitly.

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Error in `upsert_user.rs` about unused `Ok(_)` - this is expected, we'll fix in Task 5

**Step 6: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: save_or_update_user returns user_id instead of ()

- Use RETURNING id for INSERT (SQLite 3.35+)
- Return existing user_id for UPDATE
- Breaking change: callers must handle returned i64

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Add create_user_settings function

**Files:**
- Modify: `src/backend/db_backend.rs`

**Step 1: Add imports at the top of the file**

Add after line 3:
```rust
use crate::backend::settings_types::{PasswordPreset, PasswordGenConfig, UserSettings, PasswordsGenSettings};
```

**Step 2: Add the function after save_or_update_user (around line 260)**

```rust
/// Crea i settings di default per un nuovo utente.
///
/// Usa una transazione per garantire atomicità tra i due INSERT.
/// Se la transazione fallisce, nessun record viene creato.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente per cui creare i settings
/// * `preset` - Preset di default per la generazione password
///
/// # Valore Restituito
///
/// Return `Ok(())` se i settings vengono creati con successo
///
/// # Errori
///
/// - `DBError::new_general_error` - Errore nell'avviare o committare la transazione
/// - `DBError::new_save_error` - Errore durante l'INSERT
pub async fn create_user_settings(
    pool: &SqlitePool,
    user_id: i64,
    preset: PasswordPreset,
) -> Result<(), DBError> {
    // Inizia transazione
    let mut tx = pool.begin().await
        .map_err(|e| DBError::new_general_error(format!("Failed to start transaction: {}", e)))?;

    // 1. Inserisci user_settings e ottieni l'id con RETURNING
    let settings_id: i64 = sqlx::query_scalar(
        "INSERT INTO user_settings (user_id) VALUES (?) RETURNING id"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| DBError::new_save_error(format!("Failed to insert user_settings: {}", e)))?;

    // 2. Inserisci passwords_generation_settings
    let config = preset.to_config();
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)"
    )
    .bind(settings_id)
    .bind(config.length)
    .bind(config.symbols)
    .bind(config.numbers)
    .bind(config.uppercase)
    .bind(config.lowercase)
    .execute(&mut *tx)
    .await
    .map_err(|e| DBError::new_save_error(format!("Failed to insert gen_settings: {}", e)))?;

    // Commit transazione
    tx.commit().await
        .map_err(|e| DBError::new_general_error(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Success

**Step 4: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: add create_user_settings function with transaction

- Uses RETURNING id to get generated user_settings.id
- Wraps both INSERTs in a transaction for atomicity
- Creates passwords_generation_settings with preset config

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Integrate settings creation in registration flow

**Files:**
- Modify: `src/components/features/upsert_user.rs`

**Step 1: Add import for create_user_settings**

Add at line 2, modify the existing import:
```rust
use crate::backend::db_backend::{create_user_settings, delete_user, save_or_update_user};
```

**Step 2: Add import for PasswordPreset**

Add after line 11:
```rust
use crate::backend::settings_types::PasswordPreset;
```

**Step 3: Update the on_submit handler**

Replace lines 205-219 with:

```rust
        spawn(async move {
            match save_or_update_user(&pool, user_id, u, password_to_save, a).await {
                Ok(saved_user_id) => {
                    // Se è un nuovo utente, crea i settings di default
                    if user_id.is_none() {
                        if let Err(e) = create_user_settings(&pool, saved_user_id, PasswordPreset::God).await {
                            tracing::warn!("Failed to create user settings for user {}: {}", saved_user_id, e);
                            // Non blocchiamo la registrazione - l'utente può configurare i settings dopo
                        }
                    }

                    auth_state.logout();
                    let message = if is_updating {
                        "User Updated successfully!"
                    } else {
                        "User Registered successfully!"
                    };
                    schedule_toast_success(message.to_string(), toast);
                    nav.push("/login");
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Success

**Step 5: Commit**

```bash
git add src/components/features/upsert_user.rs
git commit -m "feat: create default settings after user registration

- Call create_user_settings with GOD preset for new users
- Non-blocking: log warning on failure, continue registration
- Only create settings for new users (not updates)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Run tests and verify

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run the application to test manually**

Run: `dx serve --desktop`
Expected: Application starts without errors

**Step 3: Test registration flow**

1. Open the application
2. Navigate to registration
3. Create a new user
4. Verify the user is created and redirected to login
5. (Optional) Check database for the new settings records

**Step 4: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: any issues found during testing

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Create settings_types.rs | `src/backend/settings_types.rs` |
| 2 | Export from mod.rs | `src/backend/mod.rs` |
| 3 | Modify save_or_update_user | `src/backend/db_backend.rs` |
| 4 | Add create_user_settings | `src/backend/db_backend.rs` |
| 5 | Integrate in registration | `src/components/features/upsert_user.rs` |
| 6 | Test and verify | - |

**Total estimated changes:**
- 1 new file
- 3 modified files
- ~150 lines of new code
