#![allow(dead_code)]
use crate::backend::db_backend::{fetch_user_password, list_users, save_or_update_user};
use crate::backend::test_helpers::{
    assert_has_avatar, assert_user_count, assert_username, create_test_user, setup_test_db,
};
use secrecy::SecretString;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, query};

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Categoria 1: Test INSERT ============

    #[tokio::test]
    async fn test_insert_new_user_success() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user";
        let password = SecretString::new("secure_password_123".into());
        let avatar = vec![1u8, 2u8, 3u8];

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.to_string(),
            Some(password),
            Some(avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should have exactly one user");
        assert_username(&users, 0, username);
    }

    #[tokio::test]
    async fn test_insert_new_user_without_avatar() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_no_avatar";
        let password = SecretString::new("password456".into());

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.to_string(),
            Some(password),
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "INSERT without avatar should succeed");

        // Verifica che l'utente sia stato creato senza avatar
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should have exactly one user");
        assert_username(&users, 0, username);
        assert_has_avatar(&users, 0, false);
    }

    #[tokio::test]
    async fn test_insert_new_user_empty_password() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user_empty_pass";
        let empty_password = SecretString::new("".into()); // Password vuota

        let result = save_or_update_user(
            &pool,
            None,
            username.to_string(),
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
        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;
        let new_username = "updated_username";

        // Poi aggiorna solo username
        let result = save_or_update_user(
            &pool,
            Some(user_id), // id = Some → UPDATE
            new_username.to_string(),
            None, // password = None
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE username should succeed");

        // Verifica aggiornamento
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should still have one user");
        assert_username(&users, 0, new_username);
    }

    #[tokio::test]
    async fn test_update_password_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Recupera la vecchia password per comparazione
        let old_password_hash = fetch_user_password(&pool, "test_user")
            .await
            .expect("Failed to fetch old password");

        let new_password = SecretString::new("new_password_456".into());

        // Aggiorna solo password
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(), // username invariato
            Some(new_password),
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE password should succeed");

        // Verifica che temp_old_password sia stato salvato
        let temp_password_row: Option<SqliteRow> =
            query("SELECT temp_old_password FROM users WHERE id = ?")
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

        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;
        let new_avatar = vec![9u8, 8u8, 7u8, 6u8];

        // Aggiorna solo avatar
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(), // username invariato
            None,                    // password = None
            Some(new_avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "UPDATE avatar should succeed");

        // Verifica aggiornamento
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should still have one user");
        assert_has_avatar(&users, 0, true);
    }

    #[tokio::test]
    async fn test_update_all_fields() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;

        let new_username = "fully_updated";
        let new_password = SecretString::new("new_pass_789".into());
        let new_avatar = vec![99u8, 88u8, 77u8];

        // Aggiorna tutti i campi
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            new_username.to_string(),
            Some(new_password),
            Some(new_avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "UPDATE all fields should succeed");

        // Verifica tutti i campi
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should still have one user");
        assert_username(&users, 0, new_username);
        assert_has_avatar(&users, 0, true);
    }

    #[tokio::test]
    async fn test_temp_password_saved_on_update() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Recupera la password originale (hash)
        let old_password_hash = fetch_user_password(&pool, "test_user")
            .await
            .expect("Failed to fetch old password");

        // Aggiorna la password
        let new_password = SecretString::new("completely_new_pass".into());
        let _ = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(new_password),
            None,
        )
        .await;

        // Verifica che temp_old_password contenga la vecchia password
        let temp_password_row: SqliteRow =
            query("SELECT temp_old_password FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to query temp_old_password");

        assert_eq!(
            temp_password_row.get::<String, _>("temp_old_password"),
            old_password_hash,
            "temp_old_password should contain old password hash"
        );
    }

    #[tokio::test]
    async fn test_temp_password_overwritten_on_multiple_updates() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Salva la prima password hash
        let _first_hash = fetch_user_password(&pool, "test_user")
            .await
            .expect("Failed to fetch first password");

        // Primo aggiornamento password
        let _ = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password_second".into())),
            None,
        )
        .await;

        // Salva la seconda password hash
        let second_hash = fetch_user_password(&pool, "test_user")
            .await
            .expect("Failed to fetch second password");

        // Secondo aggiornamento password
        let _ = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password_third".into())),
            None,
        )
        .await;

        // Recupera temp_old_password dopo due aggiornamenti
        let temp_password_row: SqliteRow =
            query("SELECT temp_old_password FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to query temp_old_password");

        // Verifica che temp_old_password contenga la seconda password hash (non la prima!)
        assert_eq!(
            temp_password_row.get::<String, _>("temp_old_password"),
            second_hash,
            "temp_old_password should contain second password hash"
        );
    }

    #[tokio::test]
    async fn test_update_nonexistent_user() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Prima crea un utente
        create_test_user(&pool, "test_user", "test_password_123", None).await;

        let fake_id = 99999i64;

        // Tenta UPDATE con ID inesistente
        let result = save_or_update_user(
            &pool,
            Some(fake_id),
            "test_user".to_string(),
            Some(SecretString::new("password456".into())),
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "UPDATE with nonexistent user should succeed"
        );
        // Verifica che l'utente originale sia ancora presente
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should have exactly one user");
    }

    #[tokio::test]
    async fn test_special_characters_username() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Tenta SQL injection: username con caratteri speciali
        let username = "user'; DROP TABLE users; --";

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.to_string(),
            Some(SecretString::new("password456".into())),
            None,
        )
        .await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database
        let users = list_users(&pool).await.expect("Failed to list users");
        assert_user_count(&users, 1, "Should have exactly one user");
        assert_username(&users, 0, username);
    }
}
