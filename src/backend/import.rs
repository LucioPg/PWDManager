// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Modulo per l'import delle password da file in vari formati.
//!
//! # Flusso dati
//! ```text
//! File (JSON/CSV/XML)
//!          ↓ read + parse
//! ExportablePassword (String in chiaro)
//!          ↓ deduplicate by (url, password)
//! ExportablePassword (unici)
//!          ↓ to_stored_raw() + user_id
//! StoredRawPassword (SecretString)
//!          ↓ encrypt with user cipher
//! StoredPassword (criptato)
//!          ↓ upsert to DB
//! Database
//! ```

use crate::backend::db_backend::{
    fetch_all_stored_passwords_for_vault, fetch_user_auth_from_id, upsert_stored_passwords_batch,
};
use crate::backend::export_types::{ExportFormat, ExportablePassword, XmlExportRoot};
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use crate::backend::password_utils::{
    create_cipher, create_stored_data_records, decrypt_bulk_stored_data, get_salt,
};
use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

/// Parse JSON content into ExportablePassword list.
pub fn parse_from_json(content: &str) -> Result<Vec<ExportablePassword>, String> {
    serde_json::from_str(content).map_err(|e| format!("JSON parse error: {}", e))
}

/// Parse Firefox CSV content into ExportablePassword list.
///
/// Firefox exports CSV with columns:
/// url, username, password, httpRealm, formActionOrigin, guid, timeCreated, timeLastUsed, timePasswordChanged
///
/// Mapping rules:
/// - `name` field: Firefox has no `name` → falls back to `username`, then `url`
/// - `created_at`: converted from Firefox `timeCreated` (Unix ms timestamp) to ISO string
pub fn parse_firefox_csv(content: &str) -> Result<Vec<ExportablePassword>, String> {
    use chrono::DateTime;

    let mut reader = csv::ReaderBuilder::new().from_reader(content.as_bytes());
    let mut passwords = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("Firefox CSV parse error: {}", e))?;

        let url = record.get(0).unwrap_or("").to_string();
        let username = record.get(1).unwrap_or("").to_string();
        let password = record.get(2).unwrap_or("").to_string();
        let time_created = record.get(6).unwrap_or("");

        // Fallback: name = username, then url
        let name = if !username.is_empty() {
            username.clone()
        } else if !url.is_empty() {
            url.clone()
        } else {
            String::new()
        };

        // Convert Unix ms timestamp → ISO string
        let created_at = if !time_created.is_empty() {
            time_created
                .parse::<i64>()
                .ok()
                .and_then(DateTime::from_timestamp_millis)
                .map(|dt| dt.to_string())
        } else {
            None
        };

        passwords.push(ExportablePassword {
            name,
            username,
            url,
            password,
            notes: None,
            score: None,
            created_at,
        });
    }

    Ok(passwords)
}

/// Parse CSV content into ExportablePassword list.
///
/// Auto-detects Firefox CSV (header contains `timeCreated`) and uses
/// `parse_firefox_csv` in that case. Otherwise falls back to the standard
/// app CSV format.
pub fn parse_from_csv(content: &str) -> Result<Vec<ExportablePassword>, String> {
    let is_firefox = content.lines().next().is_some_and(|header| {
        header.contains("timeCreated")
    });

    if is_firefox {
        return parse_firefox_csv(content);
    }

    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut passwords = Vec::new();

    for result in reader.deserialize() {
        let password: ExportablePassword = result.map_err(|e| format!("CSV parse error: {}", e))?;
        passwords.push(password);
    }

    Ok(passwords)
}

/// Parse XML content into ExportablePassword list.
pub fn parse_from_xml(content: &str) -> Result<Vec<ExportablePassword>, String> {
    let root: XmlExportRoot =
        quick_xml::de::from_str(content).map_err(|e| format!("XML parse error: {}", e))?;
    Ok(root.passwords)
}

/// Parse content based on format.
pub fn parse_passwords(
    content: &str,
    format: ExportFormat,
) -> Result<Vec<ExportablePassword>, String> {
    match format {
        ExportFormat::Json => parse_from_json(content),
        ExportFormat::Csv => parse_from_csv(content),
        ExportFormat::Xml => parse_from_xml(content),
    }
}

