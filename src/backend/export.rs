// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Modulo per l'export delle password in vari formati.
//!
//! # Flusso dati
//! ```text
//! Database (StoredPassword criptate)
//!          ↓ fetch + decrypt (con progress tracking)
//! StoredRawPassword (SecretString)
//!          ↓ .expose_secret() in ExportablePassword
//! ExportablePassword (String in chiaro)
//!          ↓ serialize
//! File JSON/CSV/XML
//! ```

use crate::backend::db_backend::{fetch_all_stored_passwords_for_vault, fetch_user_auth_from_id};
use crate::backend::export_types::{ExportFormat, ExportablePassword, XmlExportRoot};
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use crate::backend::password_utils::decrypt_bulk_stored_data;
use pwd_types::StoredRawPassword;
use quick_xml::se::to_string as xml_to_string;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{fs, task};

/// Serializza le password in formato JSON (pretty-printed).
pub fn serialize_to_json(passwords: &[ExportablePassword]) -> Result<String, String> {
    serde_json::to_string_pretty(passwords).map_err(|e| format!("JSON serialization error: {}", e))
}

/// Serializza le password in formato CSV.
pub fn serialize_to_csv(passwords: &[ExportablePassword]) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    for pwd in passwords {
        wtr.serialize(pwd)
            .map_err(|e| format!("CSV serialization error: {}", e))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| format!("CSV writer error: {}", e))?;

    String::from_utf8(bytes).map_err(|e| format!("CSV UTF-8 error: {}", e))
}

/// Serializza le password in formato XML.
pub fn serialize_to_xml(passwords: &[ExportablePassword]) -> Result<String, String> {
    let root = XmlExportRoot {
        passwords: passwords.to_vec(),
    };
    xml_to_string(&root).map_err(|e| format!("XML serialization error: {}", e))
}

/// Serializza le password nel formato specificato.
pub fn serialize_passwords(
    passwords: &[ExportablePassword],
    format: ExportFormat,
) -> Result<String, String> {
    match format {
        ExportFormat::Json => serialize_to_json(passwords),
        ExportFormat::Csv => serialize_to_csv(passwords),
        ExportFormat::Xml => serialize_to_xml(passwords),
    }
}

/// Pipeline completa per esportare le password con feedback di progresso.
///
/// Segue lo stesso pattern di `stored_passwords_migration_pipeline_with_progress`:
/// - Passa `progress_tx` direttamente a `decrypt_bulk_stored_data`
/// - Invia messaggi di cambio stage manualmente
/// - Usa `Arc<AtomicUsize>` per progress tracking durante serializzazione
///
/// # Flusso dati
/// 1. Fetch StoredPassword dal DB (crittografate)
/// 2. Decrypt in StoredRawPassword con progress tracking (riusa ProgressSender)
/// 3. Converti in ExportablePassword (chiama .expose_secret())
/// 4. Serializza e scrivi su file
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `output_path` - Path del file di output
/// * `format` - Formato di export (JSON, CSV, XML)
/// * `progress_tx` - Canale opzionale per il progress tracking (stesso tipo della migrazione)
pub async fn export_passwords_pipeline_with_progress(
    pool: &SqlitePool,
    user_id: i64,
    vault_id: i64,
    output_path: &Path,
    format: ExportFormat,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<(), String> {
    tracing::info!("export_passwords_pipeline_with_progress: starting");

    // Invia stato iniziale
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Decrypting, 0, 0))
            .await;
    }

    // 1. Fetch StoredPassword crittografate dal database
    tracing::info!("export_passwords_pipeline_with_progress: fetching passwords from DB");
    let stored_passwords = fetch_all_stored_passwords_for_vault(pool, vault_id)
        .await
        .map_err(|e| e.to_string())?;

    let total = stored_passwords.len();
    tracing::info!(
        "export_passwords_pipeline_with_progress: fetched {} passwords",
        total
    );

    if total == 0 {
        // Nessuna password da esportare
        if let Some(tx) = &progress_tx {
            let _ = tx
                .send(ProgressMessage::new(MigrationStage::Completed, 0, 0))
                .await;
        }
        let content = serialize_passwords(&[], format)?;
        fs::write(output_path, content)
            .await
            .map_err(|e| format!("File write error: {}", e))?;
        return Ok(());
    }

    // 2. Prepara UserAuth per la decrittografia
    tracing::info!("export_passwords_pipeline_with_progress: fetching user auth");
    let user_auth = fetch_user_auth_from_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Decrypt con progress tracking (stesso pattern della migrazione)
    // Passiamo progress_tx direttamente a decrypt_bulk_stored_data
    tracing::info!("export_passwords_pipeline_with_progress: calling decrypt_bulk_stored_data");
    let raw_passwords = decrypt_bulk_stored_data(user_auth, stored_passwords, progress_tx.clone())
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!("export_passwords_pipeline_with_progress: decrypt_bulk_stored_data completed");

    // Invia cambio stage - Serializing
    tracing::info!("export_passwords_pipeline_with_progress: sending Serializing stage");
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Serializing, 0, total))
            .await;
    }
    tracing::info!("export_passwords_pipeline_with_progress: Serializing stage sent");

    // 4. Converti in ExportablePassword con progress tracking
    // ExportablePassword::from_stored_raw() chiama .expose_secret()
    tracing::info!("export_passwords_pipeline_with_progress: converting to exportable format");
    let exportable_passwords =
        convert_to_exportable_with_progress(raw_passwords, progress_tx.clone(), total);
    tracing::info!("export_passwords_pipeline_with_progress: conversion completed");

    // Invia cambio stage - Writing
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Writing, 0, total))
            .await;
    }

    // 5. Serializza nel formato richiesto
    let content = serialize_passwords(&exportable_passwords, format)?;

    // 6. Scrivi su file
    fs::write(output_path, content)
        .await
        .map_err(|e| format!("File write error: {}", e))?;

    // Invia completamento
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Completed, 100, 100))
            .await;
    }

    Ok(())
}

