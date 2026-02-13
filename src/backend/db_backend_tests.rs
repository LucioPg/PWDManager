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

    // 3. Connetiti e inizializza
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
