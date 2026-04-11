// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#![allow(dead_code)]
use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
use crate::backend::vault_utils::*;
use pwd_types::StoredRawPassword;
use secrecy::SecretString;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // === Create Vault ===
    // =========================================================================

    #[tokio::test]
    async fn test_create_vault_success() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_create", "Pass123!", None).await;

        let vault = create_vault(
            &pool,
            user_id,
            "MyVault".to_string(),
            Some("Description".to_string()),
        )
        .await
        .expect("Should create vault");

        assert!(vault.id.is_some(), "Vault should have an ID");
        assert_eq!(vault.name, "MyVault");
        assert_eq!(vault.description, Some("Description".to_string()));
        assert_eq!(vault.user_id, user_id);
    }

    #[tokio::test]
    async fn test_create_vault_without_description() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_no_desc", "Pass123!", None).await;

        let vault = create_vault(&pool, user_id, "NoDesc".to_string(), None)
            .await
            .expect("Should create vault");

        assert!(vault.id.is_some());
        assert_eq!(vault.name, "NoDesc");
        assert!(vault.description.is_none(), "Description should be None");
    }

    // =========================================================================
    // === Fetch Vaults ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_vaults_by_user_empty() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_fetch_empty", "Pass123!", None).await;

        let vaults = fetch_vaults_by_user(&pool, user_id)
            .await
            .expect("Should fetch vaults");

        assert!(vaults.is_empty(), "New user should have no vaults");
    }

    #[tokio::test]
    async fn test_fetch_vaults_by_user_returns_sorted() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_sorted", "Pass123!", None).await;

        create_vault(&pool, user_id, "Charlie".to_string(), None)
            .await
            .expect("Should create vault");
        create_vault(&pool, user_id, "Alpha".to_string(), None)
            .await
            .expect("Should create vault");
        create_vault(&pool, user_id, "Bravo".to_string(), None)
            .await
            .expect("Should create vault");

        let vaults = fetch_vaults_by_user(&pool, user_id)
            .await
            .expect("Should fetch vaults");

        assert_eq!(vaults.len(), 3);
        assert_eq!(vaults[0].name, "Alpha");
        assert_eq!(vaults[1].name, "Bravo");
        assert_eq!(vaults[2].name, "Charlie");
    }

    #[tokio::test]
    async fn test_fetch_vaults_other_user_not_visible() {
        let pool = setup_test_db().await;
        let (user_a, _) = create_test_user(&pool, "vault_user_a", "Pass123!", None).await;
        let (user_b, _) = create_test_user(&pool, "vault_user_b", "Pass123!", None).await;

        create_vault(&pool, user_a, "SecretVault".to_string(), None)
            .await
            .expect("Should create vault for user A");

        let vaults_b = fetch_vaults_by_user(&pool, user_b)
            .await
            .expect("Should fetch vaults for user B");

        assert!(
            vaults_b.is_empty(),
            "User B should not see user A's vaults"
        );
    }

    // =========================================================================
    // === Update Vault ===
    // =========================================================================

    #[tokio::test]
    async fn test_update_vault_name() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_update_name", "Pass123!", None).await;

        let mut vault = create_vault(&pool, user_id, "OldName".to_string(), None)
            .await
            .expect("Should create vault");

        vault.name = "NewName".to_string();
        update_vault(&pool, vault.clone())
            .await
            .expect("Should update vault");

        let vaults = fetch_vaults_by_user(&pool, user_id)
            .await
            .expect("Should fetch vaults");
        assert_eq!(vaults[0].name, "NewName");
    }

    #[tokio::test]
    async fn test_update_vault_description() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_update_desc", "Pass123!", None).await;

        let mut vault = create_vault(&pool, user_id, "MyVault".to_string(), None)
            .await
            .expect("Should create vault");

        vault.description = Some("New description".to_string());
        update_vault(&pool, vault.clone())
            .await
            .expect("Should update vault");

        let vaults = fetch_vaults_by_user(&pool, user_id)
            .await
            .expect("Should fetch vaults");
        assert_eq!(
            vaults[0].description,
            Some("New description".to_string())
        );
    }

    // =========================================================================
    // === Delete Vault ===
    // =========================================================================

    #[tokio::test]
    async fn test_delete_vault_success() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_delete", "Pass123!", None).await;

        let vault = create_vault(&pool, user_id, "ToDelete".to_string(), None)
            .await
            .expect("Should create vault");
        let vault_id = vault.id.expect("Vault should have an ID");

        delete_vault(&pool, vault_id)
            .await
            .expect("Should delete vault");

        let vaults = fetch_vaults_by_user(&pool, user_id)
            .await
            .expect("Should fetch vaults");
        assert!(vaults.is_empty(), "Deleted vault should not be recoverable");
    }

    #[tokio::test]
    async fn test_delete_vault_nonexistent() {
        let pool = setup_test_db().await;

        // Deleting a vault that doesn't exist should not fail
        let result = delete_vault(&pool, 99999).await;
        assert!(result.is_ok(), "Deleting nonexistent vault should succeed silently");
    }

    // =========================================================================
    // === Password Count by Vault ===
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_password_count_by_vault_empty() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_count_empty", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        let count = fetch_password_count_by_vault(&pool, vault_id)
            .await
            .expect("Should fetch count");

        assert_eq!(count, 0, "New vault should have 0 passwords");
    }

    #[tokio::test]
    async fn test_fetch_password_count_by_vault_with_passwords() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "vault_count_some", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        let passwords: Vec<StoredRawPassword> = vec![
            ("https://s1.com", "Pass1!"),
            ("https://s2.com", "Pass2!"),
            ("https://s3.com", "Pass3!"),
        ]
        .into_iter()
        .map(|(url, pwd)| StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            vault_id,
            name: String::new(),
            username: SecretString::new(String::new().into()),
            url: SecretString::new(url.into()),
            password: SecretString::new(pwd.into()),
            notes: None,
            score: None,
            created_at: None,
        })
        .collect();

        crate::backend::password_utils::create_stored_data_pipeline_bulk(&pool, user_id, passwords)
            .await
            .expect("Should insert passwords");

        let count = fetch_password_count_by_vault(&pool, vault_id)
            .await
            .expect("Should fetch count");

        assert_eq!(count, 3, "Vault should contain 3 passwords");
    }
}