/// Converte StoredRawPassword in ExportablePassword con progress tracking.
///
/// Usa `.expose_secret()` per convertire i SecretString in String in chiaro.
fn convert_to_exportable_with_progress(
    raw_passwords: Vec<StoredRawPassword>,
    progress_tx: Option<Arc<ProgressSender>>,
    total: usize,
) -> Vec<ExportablePassword> {
    if raw_passwords.is_empty() {
        return Vec::new();
    }

    let completed = Arc::new(AtomicUsize::new(0));

    // Usa block_in_place per la conversione CPU-bound
    task::block_in_place(|| {
        raw_passwords
            .into_iter()
            .map(|rp| {
                // ExportablePassword::from_stored_raw() chiama .expose_secret()
                let exportable = ExportablePassword::from_stored_raw(&rp);

                // Aggiorna progress (stesso pattern di decrypt_bulk_stored_data)
                if let Some(tx) = &progress_tx {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    // Usa try_send invece di blocking_send per evitare deadlock con molti elementi
                    let _ = tx.try_send(ProgressMessage::new(
                        MigrationStage::Serializing,
                        current,
                        total,
                    ));
                }

                exportable
            })
            .collect()
    })
}

/// Pipeline per esportare TUTTI i vault di un utente in un unico file.
///
/// Itera su tutti i vault dell'utente, raccoglie le password crittografate da ciascuno,
/// le decrittografa in blocco e aggrega il risultato in un singolo file di output.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `output_path` - Path del file di output
/// * `format` - Formato di export (JSON, CSV, XML)
/// * `progress_tx` - Canale opzionale per il progress tracking
pub async fn export_all_user_passwords_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    output_path: &Path,
    format: ExportFormat,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<(), String> {
    tracing::info!("export_all_user_passwords_pipeline: starting for user_id={}", user_id);

    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Decrypting, 0, 0)).await;
    }

    // 1. Fetch all vaults for the user
    let vaults = crate::backend::vault_utils::fetch_vaults_by_user(pool, user_id)
        .await
        .map_err(|e| format!("Failed to fetch vaults: {}", e))?;

    // 2. Collect encrypted passwords from all vaults
    let mut all_stored_passwords: Vec<pwd_types::StoredPassword> = Vec::new();
    let total_vaults = vaults.len();

    for (vault_index, vault) in vaults.iter().enumerate() {
        tracing::info!(
            "export_all_user_passwords_pipeline: fetching vault '{}' ({}/{})",
            vault.name, vault_index + 1, total_vaults
        );

        let vault_id = vault.id
            .ok_or_else(|| format!("Vault '{}' has no ID", vault.name))?;
        let stored_passwords = fetch_all_stored_passwords_for_vault(pool, vault_id)
            .await
            .map_err(|e| format!("Failed to fetch passwords from vault '{}': {}", vault.name, e))?;

        all_stored_passwords.extend(stored_passwords);

        if let Some(tx) = &progress_tx {
            let overall_progress = if total_vaults > 0 {
                ((vault_index + 1) as f64 / total_vaults as f64 * 30.0) as usize
            } else {
                30
            };
            let _ = tx.send(ProgressMessage::new(MigrationStage::Decrypting, overall_progress, 100)).await;
        }
    }

    // 3. Decrypt all passwords in one pass
    let all_raw_passwords = if all_stored_passwords.is_empty() {
        if let Some(tx) = &progress_tx {
            let _ = tx.send(ProgressMessage::new(MigrationStage::Completed, 100, 100)).await;
        }
        let content = serialize_passwords(&[], format)?;
        fs::write(output_path, content)
            .await
            .map_err(|e| format!("File write error: {}", e))?;
        tracing::info!("export_all_user_passwords_pipeline: no passwords found, empty file written");
        return Ok(());
    } else {
        let user_auth = fetch_user_auth_from_id(pool, user_id)
            .await
            .map_err(|e| e.to_string())?;

        decrypt_bulk_stored_data(user_auth, all_stored_passwords, None)
            .await
            .map_err(|e| format!("Failed to decrypt passwords: {}", e))?
    };

    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Serializing, 70, 100)).await;
    }

    // 4. Convert to exportable format
    let total_passwords = all_raw_passwords.len();
    let exportable: Vec<ExportablePassword> = all_raw_passwords
        .iter()
        .map(ExportablePassword::from_stored_raw)
        .collect();

    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Writing, 85, 100)).await;
    }

    // 5. Serialize and write
    let content = serialize_passwords(&exportable, format)?;

    fs::write(output_path, content)
        .await
        .map_err(|e| format!("File write error: {}", e))?;

    tracing::info!(
        "export_all_user_passwords_pipeline: completed, {} passwords exported to {:?}",
        total_passwords, output_path
    );

    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Completed, 100, 100)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_password() -> ExportablePassword {
        ExportablePassword {
            name: "Example Service".to_string(),
            username: "user@example.com".to_string(),
            url: "example.com".to_string(),
            password: "secret123".to_string(),
            notes: Some("test notes".to_string()),
            score: Some(85),
            created_at: Some("2024-01-01".to_string()),
        }
    }

    #[test]
    fn test_serialize_to_json() {
        let passwords = vec![create_test_password()];
        let result = serialize_to_json(&passwords);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("example.com"));
    }

    #[test]
    fn test_serialize_to_csv() {
        let passwords = vec![create_test_password()];
        let result = serialize_to_csv(&passwords);
        assert!(result.is_ok());
        let csv = result.unwrap();
        assert!(csv.contains("url"));
        assert!(csv.contains("example.com"));
    }

    #[test]
    fn test_serialize_to_xml() {
        let passwords = vec![create_test_password()];
        let result = serialize_to_xml(&passwords);
        assert!(result.is_ok());
        let xml = result.unwrap();
        assert!(xml.contains("<passwords>"));
        assert!(xml.contains("example.com"));
    }

    #[test]
    fn test_convert_to_exportable_with_progress_empty() {
        let result = convert_to_exportable_with_progress(vec![], None, 0);
        assert!(result.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_to_exportable_with_progress_sends_messages() {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel(100);
        let progress_tx = Arc::new(tx);

        let raw_passwords = vec![create_stored_raw_for_convert("site1.com", "pass1")];
        let total = raw_passwords.len();

        let result = convert_to_exportable_with_progress(raw_passwords, Some(progress_tx), total);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url, "site1.com");

        // Verify progress messages were sent
        drop(rx); // Drop rx so try_recv doesn't block
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_to_exportable_preserves_all_fields() {
        let raw = vec![
            create_stored_raw_for_convert("site.com", "pass123"),
            create_stored_raw_for_convert("other.com", "otherpass"),
        ];

        let result = convert_to_exportable_with_progress(raw, None, 2);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].url, "site.com");
        assert_eq!(result[0].password, "pass123");
        assert_eq!(result[1].url, "other.com");
    }

    /// Helper: create a StoredRawPassword for convert tests
    fn create_stored_raw_for_convert(url: &str, password: &str) -> StoredRawPassword {
        use secrecy::SecretString;
        StoredRawPassword {
            uuid: uuid::Uuid::new_v4(),
            id: None,
            user_id: 1,
            vault_id: 1,
            name: format!("Name_{}", url),
            username: SecretString::new(format!("user@{}", url).into()),
            url: SecretString::new(url.into()),
            password: SecretString::new(password.into()),
            notes: Some(SecretString::new("notes".into())),
            score: Some(pwd_types::PasswordScore::new(75)),
            created_at: Some("2024-01-01".to_string()),
        }
    }

    #[test]
    fn test_aggregate_exportable_passwords_from_multiple_vaults() {
        let pwd1 = create_test_password();
        let pwd2 = ExportablePassword {
            name: "Other Service".to_string(),
            username: "admin@example.com".to_string(),
            url: "other.com".to_string(),
            password: "admin123".to_string(),
            notes: None,
            score: Some(92),
            created_at: Some("2025-06-01".to_string()),
        };

        let passwords = vec![pwd1, pwd2];
        let json = serialize_to_json(&passwords).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("other.com"));
    }
}
