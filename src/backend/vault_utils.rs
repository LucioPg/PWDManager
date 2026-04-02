// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! CRUD operations per i vault.

use custom_errors::DBError;
use pwd_types::Vault;
use sqlx::SqlitePool;

/// Crea un nuovo vault.
pub async fn create_vault(
    pool: &SqlitePool,
    user_id: i64,
    name: String,
    description: Option<String>,
) -> Result<Vault, DBError> {
    let vault = Vault {
        id: None,
        user_id,
        name,
        description,
        created_at: None,
    };
    Vault::upsert_by_id(&vault, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Failed to create vault: {}", e)))?;

    // Fetch the created vault back to get the auto-generated id and created_at
    let created_vault = Vault::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch created vault: {}", e)))?
        .into_iter()
        .find(|v| v.name == vault.name && v.description == vault.description)
        .ok_or_else(|| {
            DBError::new_list_error("Created vault not found after insert".to_string())
        })?;

    Ok(created_vault)
}

/// Recupera tutti i vault di un utente.
pub async fn fetch_vaults_by_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<Vault>, DBError> {
    let mut result: Vec<Vault> = Vault::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch vaults: {}", e)))?;

    // Sort by created_at ascending for deterministic "first vault" selection
    result.sort_by_key(|v| v.created_at.clone().unwrap_or_default());
    Ok(result)
}

/// Aggiorna un vault esistente (nome/descrizione).
pub async fn update_vault(pool: &SqlitePool, vault: Vault) -> Result<(), DBError> {
    Vault::upsert_by_id(&vault, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Failed to update vault: {}", e)))?;
    Ok(())
}

/// Elimina un vault (cascade elimina le password associate).
pub async fn delete_vault(pool: &SqlitePool, vault_id: i64) -> Result<(), DBError> {
    sqlx::query("DELETE FROM vaults WHERE id = ?")
        .bind(vault_id)
        .execute(pool)
        .await
        .map_err(|e| DBError::new_password_delete_error(format!("Failed to delete vault: {}", e)))?;
    Ok(())
}

/// Recupera il conteggio delle password in un vault.
pub async fn fetch_password_count_by_vault(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<u64, DBError> {
    let result: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM passwords WHERE vault_id = ?")
            .bind(vault_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to count vault passwords: {}", e))
            })?;
    Ok(result.0 as u64)
}
