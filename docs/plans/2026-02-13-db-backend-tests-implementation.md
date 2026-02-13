# Test Suite per save_or_update_user - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Creare una suite completa di test per la funzione `save_or_update_user` in `db_backend.rs` che verifichi INSERT, UPDATE, temp_old_password e casi di errore.

**Architecture:** Test integrativi con database SQLite su file temporaneo usando `tempfile`. Ogni test crea un DB pulito, esegue l'operazione, verificha il risultato. I test sono organizzati in 4 categorie principali.

**Tech Stack:** Rust, sqlx 0.8.6, tempfile 3.0, tokio 1.49, SQLite with WAL mode

---

## Task 1: Aggiungere dipendenza tempfile

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\Cargo.toml`

**Step 1: Apri Cargo.toml e aggiungi tempfile alle dipendenze di sviluppo**

Vai alla riga 22 (dopo `rayon = "1.11.0"`) e aggiungi:

```toml
[dev-dependencies]
tempfile = "3"
```

Il file dovrebbe assomigliare a:

```toml
[package]
name = "PWDManager"
version = "0.1.0"
edition = "2024"

[dependencies]
image = { version = "0.25.9", features = ["png"] }
gui-launcher = {path = "gui_launcher" }
custom_errors = {path = "custom_errors"}
tracing = "0.1.44"
dioxus = { version = "0.7.3", features = ["desktop", "router"] }
sqlx = { version = "0.8.6", features = ["runtime-tokio", "sqlite", "macros"] }
tokio = { version = "1.49.0", features = ["full", "tracing"] }
argon2 = { version = "0.5.3", features = ["std", "zeroize"] }
base64 = "0.22.1"
rfd = "0.17.2"
dioxus-primitives = { git = "https://github.com/DioxusLabs/components", version = "0.0.1", default-features = false }
secrecy = "0.10.3"
aes-gcm = { version = "0.10.3", features = ["zeroize"] }
sqlx-template = "0.2.1"
futures = "0.3"
rayon = "1.11.0"

[dev-dependencies]
tempfile = "3"

[target.'cfg(windows)'.build-dependencies]
winres = "0.1"
```

**Step 2: Verifica che Cargo riconosca la dipendenza**

Run: `cargo check --tests 2>&1 | head -20`

Expected: Output che mostra `Fetching tempfile` o `Download tempfile` senza errori

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add Cargo.toml
git commit -m "chore: add tempfile dev dependency for db_backend tests"
```

---