/// Deduplicates passwords based on (url, password) combination.
///
/// Returns (unique_passwords, duplicates_count).
/// Prioritizes first occurrence when duplicates exist.
pub fn deduplicate_passwords(
    passwords: Vec<ExportablePassword>,
) -> (Vec<ExportablePassword>, usize) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    let original_count = passwords.len();

    for pwd in passwords {
        let key = (pwd.url.clone(), pwd.password.clone());
        if seen.insert(key) {
            unique.push(pwd);
        }
    }

    let duplicates_count = original_count - unique.len();
    (unique, duplicates_count)
}

/// Validates that the import file exists and is readable.
///
/// # Arguments
/// * `path` - Path to the import file
///
/// # Returns
/// * `Ok(ExportFormat)` if valid, with detected format
/// * `Err(String)` with description of the problem
pub fn validate_import_path(path: &Path) -> Result<ExportFormat, String> {
    // Check file exists
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    // Check is a file (not directory)
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    // Detect format from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("json") => Ok(ExportFormat::Json),
        Some("csv") => Ok(ExportFormat::Csv),
        Some("xml") => Ok(ExportFormat::Xml),
        _ => Err(format!(
            "Unsupported file format. Expected: .json, .csv, .xml. Got: {:?}",
            extension
        )),
    }
}

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported_count: usize,
    pub skipped_duplicates: usize,
    pub total_in_file: usize,
}

/// Patch the imported StoredPasswords for forcing score recalculation
#[allow(non_snake_case)]
fn storedRawPasswords_score_patch(stored_passwords: &mut [StoredRawPassword]) {
    stored_passwords.iter_mut().for_each(|sp| {
        sp.score = None;
    })
}

