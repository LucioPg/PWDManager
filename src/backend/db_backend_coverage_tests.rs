// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#![allow(dead_code)]
use crate::backend::db_backend::{
    UserUpdate, build_sqlcipher_options, check_user, create_user_settings,
    delete_all_user_stored_passwords, delete_stored_password, delete_user,
    fetch_all_passwords_for_user_with_filter, fetch_all_stored_passwords_for_user,
    fetch_diceware_settings, fetch_password_stats, fetch_passwords_paginated, fetch_user_data,
    fetch_user_password, fetch_user_passwords_generation_settings, fetch_user_settings,
    register_user_with_settings, remove_temp_old_password, restore_old_password,
    save_or_update_user, upsert_diceware_settings, upsert_password_config,
    upsert_stored_passwords_batch,
};
use crate::backend::password_utils::create_stored_data_pipeline_bulk;
use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
use custom_errors::AuthError;
use pwd_types::{PasswordPreset, PasswordStrength, StoredRawPassword};
use secrecy::SecretString;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Helper: insert N stored passwords for a user with controlled scores.
async fn insert_test_passwords(
    pool: &SqlitePool,
    user_id: i64,
    entries: Vec<(&str, &str)>, // (url, raw_password) pairs
) {
    let _ = pwd_strength::init_blacklist();
    let (vault_id, _) = create_test_vault(pool, user_id).await;
    let raw_passwords: Vec<StoredRawPassword> = entries
        .into_iter()
        .map(|(url, pwd)| {
            let strength =
                crate::backend::evaluate_password_strength(&SecretString::new(pwd.into()), None);
            StoredRawPassword {
                uuid: Uuid::new_v4(),
                id: None,
                user_id,
                vault_id,
                name: String::new(),
                username: SecretString::new(String::new().into()),
                url: SecretString::new(url.into()),
                password: SecretString::new(pwd.into()),
                notes: None,
                score: strength.score,
                created_at: None,
            }
        })
        .collect();
    create_stored_data_pipeline_bulk(pool, user_id, raw_passwords)
        .await
        .expect("Failed to insert test passwords");
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // === Unit Tests: UserUpdate, get_db_path, build_sqlcipher_options ===
    // =========================================================================

    #[test]
    fn test_user_update_has_updates_all_fields() {
        let update = UserUpdate {
            username: Some("user".into()),
            password: Some(SecretString::new("pass".into())),
            avatar: Some(vec![1, 2, 3]),
            temp_old_password: Some("old".into()),
        };
        assert!(update.has_updates());
    }

    #[test]
    fn test_user_update_has_updates_no_fields() {
        let update = UserUpdate {
            username: None,
            password: None,
            avatar: None,
            temp_old_password: None,
        };
        assert!(!update.has_updates());
    }

    #[test]
    fn test_user_update_has_updates_temp_only() {
        // temp_old_password is NOT counted as an update field
        let update = UserUpdate {
            username: None,
            password: None,
            avatar: None,
            temp_old_password: Some("old_hash".into()),
        };
        assert!(!update.has_updates());
    }

    #[test]
    fn test_user_update_build_sql_fields_all() {
        let update = UserUpdate {
            username: Some("user".into()),
            password: Some(SecretString::new("p".into())),
            avatar: Some(vec![]),
            temp_old_password: Some("old".into()),
        };
        let fields = update.build_sql_fields();
        assert_eq!(fields.len(), 4);
        assert!(fields.contains(&"username = ?"));
        assert!(fields.contains(&"password = ?"));
        assert!(fields.contains(&"avatar = ?"));
        assert!(fields.contains(&"temp_old_password = ?"));
    }

    #[test]
    fn test_user_update_build_sql_fields_partial() {
        let update = UserUpdate {
            username: Some("user".into()),
            password: None,
            avatar: Some(vec![]),
            temp_old_password: None,
        };
        let fields = update.build_sql_fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"username = ?"));
        assert!(fields.contains(&"avatar = ?"));
    }

    #[test]
    fn test_user_update_build_sql_fields_none() {
        let update = UserUpdate {
            username: None,
            password: None,
            avatar: None,
            temp_old_password: None,
        };
        assert!(update.build_sql_fields().is_empty());
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn test_get_db_path_returns_string() {
        let result = crate::backend::db_backend::get_db_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.ends_with("database.db"));
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn test_build_sqlcipher_options_returns_ok() {
        let result = build_sqlcipher_options(
            "/tmp/test.db",
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        );
        assert!(result.is_ok());
    }

    // =========================================================================
    // === User CRUD: delete_user ===
    // =========================================================================

    #[tokio::test]
    async fn test_delete_user_success() {
        let pool = setup_test_db().await;
        let (user_id, username) = create_test_user(&pool, "del_user", "password123", None).await;

        delete_user(&pool, user_id)
            .await
            .expect("delete should succeed");

        let result = fetch_user_data(&pool, &username).await;
        assert!(result.is_err(), "User should be deleted");
    }

    #[tokio::test]
    async fn test_delete_user_cascade_passwords() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "del_cascade", "password123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![("https://site1.com", "abc"), ("https://site2.com", "def")],
        )
        .await;

        delete_user(&pool, user_id)
            .await
            .expect("delete should succeed");

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert!(passwords.is_empty(), "Passwords should be cascade-deleted");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let pool = setup_test_db().await;
        // SQLite DELETE with no matching row returns 0 rows affected, no error
        let result = delete_user(&pool, 99999).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // === Authentication: check_user ===
    // =========================================================================

    #[tokio::test]
    async fn test_check_user_correct_password() {
        let pool = setup_test_db().await;
        let (_, username) = create_test_user(&pool, "check_ok", "MyPassword123!", None).await;

        let result = check_user(
            &pool,
            &username,
            &SecretString::new("MyPassword123!".into()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_user_wrong_password() {
        let pool = setup_test_db().await;
        let (_, username) = create_test_user(&pool, "check_wrong", "CorrectPassword!", None).await;

        let result = check_user(
            &pool,
            &username,
            &SecretString::new("WrongPassword!".into()),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AuthError::Decryption(_) => {} // expected
            other => panic!("Expected AuthError::Decryption, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_check_user_nonexistent_user() {
        let pool = setup_test_db().await;

        let result = check_user(
            &pool,
            "nonexistent_user_xyz",
            &SecretString::new("any".into()),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AuthError::DB(_) => {} // expected
            other => panic!("Expected AuthError::DB, got {:?}", other),
        }
    }

    // =========================================================================
    // === Password Operations: fetch & delete ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_all_passwords_empty() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "fetch_empty", "pass", None).await;

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert!(passwords.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_all_passwords_returns_records() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "fetch_some", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![("https://site1.com", "abc"), ("https://site2.com", "def")],
        )
        .await;

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert_eq!(passwords.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_stored_password() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "del_pwd", "pass123", None).await;

        insert_test_passwords(&pool, user_id, vec![("https://site.com", "abc")]).await;

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert_eq!(passwords.len(), 1);
        let pwd_id = passwords[0].id.unwrap();

        delete_stored_password(&pool, pwd_id)
            .await
            .expect("delete should succeed");

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert!(passwords.is_empty());
    }

    #[tokio::test]
    async fn test_delete_all_user_stored_passwords() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "del_all", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://site1.com", "abc"),
                ("https://site2.com", "def"),
                ("https://site3.com", "ghi"),
            ],
        )
        .await;

        delete_all_user_stored_passwords(&pool, user_id)
            .await
            .expect("delete all should succeed");

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert!(passwords.is_empty());
    }

    // =========================================================================
    // === Pagination & Filtering ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_passwords_paginated_no_filter() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "page_nofilter", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://s1.com", "abc"),
                ("https://s2.com", "def"),
                ("https://s3.com", "ghi"),
                ("https://s4.com", "jkl"),
                ("https://s5.com", "mno"),
            ],
        )
        .await;

        let (results, total) = fetch_passwords_paginated(&pool, user_id, None, 0, 2)
            .await
            .expect("paginated fetch should succeed");
        assert_eq!(results.len(), 2, "First page should have 2 items");
        assert_eq!(total, 5, "Total should be 5");

        let (page1, total) = fetch_passwords_paginated(&pool, user_id, None, 1, 2)
            .await
            .expect("page 1 should succeed");
        assert_eq!(page1.len(), 2, "Second page should have 2 items");
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_fetch_passwords_paginated_filter_strength() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "page_filter", "pass123", None).await;

        // ciaociao = weak, Password2! = medium, VeryLongSecurePassword123! = epic/god
        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://weak.com", "ciaociao"),
                ("https://medium.com", "Password2!"),
                ("https://strong.com", "VeryLongSecurePassword123!"),
            ],
        )
        .await;

        let (weak, _) =
            fetch_passwords_paginated(&pool, user_id, Some(PasswordStrength::WEAK), 0, 10)
                .await
                .expect("weak filter should succeed");
        assert!(weak.len() >= 1, "Should find at least 1 weak password");

        let (no_match, total) =
            fetch_passwords_paginated(&pool, user_id, Some(PasswordStrength::GOD), 0, 10)
                .await
                .expect("GOD filter should succeed");
        assert_eq!(total, 0, "Should find 0 GOD passwords");
        assert!(no_match.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_all_passwords_filter_no_filter() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "allfilter_no", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://a.com", "abc"),
                ("https://b.com", "def"),
                ("https://c.com", "ghi"),
            ],
        )
        .await;

        let results =
            fetch_all_passwords_for_user_with_filter(&pool, user_id, None, "created_at DESC")
                .await
                .expect("fetch all no filter should succeed");
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_fetch_all_passwords_filter_by_strength() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "allfilter_str", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://weak.com", "ciaociao"),
                ("https://medium.com", "Password2!"),
                ("https://strong.com", "VeryLongSecurePassword123!"),
            ],
        )
        .await;

        let weak = fetch_all_passwords_for_user_with_filter(
            &pool,
            user_id,
            Some(PasswordStrength::WEAK),
            "created_at DESC",
        )
        .await
        .expect("weak filter should succeed");
        assert!(weak.len() >= 1, "Should find at least 1 weak password");
    }

    #[tokio::test]
    async fn test_fetch_all_passwords_order_asc() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "allfilter_ord", "pass123", None).await;

        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://first.com", "abc"),
                ("https://second.com", "def"),
                ("https://third.com", "ghi"),
            ],
        )
        .await;

        let results =
            fetch_all_passwords_for_user_with_filter(&pool, user_id, None, "created_at ASC")
                .await
                .expect("ASC order should succeed");
        assert_eq!(results.len(), 3);
        // First result should have the earliest created_at
        let first_id = results[0].id.unwrap();
        let last_id = results[2].id.unwrap();
        assert!(
            first_id < last_id,
            "ASC order: first id should be less than last id"
        );
    }

    // =========================================================================
    // === Password Stats ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_password_stats_empty() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "stats_empty", "pass123", None).await;

        let stats = fetch_password_stats(&pool, user_id)
            .await
            .expect("stats should succeed");
        assert_eq!(stats.weak, 0);
        assert_eq!(stats.medium, 0);
        assert_eq!(stats.strong, 0);
        assert_eq!(stats.epic, 0);
        assert_eq!(stats.god, 0);
        assert_eq!(stats.total, 0);
    }

    #[tokio::test]
    async fn test_fetch_password_stats_mixed() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "stats_mixed", "pass123", None).await;

        // ciaociao = weak, Password2! = medium
        insert_test_passwords(
            &pool,
            user_id,
            vec![
                ("https://weak1.com", "ciaociao"),
                ("https://weak2.com", "ciaociao"),
                ("https://medium.com", "Password2!"),
            ],
        )
        .await;

        let stats = fetch_password_stats(&pool, user_id)
            .await
            .expect("stats should succeed");
        assert!(stats.weak >= 2, "Should have at least 2 weak passwords");
        assert!(stats.medium >= 1, "Should have at least 1 medium password");
        assert_eq!(
            stats.total,
            stats.weak + stats.medium + stats.strong + stats.epic + stats.god
        );
    }

    // =========================================================================
    // === User Settings ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_user_settings_none() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "settings_none", "pass123", None).await;

        let settings = fetch_user_settings(&pool, user_id)
            .await
            .expect("fetch settings should succeed");
        assert!(settings.is_none());
    }

    #[tokio::test]
    async fn test_fetch_user_settings_after_registration() {
        let pool = setup_test_db().await;
        let (_, username) = create_test_user(&pool, "settings_reg", "pass123", None).await;

        let user_id = register_user_with_settings(
            &pool,
            format!("reg_settings_{}", username), // unique username
            Some(SecretString::new("RegPass123!".into())),
            None,
            PasswordPreset::Strong,
        )
        .await
        .expect("registration should succeed");

        let settings = fetch_user_settings(&pool, user_id)
            .await
            .expect("fetch settings should succeed");
        assert!(settings.is_some());
        let s = settings.unwrap();
        assert_eq!(s.user_id, user_id);
    }

    #[tokio::test]
    async fn test_upsert_password_config_update() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "upsert_cfg", "pass123", None).await;

        let settings_id = create_user_settings(&pool, user_id, PasswordPreset::God)
            .await
            .expect("create settings should succeed");

        let mut config = fetch_user_passwords_generation_settings(&pool, user_id)
            .await
            .expect("fetch config should succeed");
        assert_eq!(config.id, Some(settings_id));

        // Modify and upsert
        config.length = 20;
        upsert_password_config(&pool, config.clone())
            .await
            .expect("upsert should succeed");

        let updated = fetch_user_passwords_generation_settings(&pool, user_id)
            .await
            .expect("fetch updated config should succeed");
        assert_eq!(updated.length, 20);
    }

    // =========================================================================
    // === Diceware Settings ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_diceware_settings_default() {
        let pool = setup_test_db().await;
        let (_, username) = create_test_user(&pool, "diceware_fetch", "pass123", None).await;

        let user_id = register_user_with_settings(
            &pool,
            format!("diceware_reg_{}", username),
            Some(SecretString::new("DicewarePass123!".into())),
            None,
            PasswordPreset::God,
        )
        .await
        .expect("registration should succeed");

        let settings = fetch_diceware_settings(&pool, user_id)
            .await
            .expect("fetch diceware settings should succeed");
        assert_eq!(settings.word_count, 6);
        assert_eq!(settings.add_special_char, false);
        assert_eq!(settings.numbers, 0);
    }

    #[tokio::test]
    async fn test_upsert_diceware_settings_update() {
        let pool = setup_test_db().await;
        let (_, username) = create_test_user(&pool, "diceware_up", "pass123", None).await;

        let user_id = register_user_with_settings(
            &pool,
            format!("diceware_upsert_{}", username),
            Some(SecretString::new("DicewarePass123!".into())),
            None,
            PasswordPreset::God,
        )
        .await
        .expect("registration should succeed");

        let mut settings = fetch_diceware_settings(&pool, user_id)
            .await
            .expect("fetch diceware settings should succeed");
        settings.word_count = 8;
        settings.add_special_char = true;

        upsert_diceware_settings(&pool, settings.clone())
            .await
            .expect("upsert diceware should succeed");

        let updated = fetch_diceware_settings(&pool, user_id)
            .await
            .expect("fetch updated diceware settings should succeed");
        assert_eq!(updated.word_count, 8);
        assert_eq!(updated.add_special_char, true);
    }

    // =========================================================================
    // === Batch Upsert Validation ===
    // =========================================================================

    #[tokio::test]
    async fn test_upsert_stored_passwords_batch_empty() {
        let pool = setup_test_db().await;

        let result = upsert_stored_passwords_batch(&pool, vec![]).await;
        assert!(result.is_ok(), "Empty batch should succeed");
    }

    #[tokio::test]
    async fn test_upsert_stored_passwords_batch_valid() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "batch_valid", "pass123", None).await;

        // Insert passwords via pipeline first, then batch-upsert them back
        insert_test_passwords(
            &pool,
            user_id,
            vec![("https://batch1.com", "abc"), ("https://batch2.com", "def")],
        )
        .await;

        let passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert_eq!(passwords.len(), 2);

        // Re-upsert the same passwords (validates the batch path)
        let result = upsert_stored_passwords_batch(&pool, passwords).await;
        assert!(result.is_ok(), "Valid batch should succeed");

        // Verify count is still 2 (upsert, not duplicate insert)
        let after = fetch_all_stored_passwords_for_user(&pool, user_id)
            .await
            .expect("fetch should succeed");
        assert_eq!(after.len(), 2);
    }

    // =========================================================================
    // === Temp Password Management ===
    // =========================================================================

    #[tokio::test]
    async fn test_remove_temp_old_password() {
        let pool = setup_test_db().await;
        let (user_id, username) = create_test_user(&pool, "rem_temp", "OldPassword1!", None).await;

        // Update password → sets temp_old_password
        save_or_update_user(
            &pool,
            Some(user_id),
            username,
            Some(SecretString::new("NewPassword2!".into())),
            None,
        )
        .await
        .expect("update should succeed");

        // Verify temp is set
        let temp = crate::backend::db_backend::fetch_user_temp_old_password(&pool, user_id)
            .await
            .expect("fetch temp should succeed");
        assert!(
            temp.is_some(),
            "temp_old_password should be set after update"
        );

        // Remove temp
        remove_temp_old_password(&pool, user_id)
            .await
            .expect("remove temp should succeed");

        let temp_after = crate::backend::db_backend::fetch_user_temp_old_password(&pool, user_id)
            .await
            .expect("fetch temp should succeed");
        assert!(
            temp_after.is_none(),
            "temp_old_password should be NULL after remove"
        );
    }

    #[tokio::test]
    async fn test_remove_temp_old_password_already_null() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "rem_temp_null", "pass123", None).await;

        // No password update → temp is NULL
        let result = remove_temp_old_password(&pool, user_id).await;
        assert!(result.is_ok(), "Remove on NULL temp should succeed");
    }

    #[tokio::test]
    async fn test_restore_old_password() {
        let pool = setup_test_db().await;
        let (user_id, username) =
            create_test_user(&pool, "restore_old", "OldPassword1!", None).await;

        let old_hash = fetch_user_password(&pool, &username)
            .await
            .expect("fetch old password should succeed");

        // Update password → sets temp_old_password to old hash
        save_or_update_user(
            &pool,
            Some(user_id),
            username.clone(),
            Some(SecretString::new("NewPassword2!".into())),
            None,
        )
        .await
        .expect("update should succeed");

        // Verify password changed
        let new_hash = fetch_user_password(&pool, &username)
            .await
            .expect("fetch new password should succeed");
        assert_ne!(old_hash, new_hash, "Password should have changed");

        // Restore old password
        restore_old_password(&pool, user_id)
            .await
            .expect("restore should succeed");

        // Verify password is back to old hash
        let restored_hash = fetch_user_password(&pool, &username)
            .await
            .expect("fetch restored password should succeed");
        assert_eq!(
            old_hash, restored_hash,
            "Password should be restored to old hash"
        );

        // Verify temp is NULL
        let temp = crate::backend::db_backend::fetch_user_temp_old_password(&pool, user_id)
            .await
            .expect("fetch temp should succeed");
        assert!(
            temp.is_none(),
            "temp_old_password should be NULL after restore"
        );
    }
}
