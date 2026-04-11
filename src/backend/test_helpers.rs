// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Helper per i test del database
//!
//! Questo modulo fornisce un singleton `SqlitePool` condiviso tra tutti i test
//! della sessione. Il database viene creato una sola volta all'inizio della
//! sessione e tutti i test condividono lo stesso file.

use crate::backend::db_backend::save_or_update_user;
use crate::backend::init_queries::QUERIES;
use crate::backend::vault_utils::create_vault;
use secrecy::SecretString;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use sqlx::query;
use std::str::FromStr;
use std::sync::OnceLock;
use tokio::sync::OnceCell;

/// Percorso predefinito per i database di test
const TEST_DB_DIR: &str = "test_dbs";

/// Nome fisso del database di test per l'intera sessione
const TEST_DB_NAME: &str = "test_session.db";

/// Singleton per il pool del database di test
/// Viene inizializzato una sola volta per sessione di test
static TEST_POOL: OnceCell<SqlitePool> = OnceCell::const_new();

/// Flag per tracciare se il cleanup iniziale è stato fatto
static CLEANUP_DONE: OnceLock<bool> = OnceLock::new();

/// Helper: Restituisce il pool singleton del database di test.
///
/// Il database viene creato una sola volta all'inizio della sessione di test.
/// Tutti i test condividono lo stesso database (stesso file).
///
/// # Cleanup
/// All'inizio di ogni sessione, il file esistente viene eliminato.
///
/// # Returns
/// `SqlitePool` - Pool di connessioni al database di test condiviso
pub async fn setup_test_db() -> SqlitePool {
    TEST_POOL
        .get_or_init(|| async {
            // Cleanup iniziale: elimina il database esistente
            do_initial_cleanup();

            // Crea il database
            create_test_db().await
        })
        .await
        .clone()
}

/// Esegue il cleanup iniziale del database di test
fn do_initial_cleanup() {
    if CLEANUP_DONE.get().is_some() {
        return;
    }

    let test_dir = get_test_db_dir();
    let db_path = test_dir.join(TEST_DB_NAME);

    if db_path.exists() {
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(&db_path);
    }

    let _ = CLEANUP_DONE.set(true);
}

/// Restituisce il percorso assoluto della directory test_dbs
fn get_test_db_dir() -> std::path::PathBuf {
    // Usa CARGO_MANIFEST_DIR per ottenere la root del progetto
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(manifest_dir).join(TEST_DB_DIR);
    path
}

/// Crea il database di test con il nome fisso per la sessione
async fn create_test_db() -> SqlitePool {
    // Crea la directory se non esiste
    let test_dir = get_test_db_dir();
    if !test_dir.exists() {
        std::fs::create_dir_all(&test_dir).expect("Failed to create test_dbs directory");
    }

    // Configura il database
    let db_path = test_dir.join(TEST_DB_NAME);
    let db_path_str = format!("sqlite:{}", db_path.to_str().unwrap());
    let options = SqliteConnectOptions::from_str(&db_path_str)
        .expect("Invalid DB path")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to test DB");

    // Esegui le query di inizializzazione
    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .expect("Failed to create table during test setup");
    }

    pool
}

/// Helper: Crea un utente di test con username univoco e restituisce il suo ID
///
/// Usa un suffisso con thread_id + timestamp per garantire username univoci
/// tra test paralleli che condividono lo stesso database.
///
/// # Returns
/// Tupla (user_id, username) dove username è quello univoco generato
pub async fn create_test_user(
    pool: &SqlitePool,
    base_username: &str,
    password: &str,
    avatar: Option<Vec<u8>>,
) -> (i64, String) {
    // Genera username univoco con thread_id + timestamp
    let thread_id = format!("{:?}", std::thread::current().id());
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let unique_username = format!("{}_{}_{}", base_username, thread_id, timestamp);

    save_or_update_user(
        pool,
        None, // id = None → INSERT
        unique_username.clone(),
        Some(SecretString::new(password.into())),
        avatar,
    )
    .await
    .expect("Failed to create test user");

    // Recupera l'ID dell'utente creato tramite username
    let (user_id, _, _, _) = crate::backend::db_backend::fetch_user_data(pool, &unique_username)
        .await
        .expect("Failed to fetch created test user");
    (user_id, unique_username)
}

/// Helper: Crea un vault di test con nome univoco e restituisce il suo ID
///
/// # Returns
/// Tupla (vault_id, vault_name) dove vault_name è quello univoco generato
pub async fn create_test_vault(pool: &SqlitePool, user_id: i64) -> (i64, String) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let vault_name = format!("TestVault_{}", timestamp);

    let vault = create_vault(pool, user_id, vault_name.clone(), None)
        .await
        .expect("Failed to create test vault");

    (vault.id.expect("Created vault should have an ID"), vault_name)
}

#[cfg(test)]
mod tests {
    // Questo modulo può contenere test per gli helper functions stessi
}
