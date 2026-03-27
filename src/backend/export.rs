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

use crate::backend::db_backend::{fetch_all_stored_passwords_for_user, fetch_user_auth_from_id};
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
    let stored_passwords = fetch_all_stored_passwords_for_user(pool, user_id)
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

use std::path::PathBuf;

/// Genera un path di export default nella directory specificata.
///
/// # Arguments
/// * `directory` - Directory di destinazione
/// * `format` - Formato di export
///
/// # Returns
/// Path completo con nome file generato (es. pwdmanager_export_20260306_143022.json)
pub fn generate_export_path(directory: &Path, format: ExportFormat) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("pwdmanager_export_{}.{}", timestamp, format.extension());
    directory.join(filename)
}

/// Valida che il path di export sia scrivibile.
///
/// # Arguments
/// * `path` - Path da validare
///
/// # Returns
/// * `Ok(())` se il path è valido
/// * `Err(String)` con descrizione del problema
pub fn validate_export_path(path: &Path) -> Result<(), String> {
    // Verifica che la directory padre esista
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        return Err(format!("Directory does not exist: {}", parent.display()));
    }

    // Verifica che non sia una directory
    if path.exists() && path.is_dir() {
        return Err(format!(
            "Path is a directory, not a file: {}",
            path.display()
        ));
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
}
