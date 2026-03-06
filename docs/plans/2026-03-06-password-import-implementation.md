# Password Import Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

## Progress Status

| Task | Status | Commit |
|------|--------|--------|
| Task 1: Add Import Stages to MigrationStage | ✅ DONE | `db6e252` |
| Task 2: Create ImportablePassword DTO | ✅ DONE | `570583d` |
| Task 3: Implement File Parsers | ✅ DONE | `6b85630` |
| Task 4: Register Import Module | ✅ DONE | `6b85630` |
| Task 5: Implement Import Pipeline with Progress | ⏳ TODO | - |
| Task 6: Add Integration Tests | ⏳ TODO | - |
| Task 7: Run Full Test Suite | ⏳ TODO | - |

**Note per Task 5:** La funzione `get_salt` in `password_utils.rs:63` è privata. Renderla `pub(crate)` prima di implementare `create_cipher_from_auth`.

---

**Goal:** Implement password import from JSON, CSV, XML files produced by the export feature, with progress tracking via mpsc channel pattern.

**Architecture:** Mirror the export pipeline architecture - read file → parse format → deduplicate (by location+password) → encrypt → save to DB. Uses same `ProgressSender` pattern as migration and export pipelines for real-time UI feedback.

**Tech Stack:** Rust, serde (JSON), csv crate, quick-xml, tokio mpsc, rayon, pwd-crypto, pwd-types

---

## Prerequisites

- Files are those produced by the export feature (same format: `ExportablePassword` structure)
- `user_id` is provided at import time (not in file)
- Duplicates are identified by (location, password) combination
- Existing passwords are NOT deleted before import

---

## Task 1: Add Import Stages to MigrationStage

**Files:**
- Modify: `src/backend/migration_types.rs:6-17`

**Step 1: Add new import-specific stages**

Add these stages to `MigrationStage` enum:
- `Reading` - Reading file from disk
- `Deserializing` - Parsing file content (JSON/CSV/XML)
- `Deduplicating` - Removing duplicates based on (location, password)
- `Encrypting` - Encrypting passwords for storage
- `Importing` - Saving to database

```rust
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MigrationStage {
    #[default]
    Idle,
    Decrypting,
    Encrypting,
    Serializing,
    Deserializing,  // NEW: for import parsing
    Reading,        // NEW: file read
    Writing,
    Deduplicating,  // NEW: remove duplicates
    Importing,      // NEW: save to DB
    Finalizing,
    Completed,
    Failed,
}
```

**Step 2: Run tests to verify no breakage**

Run: `cargo test migration_types`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/backend/migration_types.rs
git commit -m "feat(import): add import stages to MigrationStage enum"
```

---

## Task 2: Create ImportablePassword DTO

**Files:**
- Modify: `src/backend/export_types.rs:72` (append to file)

**Step 1: Add Deserialize to ExportablePassword**

The `ExportablePassword` struct already exists for export. We need to make it usable for import by adding `Deserialize`.

```rust
use serde::{Deserialize, Serialize};
use secrecy::ExposeSecret;
use pwd_types::StoredRawPassword;
use secrecy::SecretString;
use uuid::Uuid;

// Update existing ExportablePassword to include Deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportablePassword {
    pub location: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub created_at: Option<String>,
}

// Update XmlExportRoot to include Deserialize for XML import
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "passwords")]
pub struct XmlExportRoot {
    #[serde(rename = "password")]
    pub passwords: Vec<ExportablePassword>,
}
```

**Step 2: Add conversion method to StoredRawPassword**

Add to impl block:

```rust
impl ExportablePassword {
    /// Converts an ExportablePassword to StoredRawPassword for import.
    ///
    /// Creates a new UUID and assigns the provided user_id.
    /// `id` is None (new record, will be assigned by DB).
    /// `created_at` preserves the original timestamp from the import file,
    /// or uses current time if not present.
    pub fn to_stored_raw(&self, user_id: i64) -> StoredRawPassword {
        use pwd_types::PasswordScore;

        StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None, // New record, will be assigned by DB
            user_id,
            location: SecretString::new(self.location.clone().into()),
            password: SecretString::new(self.password.clone().into()),
            notes: self.notes.as_ref().map(|n| SecretString::new(n.clone().into())),
            score: self.score.map(PasswordScore::new),
            created_at: self.created_at.clone(), // Preserve original or use from file
        }
    }
}
```

**Step 3: Run tests to verify serialization**

Run: `cargo test export_types`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/backend/export_types.rs
git commit -m "feat(import): add Deserialize and to_stored_raw to ExportablePassword"
```