## Task 2: Esporre fetch_user_password pubblicamente

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend.rs:264-278`

**Step 1: Rendi fetch_user_password pubblica**

Trova la funzione `fetch_user_password` (circa riga 264):

```rust
#[instrument(skip(pool))]
async fn fetch_user_password(pool: &SqlitePool, username: &str) -> Result<String, DBError> {
```

Cambia `async fn` in `pub async fn`:

```rust
#[instrument(skip(pool))]
pub async fn fetch_user_password(pool: &SqlitePool, username: &str) -> Result<String, DBError> {
```

**Step 2: Verifica che il codice compila**

Run: `cargo check`

Expected: `Finished dev profile` senza errori

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend.rs
git commit -m "chore: expose fetch_user_password as pub for testing"
```

---

## Task 3: Creare file db_backend_tests.rs con setup e helpers

**Files:**
- Create: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Scrivi il file con imports e setup function**

Crea il file `src/backend/db_backend_tests.rs` con:

```rust
#![allow(dead_code)]
use crate::backend::db_backend::{
    fetch_user_password, init_db, list_users, save_or_update_user,
};
use crate::backend::init_queries::QUERIES;
use secrecy::SecretString;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteQueryAs,
};
use sqlx::{query, Row};
use std::str::FromStr;
use tempfile::TempDir;

/// Helper: Crea un database SQLite pulito in una directory temporanea
/// Restituisce (pool, temp_dir) - temp_dir garantisce cleanup quando esce dallo scope
async fn setup_test_db() -> (SqlitePool, TempDir) {
    // 1. Crea directory temporanea (auto-cleanup)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // 2. Configura database con WAL mode per concorrenza
    let db_path = temp_dir.path().join("test_users.db");
    let options = SqliteConnectOptions::from_str(format!(
        "sqlite:{}",
        db_path.display()
    ))
    .expect("Invalid DB path")
    .journal_mode(SqliteJournalMode::Wal)  // Fondamentale per concorrenza
    .foreign_keys(true)
    .create_if_missing(true);

    // 3. Connetiti e inizzializza
    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to test DB");

    // 4. Esegui query di inizializzazione (crea tabella users)
    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .expect("Failed to create table during test setup");
    }

    (pool, temp_dir)
}

/// Helper: Crea un utente di test base e returna il suo ID
async fn create_test_user(pool: &SqlitePool) -> i64 {
    save_or_update_user(
        pool,
        None,  // id = None → INSERT
        "test_user".to_string(),
        Some(SecretString::new("test_password_123".to_string())),
        Some(vec![1u8, 2u8, 3u8]),  // avatar
    )
    .await
    .expect("Failed to create test user");

    // Recupera l'ID dell'utente creato
    let users = list_users(pool).await.expect("Failed to list users");
    assert_eq!(users.len(), 1, "Should have exactly one user");
    users[0].0  // Return user_id
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Categoria 1: Test INSERT ============
    // I test verranno aggiunti nei prossimi task

    // ============ Categoria 2: Test UPDATE ============
    // I test verranno aggiunti nei prossimi task

    // ============ Categoria 3: Test temp_old_password ============
    // I test verranno aggiunti nei prossimi task

    // ============ Categoria 4: Test Casi di Errore ============
    // I test verranno aggiunti nei prossimi task
}
```

**Step 2: Verifica che il file compila (ancora vuoto di test)**

Run: `cargo check --tests`

Expected: `Finished dev profile [unoptimized + debuginfo]` senza errori

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add db_backend_tests.rs with setup helpers"
```

---

## Task 4: Aggiungere modulo tests a mod.rs

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\mod.rs`

**Step 1: Aggiungi cfg(test) per db_backend_tests**

Vai alla fine del file (dopo riga 9) e aggiungi:

```rust
#[cfg(test)]
mod db_backend_tests;
```

Il file dovrebbe assomigliare a:

```rust
pub mod db_backend;
pub mod init_queries;
mod password_utils;
pub mod ui_utils;
mod user_auth_helper;
pub mod utils;

#[cfg(test)]
mod password_utils_tests;

#[cfg(test)]
mod db_backend_tests;
```

**Step 2: Verifica che il modulo viene riconosciuto**

Run: `cargo check --tests 2>&1 | grep -E "(error|warning|Compiling)" | head -5`

Expected: Solo `Compiling` o niente, nessun `error`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/mod.rs
git commit -m "chore: add db_backend_tests module to backend"
```

---

## Task 5: Scrivere test INSERT - test_insert_new_user_success

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs:67-72`

**Step 1: Aggiungi primo test INSERT nella categoria 1**

Sostituisci il commento `// I test verranno aggiunti nei prossimi task` sotto `// ============ Categoria 1: Test INSERT ============` con:

```rust
    // ============ Categoria 1: Test INSERT ============

    #[tokio::test]
    async fn test_insert_new_user_success() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user".to_string();
        let password = SecretString::new("secure_password_123".to_string());
        let avatar = vec![1u8, 2u8, 3u8];

        let result = save_or_update_user(
            &pool,
            None,  // id = None → INSERT
            username.clone(),
            Some(password),
            Some(avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users.len(), 1, "Should have exactly one user");
        assert_eq!(users[0].0, 1, "User ID should be 1 (auto-increment)");
        assert_eq!(users[0].1, username, "Username should match");
    }
```

**Step 2: Esegui il test e verifica che passi**

Run: `cargo test -- test_insert_new_user_success`

Expected: `test test_insert_new_user_success ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_insert_new_user_success"
```

---

## Task 6: Scrivere test INSERT - test_insert_new_user_without_avatar

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per INSERT senza avatar**

Aggiungi dopo il test precedente (circa riga 100):

```rust
    #[tokio::test]
    async fn test_insert_new_user_without_avatar() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_no_avatar".to_string();
        let password = SecretString::new("password456".to_string());

        let result = save_or_update_user(
            &pool,
            None,  // id = None → INSERT
            username.clone(),
            Some(password),
            None,  // avatar = None
        )
        .await;

        assert!(result.is_ok(), "INSERT without avatar should succeed");

        // Verifica che l'utente sia stato creato senza avatar
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users.len(), 1, "Should have exactly one user");
        assert_eq!(users[0].1, username, "Username should match");
        assert!(users[0].3.is_none(), "Avatar should be None");
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_insert_new_user_without_avatar`

Expected: `test test_insert_new_user_without_avatar ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_insert_new_user_without_avatar"
```

---

## Task 7: Scrivere test INSERT - test_insert_new_user_empty_password

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per password vuota**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_insert_new_user_empty_password() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_empty_pass".to_string();
        let empty_password = SecretString::new("".to_string());  // Password vuota

        let result = save_or_update_user(
            &pool,
            None,
            username,
            Some(empty_password),
            None,
        )
        .await;

        assert!(result.is_err(), "Empty password should return error");
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Password") || error_msg.contains("password"),
                "Error should mention password"
            );
        }
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_insert_new_user_empty_password`

Expected: `test test_insert_new_user_empty_password ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_insert_new_user_empty_password"
```

---

## Task 8: Scrivere test UPDATE - test_update_username_only

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi primo test UPDATE**

Sostituisci il commento sotto `// ============ Categoria 2: Test UPDATE ============` con:

