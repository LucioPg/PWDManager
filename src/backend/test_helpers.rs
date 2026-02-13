#![allow(dead_code)]
use crate::backend::db_backend::{list_users, save_or_update_user};
use crate::backend::init_queries::QUERIES;
use secrecy::SecretString;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use sqlx::{Row, query};
use std::str::FromStr;
use tempfile::TempDir;

/// Helper: Crea un database SQLite pulito in una directory temporanea
/// Restituisce (pool, temp_dir) - temp_dir garantisce cleanup quando esce dallo scope
pub async fn setup_test_db() -> (SqlitePool, TempDir) {
    // 1. Crea directory temporanea (auto-cleanup)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // 2. Configura database con WAL mode per concorrenza
    let db_path = temp_dir.path().join("test_users.db");
    let db_path_str = format!(r"sqlite:{}", db_path.to_str().unwrap());
    let options = SqliteConnectOptions::from_str(&db_path_str)
        .expect("Invalid DB path")
        .journal_mode(SqliteJournalMode::Wal) // Fondamentale per concorrenza
        .foreign_keys(true)
        .create_if_missing(true);

    // 3. Connetti e inizializza
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

/// Helper: Crea un utente di test base e restituisce il suo ID
pub async fn create_test_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    avatar: Option<Vec<u8>>,
) -> i64 {
    save_or_update_user(
        pool,
        None, // id = None → INSERT
        username.to_string(),
        Some(SecretString::new(password.into())),
        avatar,
    )
    .await
    .expect("Failed to create test user");

    // Recupera l'ID dell'utente creato
    let users = list_users(pool).await.expect("Failed to list users");
    assert_eq!(users.len(), 1, "Should have exactly one user");
    users[0].0 // Return user_id
}

/// Helper: Verifica che il database contenga esattamente N utenti
pub fn assert_user_count(
    users: &[(i64, String, String, Option<Vec<u8>>)],
    expected: usize,
    msg: &str,
) {
    assert_eq!(users.len(), expected, "{}", msg);
}

/// Helper: Verifica che l'utente abbia username specificato
pub fn assert_username(
    users: &[(i64, String, String, Option<Vec<u8>>)],
    index: usize,
    expected: &str,
) {
    assert_eq!(users[index].1, expected, "Username should match");
}

/// Helper: Verifica che l'utente abbia avatar specificato
pub fn assert_has_avatar(
    users: &[(i64, String, String, Option<Vec<u8>>)],
    index: usize,
    should_have: bool,
) {
    assert_eq!(
        users[index].3.is_some(),
        should_have,
        "Avatar should match expected value"
    );
}
// TODO: Fix compilation error with closing delimiter - needs investigation

#[cfg(test)]
mod tests {
    // Questo modulo può contenere test per gli helper functions stessi
}