---

## Task 3: Implement File Parsers

**Files:**
- Create: `src/backend/import.rs`

**Step 1: Create the import module with parsers**

```rust
//! Modulo per l'import delle password da file in vari formati.
//!
//! # Flusso dati
//! ```text
//! File (JSON/CSV/XML)
//!          ↓ read + parse
//! ExportablePassword (String in chiaro)
//!          ↓ deduplicate by (location, password)
//! ExportablePassword (unici)
//!          ↓ to_stored_raw() + user_id
//! StoredRawPassword (SecretString)
//!          ↓ encrypt with user cipher
//! StoredPassword (criptato)
//!          ↓ upsert to DB
//! Database
//! ```

use crate::backend::export_types::{ExportFormat, ExportablePassword, XmlExportRoot};
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

// Note: AtomicUsize, Ordering, task will be imported in Task 5 when needed

/// Parse JSON content into ExportablePassword list.
pub fn parse_from_json(content: &str) -> Result<Vec<ExportablePassword>, String> {
    serde_json::from_str(content)
        .map_err(|e| format!("JSON parse error: {}", e))
}

/// Parse CSV content into ExportablePassword list.
pub fn parse_from_csv(content: &str) -> Result<Vec<ExportablePassword>, String> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut passwords = Vec::new();

    for result in reader.deserialize() {
        let password: ExportablePassword = result
            .map_err(|e| format!("CSV parse error: {}", e))?;
        passwords.push(password);
    }

    Ok(passwords)
}