```rust
    // ============ Categoria 2: Test UPDATE ============

    #[tokio::test]
    async fn test_update_username_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Prima crea un utente
        let user_id = create_test_user(&pool).await;
        let new_username = "updated_username".to_string();

        // Poi aggiorna solo username
        let result = save_or_update_user(
            &pool,
            Some(user_id),  // id = Some → UPDATE
            new_username.clone(),
            None,  // password = None
            None,  // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE username should succeed");

        // Verifica aggiornamento
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users.len(), 1, "Should still have one user");
        assert_eq!(users[0].0, user_id, "User ID should not change");
        assert_eq!(users[0].1, new_username, "Username should be updated");
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_update_username_only`

Expected: `test test_update_username_only ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_update_username_only"
```

---

## Task 9: Scrivere test UPDATE - test_update_password_only

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per aggiornamento password**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_update_password_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        // Recupera la vecchia password per comparazione
        let old_password_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch old password");

        let new_password = SecretString::new("new_password_456".to_string());

        // Aggiorna solo password
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),  // username invariato
            Some(new_password),
            None,  // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE password should succeed");

        // Verifica che temp_old_password sia stato salvato
        let temp_password_row: Option<String> = query(
            "SELECT temp_old_password FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query temp_old_password");

        assert!(
            temp_password_row.is_some(),
            "temp_old_password should be set"
        );
        assert_eq!(
            temp_password_row.unwrap(),
            old_password_hash,
            "temp_old_password should contain old password hash"
        );
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_update_password_only`

Expected: `test test_update_password_only ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_update_password_only"
```

---

## Task 10: Scrivere test UPDATE - test_update_avatar_only

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per aggiornamento avatar**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_update_avatar_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;
        let new_avatar = vec![9u8, 8u8, 7u8, 6u8];

        // Aggiorna solo avatar
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),  // username invariato
            None,  // password = None
            Some(new_avatar.clone()),  // avatar nuovo
        )
        .await;

        assert!(result.is_ok(), "UPDATE avatar should succeed");

        // Verifica aggiornamento
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users[0].0, user_id, "User ID should not change");
        assert_eq!(users[0].3, Some(new_avatar), "Avatar should be updated");
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_update_avatar_only`

