// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#![allow(dead_code)]
use crate::backend::db_backend::{
    fetch_user_data, fetch_user_password, fetch_user_temp_old_password, save_or_update_user,
};
use crate::backend::test_helpers::{create_test_user, setup_test_db};
use secrecy::SecretString;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, query};

#[cfg(test)]
mod tests {
    use super::*;

    // Helper per verificare che un utente esista per ID
    async fn get_user_by_id(pool: &sqlx::SqlitePool, user_id: i64) -> Option<SqliteRow> {
        query("SELECT id, username, avatar FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .expect("Failed to query user")
    }

    // ============ Categoria 1: Test INSERT ============

    #[tokio::test]
    async fn test_insert_new_user_success() {
        let pool = setup_test_db().await;

        // Genera username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_{}", timestamp);

        let password = SecretString::new("secure_password_123".into());
        let avatar = vec![1u8, 2u8, 3u8];

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.clone(),
            Some(password),
            Some(avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database tramite fetch_user_data
        let user = fetch_user_data(&pool, &username).await;
        assert!(user.is_ok(), "User should exist in database");
    }

    #[tokio::test]
    async fn test_insert_new_user_without_avatar() {
        let pool = setup_test_db().await;

        // Genera username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_no_avatar_{}", timestamp);

        let password = SecretString::new("password456".into());

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.clone(),
            Some(password),
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "INSERT without avatar should succeed");

        // Verifica che l'utente sia stato creato senza avatar
        let (user_id, _, _, avatar) = fetch_user_data(&pool, &username)
            .await
            .expect("Failed to fetch user");
        assert!(avatar.is_none(), "User should not have avatar");
    }

    #[tokio::test]
    async fn test_insert_new_user_empty_password() {
        let pool = setup_test_db().await;

        // Genera username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_empty_pass_{}", timestamp);

        let empty_password = SecretString::new("".into()); // Password vuota

        let result = save_or_update_user(&pool, None, username, Some(empty_password), None).await;

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
        let pool = setup_test_db().await;

        // Crea un utente con username univoco
        let (user_id, _original_username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Genera nuovo username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let new_username = format!("updated_username_{}", timestamp);

        // Aggiorna solo username
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            new_username.clone(),
            None, // password = None
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE username should succeed");

        // Verifica aggiornamento cercando con il nuovo username
        let user = fetch_user_data(&pool, &new_username)
            .await
            .expect("Should find user with new username");
        assert_eq!(user.0, user_id, "User ID should be unchanged");
    }

    #[tokio::test]
    async fn test_update_password_only() {
        let pool = setup_test_db().await;

        let (user_id, username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Recupera la vecchia password per comparazione
        let old_password_hash = fetch_user_password(&pool, &username)
            .await
            .expect("Failed to fetch old password");

        let new_password = SecretString::new("new_password_456".into());

        // Aggiorna solo password
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            username.clone(),
            Some(new_password),
            None, // avatar = None
        )
        .await;

        assert!(result.is_ok(), "UPDATE password should succeed");

        // Verifica che temp_old_password sia stato salvato
        // let temp_password_row: Option<SqliteRow> =
        //     query("SELECT temp_old_password FROM users WHERE id = ?")
        //         .bind(user_id)
        //         .fetch_optional(&pool)
        //         .await
        //         .expect("Failed to query temp_old_password");
        let temp_password_row = fetch_user_temp_old_password(&pool, user_id)
            .await
            .expect("Failed to fetch temp_old_password");

        assert!(
            temp_password_row.is_some(),
            "temp_old_password should be set"
        );
        let temp_password = temp_password_row.unwrap();
        assert_eq!(
            // temp_password.get::<String, _>("temp_old_password"),
            temp_password,
            old_password_hash,
            "temp_old_password should contain old password hash"
        );
    }

    #[tokio::test]
    async fn test_update_avatar_only() {
        let pool = setup_test_db().await;

        let (user_id, username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;
        let new_avatar = vec![9u8, 8u8, 7u8, 6u8];

        // Aggiorna solo avatar
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            username,
            None, // password = None
            Some(new_avatar.clone()),
        )
        .await;

        assert!(result.is_ok(), "UPDATE avatar should succeed");

        // Verifica aggiornamento per ID
        let row = get_user_by_id(&pool, user_id).await;
        assert!(row.is_some(), "User should exist");
        let row = row.unwrap();
        let avatar: Option<Vec<u8>> = row.get("avatar");
        assert!(avatar.is_some(), "User should now have avatar");
    }

    #[tokio::test]
    async fn test_update_all_fields() {
        let pool = setup_test_db().await;

        let (user_id, _username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Genera nuovo username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let new_username = format!("fully_updated_{}", timestamp);
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
        let (found_id, found_username, _, found_avatar) = fetch_user_data(&pool, &new_username)
            .await
            .expect("Should find updated user");
        assert_eq!(found_id, user_id);
        assert_eq!(found_username, new_username);
        assert!(found_avatar.is_some(), "Should have avatar");
    }

    #[tokio::test]
    async fn test_temp_password_saved_on_update() {
        let pool = setup_test_db().await;

        let (user_id, username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Recupera la password originale (hash)
        let old_password_hash = fetch_user_password(&pool, &username)
            .await
            .expect("Failed to fetch old password");

        // Aggiorna la password
        let new_password = SecretString::new("completely_new_pass".into());
        let _ = save_or_update_user(&pool, Some(user_id), username, Some(new_password), None).await;

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
        let pool = setup_test_db().await;

        let (user_id, username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        // Salva la prima password hash
        let _first_hash = fetch_user_password(&pool, &username)
            .await
            .expect("Failed to fetch first password");

        // Primo aggiornamento password
        let _ = save_or_update_user(
            &pool,
            Some(user_id),
            username.clone(),
            Some(SecretString::new("password_second".into())),
            None,
        )
        .await;

        // Salva la seconda password hash
        let second_hash = fetch_user_password(&pool, &username)
            .await
            .expect("Failed to fetch second password");

        // Secondo aggiornamento password
        let _ = save_or_update_user(
            &pool,
            Some(user_id),
            username,
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
        let pool = setup_test_db().await;

        // Crea un utente
        let (user_id, _username) =
            create_test_user(&pool, "test_user", "test_password_123", None).await;

        let fake_id = 99999i64;

        // Tenta UPDATE con ID inesistente
        let result = save_or_update_user(
            &pool,
            Some(fake_id),
            "fake_user".to_string(),
            Some(SecretString::new("password456".into())),
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "UPDATE with nonexistent user should succeed"
        );

        // Verifica che l'utente originale sia ancora presente
        let row = get_user_by_id(&pool, user_id).await;
        assert!(row.is_some(), "Original user should still exist");
    }

    #[tokio::test]
    async fn test_special_characters_username() {
        let pool = setup_test_db().await;

        // Tenta SQL injection: username con caratteri speciali
        // Aggiunge timestamp per renderlo univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("user'; DROP TABLE users; --_{}", timestamp);

        let result = save_or_update_user(
            &pool,
            None, // id = None → INSERT
            username.clone(),
            Some(SecretString::new("password456".into())),
            None,
        )
        .await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database
        let user = fetch_user_data(&pool, &username).await;
        assert!(
            user.is_ok(),
            "User should exist with special characters in username"
        );
    }
}
