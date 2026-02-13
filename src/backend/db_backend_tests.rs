#![allow(dead_code)]
use crate::backend::db_backend::{
    fetch_user_password, init_db, list_users, save_or_update_user,
};
use crate::backend::init_queries::QUERIES;
use secrecy::SecretString;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool,
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
    let db_path_str = format!(r"sqlite:{}", db_path.to_str().unwrap());
    let options = SqliteConnectOptions::from_str(&db_path_str)
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
        Some(SecretString::new("test_password_123".into())),
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

    #[tokio::test]
    async fn test_insert_new_user_success() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user".to_string();
        let password = SecretString::new("secure_password_123".into());
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

    #[tokio::test]
    async fn test_insert_new_user_without_avatar() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_no_avatar".to_string();
        let password = SecretString::new("password456".into());

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

    #[tokio::test]
    async fn test_insert_new_user_empty_password() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_empty_pass".to_string();
        let empty_password = SecretString::new("".into());  // Password vuota

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

    // ============ Categoria 3: Test temp_old_password ============
    // I test verranno aggiunti nei prossimi task

    // ============ Categoria 4: Test Casi di Errore ============
    // I test verranno aggiunti nei prossimi task
}