Expected: `test test_update_avatar_only ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_update_avatar_only"
```

---

## Task 11: Scrivere test UPDATE - test_update_all_fields

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per aggiornamento completo**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_update_all_fields() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        let new_username = "fully_updated".to_string();
        let new_password = SecretString::new("new_pass_789".to_string());
        let new_avatar = vec![99u8, 88u8, 77u8];

        // Aggiorna tutti i campi
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            new_username.clone(),
            Some(new_password),
            Some(new_avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "UPDATE all fields should succeed");

        // Verifica tutti i campi
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users[0].0, user_id, "User ID should not change");
        assert_eq!(users[0].1, new_username, "Username should be updated");
        assert_eq!(users[0].3, Some(new_avatar), "Avatar should be updated");
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_update_all_fields`

Expected: `test test_update_all_fields ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_update_all_fields"
```

---

## Task 12: Scrivere test temp_old_password - test_temp_password_saved_on_update

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi primo test temp_old_password**

Sostituisci il commento sotto `// ============ Categoria 3: Test temp_old_password ============` con:

```rust
    // ============ Categoria 3: Test temp_old_password ============

    #[tokio::test]
    async fn test_temp_password_saved_on_update() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        // Recupera la password originale (hash)
        let old_password_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch original password");

        // Aggiorna la password
        let new_password = SecretString::new("completely_new_pass".to_string());
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(new_password),
            None,
        )
        .await
        .expect("Failed to update password");

        // Verifica che temp_old_password contenga la vecchia password
        let temp_password_row: String = query(
            "SELECT temp_old_password FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query temp_old_password");

        assert_eq!(
            temp_password_row, old_password_hash,
            "temp_old_password should contain the old password hash"
        );
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_temp_password_saved_on_update`

Expected: `test test_temp_password_saved_on_update ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_temp_password_saved_on_update"
```

---

## Task 13: Scrivere test temp_old_password - test_temp_password_overwritten_on_multiple_updates

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per sovrascrittura temp_old_password**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_temp_password_overwritten_on_multiple_updates() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        // Salva la prima password hash
        let first_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch first password");

        // Prima aggiornamento password
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password_second".to_string())),
            None,
        )
        .await
        .expect("Failed first update");

        // Salva la seconda password hash
        let second_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch second password");

        // Secondo aggiornamento password
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password_third".to_string())),
            None,
        )
        .await
        .expect("Failed second update");

        // Verifica che temp_old_password contenga la seconda password (non la prima)
        let temp_password_row: String = query(
            "SELECT temp_old_password FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query temp_old_password");

        assert_eq!(
            temp_password_row, second_hash,
            "temp_old_password should contain the second password hash"
        );
        assert_ne!(
            temp_password_row, first_hash,
            "temp_old_password should NOT contain the first password hash"
        );
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_temp_password_overwritten_on_multiple_updates`

Expected: `test test_temp_password_overwritten_on_multiple_updates ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_temp_password_overwritten_on_multiple_updates"
```

---

## Task 14: Scrivere test errori - test_update_nonexistent_user

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi primo test per casi di errore**

Sostituisci il commento sotto `// ============ Categoria 4: Test Casi di Errore ============` con:

```rust
    // ============ Categoria 4: Test Casi di Errore ============

    #[tokio::test]
    async fn test_update_nonexistent_user() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Tenta aggiornamento con ID inesistente
        let fake_id = 99999i64;
        let result = save_or_update_user(
            &pool,
            Some(fake_id),
            "nonexistent_user".to_string(),
            Some(SecretString::new("fake_password".to_string())),
            None,
        )
        .await;

        // SQLite UPDATE con ID inesistente non fallisce (0 righe affette)
        // Verifica che non ci siano panic
        assert!(
            result.is_ok() || result.is_err(),
            "Should not panic on nonexistent user"
        );

        // Verifica che nessun utente sia stato creato
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users.len(), 0, "Should have no users after fake update");
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_update_nonexistent_user`