/// Parse XML content into ExportablePassword list.
pub fn parse_from_xml(content: &str) -> Result<Vec<ExportablePassword>, String> {
    let root: XmlExportRoot = quick_xml::de::from_str(content)
        .map_err(|e| format!("XML parse error: {}", e))?;
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

/// Deduplicates passwords based on (location, password) combination.
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
        let key = (pwd.location.clone(), pwd.password.clone());
        if seen.insert(key) {
            unique.push(pwd);
        }
    }

    (unique, original_count - unique.len())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_password() -> ExportablePassword {
        ExportablePassword {
            location: "example.com".to_string(),
            password: "secret123".to_string(),
            notes: Some("test notes".to_string()),
            score: Some(85),
            created_at: Some("2024-01-01".to_string()),
        }
    }

    #[test]
    fn test_parse_from_json() {
        let json = r#"[{"location":"site.com","password":"pass123"}]"#;
        let result = parse_from_json(json);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].location, "site.com");
    }

    #[test]
    fn test_parse_from_csv() {
        let csv = "location,password,notes\nsite.com,pass123,test";
        let result = parse_from_csv(csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].location, "site.com");
    }

    #[test]
    fn test_parse_from_xml() {
        let xml = r#"<passwords><password><location>site.com</location><password>pass123</password></password></passwords>"#;
        let result = parse_from_xml(xml);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].location, "site.com");
    }

    #[test]
    fn test_deduplicate_passwords_no_duplicates() {
        let passwords = vec![
            create_test_password(),
            ExportablePassword {
                location: "other.com".to_string(),
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
}
```

**Step 2: Run tests to verify parsers work**

Run: `cargo test import::tests`
Expected: All 5 tests pass

**Step 3: Commit**

```bash
git add src/backend/import.rs
git commit -m "feat(import): add file parsers for JSON, CSV, XML"
```

---

## Task 4: Register Import Module

**Files:**
- Modify: `src/backend/mod.rs`

**Step 1: Add import module**

Add at the end of the module declarations:

```rust
pub mod import;
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/mod.rs
git commit -m "feat(import): register import module"
```

---

## Task 5: Implement Import Pipeline with Progress

**Files:**
- Modify: `src/backend/import.rs` (append)

**Step 1: Add the import pipeline function**

Append to `src/backend/import.rs`:

```rust
use crate::backend::db_backend::{
    fetch_all_stored_passwords_for_user, fetch_user_auth_from_id, upsert_stored_passwords_batch,
};
use crate::backend::password_utils::{create_stored_data_records, decrypt_bulk_stored_data};
use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported_count: usize,
    pub skipped_duplicates: usize,
    pub total_in_file: usize,
}

/// Pipeline completa per importare password con feedback di progresso.
///
/// # Flusso
/// 1. Leggi file dal disco
/// 2. Parse nel formato appropriato
/// 3. Deduplica (per location + password)
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
    input_path: &Path,
    format: ExportFormat,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<ImportResult, String> {
    // Invia stato iniziale - Reading
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Reading, 0, 0))
            .await;
    }

    // 1. Leggi file
    let content = fs::read_to_string(input_path)
        .await
        .map_err(|e| format!("File read error: {}", e))?;

    // Invia cambio stage - Parsing
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Deserializing, 0, 0))
            .await;
    }

    // 2. Parse content
    let mut passwords = parse_passwords(&content, format)?;
    let total_in_file = passwords.len();

    if total_in_file == 0 {
        // File vuoto
        if let Some(tx) = &progress_tx {
            let _ = tx
                .send(ProgressMessage::new(MigrationStage::Completed, 0, 0))
                .await;
        }
        return Ok(ImportResult {
            imported_count: 0,
            skipped_duplicates: 0,
            total_in_file: 0,
        });
    }

    // Invia cambio stage - Deduplicating
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Deduplicating, 0, total_in_file))
            .await;
    }

    // 3. Deduplica password nel file
    let (unique_passwords, file_duplicates) = deduplicate_passwords(passwords);

    // 4. Recupera password esistenti dell'utente per confronto
    let existing_passwords = fetch_all_stored_passwords_for_user(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    let user_auth = fetch_user_auth_from_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // Decrypt password esistenti per confronto (senza progress tracking)
    let existing_raw = decrypt_bulk_stored_data(user_auth.clone(), existing_passwords, None)
        .await
        .map_err(|e| e.to_string())?;

    // Crea set di (location, password) esistenti
    let existing_set: std::collections::HashSet<(String, String)> = existing_raw
        .iter()
        .map(|rp| {
            (
                rp.location.expose_secret().to_string(),
                rp.password.expose_secret().to_string(),
            )
        })
        .collect();

    // Filtra password che esistono già
    let new_passwords: Vec<ExportablePassword> = unique_passwords
        .into_iter()
        .filter(|p| !existing_set.contains(&(p.location.clone(), p.password.clone())))
        .collect();

    let db_duplicates = total_in_file - file_duplicates - new_passwords.len();
    let skipped_duplicates = file_duplicates + db_duplicates;

    if new_passwords.is_empty() {
        // Nessuna nuova password da importare
        if let Some(tx) = &progress_tx {
            let _ = tx
                .send(ProgressMessage::new(MigrationStage::Completed, 0, total_in_file))
                .await;
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
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Encrypting, 0, to_import))
            .await;
    }

    // 5. Converti in StoredRawPassword con user_id
    let stored_raw: Vec<StoredRawPassword> = new_passwords
        .into_iter()
        .map(|p| p.to_stored_raw(user_id))
        .collect();

    // 6. Cripta con progress tracking
    let stored_passwords = create_stored_data_records(
        create_cipher_from_auth(&user_auth)?,
        user_auth,
        stored_raw,
        progress_tx.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Invia cambio stage - Importing
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Importing, 0, to_import))
            .await;
    }

    // 7. Salva nel DB
    upsert_stored_passwords_batch(pool, stored_passwords)
        .await
        .map_err(|e| e.to_string())?;

    // Invia completamento
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Completed, to_import, to_import))
            .await;
    }

    Ok(ImportResult {
        imported_count: to_import,
        skipped_duplicates,
        total_in_file,
    })
}

/// Helper per creare cipher da UserAuth.
fn create_cipher_from_auth(user_auth: &pwd_types::UserAuth) -> Result<aes_gcm::Aes256Gcm, String> {
    use crate::backend::password_utils::{create_cipher, get_salt};

    let salt = get_salt(&user_auth.password);
    create_cipher(&salt, user_auth).map_err(|e| e.to_string())
}
```

**Step 2: Run cargo check to verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/import.rs
git commit -m "feat(import): add import pipeline with progress tracking"
```

---

## Task 6: Add Integration Tests

**Files:**
- Create: `src/backend/import_tests.rs`

**Step 1: Create integration tests**

