#![allow(dead_code)]
use crate::backend::db_backend::{
    fetch_user_password, init_db, list_users, save_or_update_user,
};
use crate::backend::init_queries::QUERIES;
use secrecy::SecretString;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteRow,
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

    #[tokio::test]
    async fn test_update_password_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        // Recupera la vecchia password per comparazione
        let old_password_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch old password");

        eprintln!("DEBUG: old_password_hash length = {}", old_password_hash.len());

        let new_password = SecretString::new("new_password_456".into());

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
        let temp_password_row: Option<SqliteRow> = query(
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
        let temp_password = temp_password_row.unwrap();
        assert_eq!(
            temp_password.get::<String, _>("temp_old_password"),
            old_password_hash,
            "temp_old_password should contain old password hash"
        );
    }

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
        assert_eq!(users.len(), 1, "Should still have one user");
        assert_eq!(users[0].0, user_id, "User ID should not change");
        assert_eq!(users[0].3, Some(new_avatar), "Avatar should be updated");
    }

    #[tokio::test]
    async fn test_update_all_fields() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        let new_username = "fully_updated".to_string();
        let new_password = SecretString::new("new_pass_789".into());
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
        assert_eq!(users.len(), 1, "Should still have one user");
        assert_eq!(users[0].0, user_id, "User ID should not change");
        assert_eq!(users[0].1, new_username, "Username should be updated");
        assert_eq!(users[0].3, Some(new_avatar), "Avatar should be updated");
    }

    #[tokio::test]
    async fn test_temp_password_saved_on_update() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        // Recupera la password originale (hash)
        let old_password_hash =
            fetch_user_password(&pool, "test_user")
                .await
                .expect("Failed to fetch old password");

        // Aggiorna la password
        let new_password = SecretString::new("completely_new_pass".into());
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),  // username invariato
            Some(new_password),
            None,  // avatar = None
        )
        .await;

        // Verifica che temp_old_password contenga la vecchia password
        let temp_password_row: Option<SqliteRow> = query(
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
        let temp_password = temp_password_row.unwrap();
        assert_eq!(
            temp_password.get::<String, _>("temp_old_password"),
            old_password_hash,
            "temp_old_password should contain old password hash"
        );
    }

    // ============ Categoria 3: Test temp_old_password ============
    // I test verranno aggiunti nei prossimi task

    // ============ Categoria 4: Test Casi di Errore ============
    // I test verranno aggiunti nei prossimi task
}