/// Pipeline completa per importare password con feedback di progresso.
///
/// # Flusso
/// 1. Leggi file dal disco
/// 2. Parse nel formato appropriato
/// 3. Deduplica (per url + password)
/// 4. Filtra password che esistono già nel DB per questo utente
/// 5. Cripta e salva nel DB
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente (assegnato alle password importate)
/// * `input_path` - Path del file di input
/// * `format` - Formato del file (JSON, CSV, XML)
/// * `progress_tx` - Canale opzionale per il progress tracking
///
/// # Returns
/// * `Ok(ImportResult)` - Con conteggi di importazione
/// * `Err(String)` - Con descrizione dell'errore
pub async fn import_passwords_pipeline_with_progress(
    pool: &SqlitePool,
    user_id: i64,
    vault_id: i64,
    input_path: &Path,
    format: ExportFormat,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<ImportResult, String> {
    // Invia stato iniziale - Reading
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock con molti elementi
        let _ = tx.try_send(ProgressMessage::new(MigrationStage::Reading, 0, 0));
    }

    // 1. Leggi file
    let content = fs::read_to_string(input_path)
        .await
        .map_err(|e| format!("File read error: {}", e))?;

    // Invia cambio stage - Parsing
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock
        let _ = tx.try_send(ProgressMessage::new(MigrationStage::Deserializing, 0, 0));
    }

    // 2. Parse content
    let passwords = parse_passwords(&content, format)?;
    let total_in_file = passwords.len();

    if total_in_file == 0 {
        // File vuoto
        if let Some(tx) = &progress_tx {
            // Usa try_send invece di blocking_send per evitare deadlock
            let _ = tx.try_send(ProgressMessage::new(MigrationStage::Completed, 0, 0));
        }
        return Ok(ImportResult {
            imported_count: 0,
            skipped_duplicates: 0,
            total_in_file: 0,
        });
    }

    // Invia cambio stage - Deduplicating
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock
        let _ = tx.try_send(ProgressMessage::new(
            MigrationStage::Deduplicating,
            0,
            total_in_file,
        ));
    }

    // 3. Deduplica password nel file
    let (unique_passwords, file_duplicates) = deduplicate_passwords(passwords);

    // 4. Recupera password esistenti dell'utente per confronto
    let existing_passwords = fetch_all_stored_passwords_for_vault(pool, vault_id)
        .await
        .map_err(|e| e.to_string())?;

    // Recupera user_auth per il decrypt delle password esistenti
    let user_auth_for_decrypt = fetch_user_auth_from_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // Decrypt password esistenti per confronto (senza progress tracking)
    let existing_raw = decrypt_bulk_stored_data(user_auth_for_decrypt, existing_passwords, None)
        .await
        .map_err(|e| e.to_string())?;

    // Crea set di (url, password) esistenti
    let existing_set: std::collections::HashSet<(String, String)> = existing_raw
        .iter()
        .map(|rp| {
            (
                rp.url.expose_secret().to_string(),
                rp.password.expose_secret().to_string(),
            )
        })
        .collect();

    // Filtra password che esistono già
    let new_passwords: Vec<ExportablePassword> = unique_passwords
        .into_iter()
        .filter(|p| !existing_set.contains(&(p.url.clone(), p.password.clone())))
        .collect();

    let db_duplicates = total_in_file - file_duplicates - new_passwords.len();
    let skipped_duplicates = file_duplicates + db_duplicates;

    if new_passwords.is_empty() {
        // Nessuna nuova password da importare
        if let Some(tx) = &progress_tx {
            // Usa try_send invece di blocking_send per evitare deadlock
            let _ = tx.try_send(ProgressMessage::new(
                MigrationStage::Completed,
                0,
                total_in_file,
            ));
        }
        return Ok(ImportResult {
            imported_count: 0,
            skipped_duplicates,
            total_in_file,
        });
    }

    let to_import = new_passwords.len();

    // Invia cambio stage - Encrypting
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock
        let _ = tx.try_send(ProgressMessage::new(
            MigrationStage::Encrypting,
            0,
            to_import,
        ));
    }

    // 5. Converti in StoredRawPassword con user_id
    let mut stored_raw: Vec<StoredRawPassword> = new_passwords
        .into_iter()
        .map(|p| p.to_stored_raw(user_id, vault_id))
        .collect();
    storedRawPasswords_score_patch(&mut stored_raw);
    // 6. Cripta con progress tracking
    // Recupera user_auth per l'encrypt delle nuove password
    let user_auth = fetch_user_auth_from_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth).map_err(|e| e.to_string())?;

    let stored_passwords =
        create_stored_data_records(cipher, user_auth, stored_raw, progress_tx.clone())
            .await
            .map_err(|e| e.to_string())?;

    // Invia cambio stage - Importing
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock
        let _ = tx.try_send(ProgressMessage::new(
            MigrationStage::Importing,
            0,
            to_import,
        ));
    }

    // 7. Salva nel DB
    upsert_stored_passwords_batch(pool, stored_passwords)
        .await
        .map_err(|e| e.to_string())?;

    // Invia completamento
    if let Some(tx) = &progress_tx {
        // Usa try_send invece di blocking_send per evitare deadlock
        let _ = tx.try_send(ProgressMessage::new(
            MigrationStage::Completed,
            to_import,
            to_import,
        ));
    }

    Ok(ImportResult {
        imported_count: to_import,
        skipped_duplicates,
        total_in_file,
    })
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
    fn test_parse_from_json() {
        let json = r#"[{"url":"site.com","password":"pass123"}]"#;
        let result = parse_from_json(json);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].url, "site.com");
    }

    #[test]
    fn test_parse_from_csv() {
        let csv = "url,password,notes\nsite.com,pass123,test";
        let result = parse_from_csv(csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].url, "site.com");
    }

    #[test]
    fn test_parse_from_xml() {
        let xml = r#"<passwords><password><url>site.com</url><password>pass123</password></password></passwords>"#;
        let result = parse_from_xml(xml);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].url, "site.com");
    }

    #[test]
    fn test_parse_from_json_with_name_username() {
        let json =
            r#"[{"name":"GitHub","username":"devuser","url":"github.com","password":"pass123"}]"#;
        let result = parse_from_json(json);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].name, "GitHub");
        assert_eq!(passwords[0].username, "devuser");
        assert_eq!(passwords[0].url, "github.com");
    }

    #[test]
    fn test_parse_from_json_missing_name_username_defaults_to_empty() {
        // Test backwards compatibility: old files without name/username
        let json = r#"[{"url":"legacy.com","password":"oldpass"}]"#;
        let result = parse_from_json(json);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].name, ""); // Default from #[serde(default)]
        assert_eq!(passwords[0].username, ""); // Default from #[serde(default)]
    }

    #[test]
    fn test_deduplicate_passwords_no_duplicates() {
        let passwords = vec![
            create_test_password(),
            ExportablePassword {
                name: "Other Service".to_string(),
                username: "other@example.com".to_string(),
                url: "other.com".to_string(),
                password: "different".to_string(),
                notes: None,
                score: None,
                created_at: None,
            },
        ];
        let (unique, dupes) = deduplicate_passwords(passwords);
        assert_eq!(unique.len(), 2);
        assert_eq!(dupes, 0);
    }

    #[test]
    fn test_deduplicate_passwords_with_duplicates() {
        let passwords = vec![
            create_test_password(),
            create_test_password(), // Duplicate
        ];
        let (unique, dupes) = deduplicate_passwords(passwords);
        assert_eq!(unique.len(), 1);
        assert_eq!(dupes, 1);
    }

    // ==================== PIPELINE INTEGRATION TESTS ====================

    #[tokio::test]
    async fn test_import_pipeline_json_with_db() {
        use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
        use tempfile::NamedTempFile;
        use std::io::Write;

        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "import_pipe_json", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        let json = r#"[
            {"url":"site1.com","password":"pass1","name":"Site1","username":"user1"},
            {"url":"site2.com","password":"pass2","name":"Site2","username":"user2"}
        ]"#;

        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        write!(file, "{}", json).unwrap();

        let result = import_passwords_pipeline_with_progress(
            &pool,
            user_id,
            vault_id,
            file.path(),
            ExportFormat::Json,
            None,
        )
        .await;

        assert!(result.is_ok(), "Import pipeline should succeed: {:?}", result);
        let import_result = result.unwrap();
        assert_eq!(import_result.imported_count, 2);
        assert_eq!(import_result.total_in_file, 2);
        assert_eq!(import_result.skipped_duplicates, 0);

        // Verify passwords are in the DB
        let passwords =
            crate::backend::db_backend::fetch_all_stored_passwords_for_vault(&pool, vault_id)
                .await
                .expect("Should fetch passwords");
        assert_eq!(passwords.len(), 2);
    }

    #[tokio::test]
    async fn test_import_pipeline_skips_db_duplicates() {
        use crate::backend::password_utils::create_stored_data_pipeline_bulk;
        use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
        use secrecy::SecretString;
        use tempfile::NamedTempFile;
        use std::io::Write;

        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "import_pipe_dedup", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        // Pre-insert one password in the DB
        let existing = pwd_types::StoredRawPassword {
            uuid: uuid::Uuid::new_v4(),
            id: None,
            user_id,
            vault_id,
            name: "Site1".to_string(),
            username: SecretString::new("user1".into()),
            url: SecretString::new("site1.com".into()),
            password: SecretString::new("pass1".into()),
            notes: None,
            score: None,
            created_at: None,
        };
        create_stored_data_pipeline_bulk(&pool, user_id, vec![existing])
            .await
            .expect("Should insert existing password");

        // Try to import the same password
        let json = r#"[
            {"url":"site1.com","password":"pass1","name":"Site1","username":"user1"},
            {"url":"site2.com","password":"pass2","name":"Site2","username":"user2"}
        ]"#;

        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        write!(file, "{}", json).unwrap();

        let result = import_passwords_pipeline_with_progress(
            &pool,
            user_id,
            vault_id,
            file.path(),
            ExportFormat::Json,
            None,
        )
        .await;

        assert!(result.is_ok(), "Import pipeline should succeed: {:?}", result);
        let import_result = result.unwrap();
        assert_eq!(import_result.imported_count, 1, "Should import only the new password");
        assert_eq!(import_result.skipped_duplicates, 1, "Should skip the DB duplicate");
        assert_eq!(import_result.total_in_file, 2);
    }

    #[tokio::test]
    async fn test_import_pipeline_csv_format() {
        use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
        use tempfile::NamedTempFile;
        use std::io::Write;

        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "import_pipe_csv", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        let csv = "url,password,notes,score,created_at\nsite1.com,pass1,note1,80,2024-01-01";

        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        write!(file, "{}", csv).unwrap();

        let result = import_passwords_pipeline_with_progress(
            &pool,
            user_id,
            vault_id,
            file.path(),
            ExportFormat::Csv,
            None,
        )
        .await;

        assert!(result.is_ok(), "CSV import should succeed: {:?}", result);
        let import_result = result.unwrap();
        assert_eq!(import_result.imported_count, 1);
    }

    #[tokio::test]
    async fn test_import_pipeline_empty_file() {
        use crate::backend::test_helpers::{create_test_user, create_test_vault, setup_test_db};
        use tempfile::NamedTempFile;
        use std::io::Write;

        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "import_pipe_empty", "Pass123!", None).await;
        let (vault_id, _) = create_test_vault(&pool, user_id).await;

        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        write!(file, "[]").unwrap();

        let result = import_passwords_pipeline_with_progress(
            &pool,
            user_id,
            vault_id,
            file.path(),
            ExportFormat::Json,
            None,
        )
        .await;

        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.imported_count, 0);
        assert_eq!(import_result.total_in_file, 0);
    }
}