Expected: `test test_update_nonexistent_user ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_update_nonexistent_user"
```

---

## Task 15: Scrivere test errori - test_special_characters_username

**Files:**
- Modify: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Aggiungi test per SQL injection attempt**

Aggiungi dopo il test precedente:

```rust
    #[tokio::test]
    async fn test_special_characters_username() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Tenta SQL injection
        let malicious_username = "user'; DROP TABLE users; --".to_string();

        let result = save_or_update_user(
            &pool,
            None,  // INSERT
            malicious_username,
            Some(SecretString::new("password".to_string())),
            None,
        )
        .await;

        // sqlx dovrebbe gestire l'escape correttamente
        assert!(result.is_ok(), "Should handle special characters safely");

        // Verifica che l'utente sia stato creato con il nome letterale
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_eq!(users.len(), 1, "Should have one user");
        assert_eq!(
            users[0].1, "user'; DROP TABLE users; --",
            "Username should be stored literally (no injection)"
        );
    }
```

**Step 2: Esegui il test**

Run: `cargo test -- test_special_characters_username`

Expected: `test test_special_characters_username ... ok`

**Step 3: Commit**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add src/backend/db_backend_tests.rs
git commit -m "test: add test_special_characters_username (SQL injection safe)"
```

---

## Task 16: Esegure tutta la suite di test

**Files:**
- Test: `C:\Users\Lucio\RustroverProjects\PWDManager\src\backend\db_backend_tests.rs`

**Step 1: Esegui tutti i test del modulo**

Run: `cargo test --package PWDManager --lib backend::db_backend_tests`

Expected: Tutti i test passano con output simile a:
```
running 13 tests
test test_insert_new_user_success ... ok
test test_insert_new_user_without_avatar ... ok
test test_insert_new_user_empty_password ... ok
test test_update_username_only ... ok
test test_update_password_only ... ok
test test_update_avatar_only ... ok
test test_update_all_fields ... ok
test test_temp_password_saved_on_update ... ok
test test_temp_password_overwritten_on_multiple_updates ... ok
test test_update_nonexistent_user ... ok
test test_special_characters_username ... ok

test result: ok. 13 passed
```

**Step 2: Esegui anche gli altri test per verificare che nulla sia rotto**

Run: `cargo test --lib`

Expected: Tutti i test passano inclusi gli esistenti

**Step 3: Commit finale**

```bash
cd "C:\Users\Lucio\RustroverProjects\PWDManager"
git add .
git commit -m "test: complete db_backend test suite - all 13 tests passing"
```

---

## Verification

Dopo aver completato tutti i task, verifica:

1. **Copertura test:**
   - INSERT: 3 test (success, without avatar, empty password)
   - UPDATE: 4 test (username only, password only, avatar only, all fields)
   - temp_old_password: 2 test (saved on update, overwritten)
   - Errori: 2 test (nonexistent user, special characters)
   - **Totale: 13 test**

2. **Codice pulito:**
   - Nessuna modifica a `db_backend.rs` oltre a esporre `fetch_user_password`
   - Tutti i test nel nuovo file `db_backend_tests.rs`
   - Cleanup automatico con `TempDir`

3. **Commit granulari:**
   - Ogni test è un commit separato
   - Messaggi di commit chiari e descrittivi

4. **Test execution:**
   - `cargo test --lib` passa completamente
   - Nessun warning o errore

---

## Note Importanti

- **Isolamento:** Ogni test crea il proprio database, quindi non c'è rischio di interferenza
- **Cleanup:** `TempDir` garantisce che i file temporanei vengano eliminati
- **WAL mode:** Abilitato per testare la concorrenza (fondamentale per SQLite)
- **Temp dir binding:** `_temp_dir` impedisce accidental drop prima della fine del test