```rust
//! Integration tests for import functionality.

#[cfg(test)]
mod tests {
    use crate::backend::export_types::{ExportFormat, ExportablePassword};
    use crate::backend::import::{
        deduplicate_passwords, parse_from_csv, parse_from_json, parse_from_xml,
        validate_import_path, ImportResult,
    };
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_passwords() -> Vec<ExportablePassword> {
        vec![
            ExportablePassword {
                location: "site1.com".to_string(),
                password: "pass1".to_string(),
                notes: Some("note1".to_string()),
                score: Some(80),
                created_at: Some("2024-01-01".to_string()),
            },
            ExportablePassword {
                location: "site2.com".to_string(),
                password: "pass2".to_string(),
                notes: None,
                score: Some(90),
                created_at: None,
            },
        ]
    }

    #[test]
    fn test_roundtrip_json() {
        let passwords = create_test_passwords();
        let json = crate::backend::export::serialize_to_json(&passwords).unwrap();
        let parsed = parse_from_json(&json).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].location, "site1.com");
    }

    #[test]
    fn test_roundtrip_csv() {
        let passwords = create_test_passwords();
        let csv = crate::backend::export::serialize_to_csv(&passwords).unwrap();
        let parsed = parse_from_csv(&csv).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].location, "site1.com");
    }

    #[test]
    fn test_roundtrip_xml() {
        let passwords = create_test_passwords();
        let xml = crate::backend::export::serialize_to_xml(&passwords).unwrap();
        let parsed = parse_from_xml(&xml).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].location, "site1.com");
    }

    #[test]
    fn test_validate_import_path_json() {
        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        writeln!(file, "[]").unwrap();
        let path = file.path();
        let result = validate_import_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExportFormat::Json);
    }

    #[test]
    fn test_validate_import_path_csv() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(file, "location,password").unwrap();
        let path = file.path();
        let result = validate_import_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExportFormat::Csv);
    }

    #[test]
    fn test_validate_import_path_xml() {
        let mut file = NamedTempFile::with_suffix(".xml").unwrap();
        writeln!(file, "<passwords></passwords>").unwrap();
        let path = file.path();
        let result = validate_import_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExportFormat::Xml);
    }

    #[test]
    fn test_validate_import_path_unsupported() {
        let mut file = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(file, "test").unwrap();
        let path = file.path();
        let result = validate_import_path(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported"));
    }

    #[test]
    fn test_validate_import_path_nonexistent() {
        let result = validate_import_path(std::path::Path::new("/nonexistent/file.json"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_deduplicate_keeps_first() {
        let passwords = vec![
            ExportablePassword {
                location: "site.com".to_string(),
                password: "pass".to_string(),
                notes: Some("first".to_string()),
                score: Some(80),
                created_at: None,
            },
            ExportablePassword {
                location: "site.com".to_string(),
                password: "pass".to_string(),
                notes: Some("second".to_string()),
                score: Some(90),
                created_at: None,
            },
        ];
        let (unique, dupes) = deduplicate_passwords(passwords);
        assert_eq!(unique.len(), 1);
        assert_eq!(dupes, 1);
        assert_eq!(unique[0].notes, Some("first".to_string())); // Keeps first
    }
}
```

**Step 2: Register test module in mod.rs**

Add to `src/backend/mod.rs`:

```rust
#[cfg(test)]
mod import_tests;
```

**Step 3: Add tempfile dev dependency**

Add to `Cargo.toml` in `[dev-dependencies]`:

```toml
tempfile = "3"
```

**Step 4: Run tests**

Run: `cargo test import_tests`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/backend/import_tests.rs src/backend/mod.rs Cargo.toml
git commit -m "test(import): add integration tests for import functionality"
```

---

## Task 7: Run Full Test Suite

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Commit if any fixes needed**

```bash
git add -A
git commit -m "fix(import): address clippy warnings"
```

---

## Summary

**Files Created:**
- `src/backend/import.rs` - Import parsers and pipeline
- `src/backend/import_tests.rs` - Integration tests

**Files Modified:**
- `src/backend/mod.rs` - Register new modules
- `src/backend/migration_types.rs` - Add import stages
- `src/backend/export_types.rs` - Add Deserialize and to_stored_raw
- `Cargo.toml` - Add tempfile dev dependency

**Key Design Decisions:**
1. **Reuse ExportablePassword** - Same DTO for export/import, added Deserialize
2. **Deduplication** - Based on (location, password) tuple
3. **Progress Pattern** - Same `ProgressSender` pattern as export/migration
4. **No deletion** - Existing passwords preserved, only duplicates skipped
5. **user_id injection** - Provided at import time, not in file

**Import Stages:**
1. `Reading` - File read from disk
2. `Deserializing` - Parse JSON/CSV/XML
3. `Deduplicating` - Remove duplicates within file
4. `Encrypting` - Encrypt with user cipher
5. `Importing` - Save to database
6. `Completed` - Done

---

## Frontend Integration (Future)

This plan covers backend only. Frontend integration will need:
- File picker component
- Progress display (reuse `ProgressMigrationChn` pattern)
- Result display (imported count, skipped duplicates)
- Error handling UI
