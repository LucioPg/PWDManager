# Password Export Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

## 📊 Progress Status

| Task | Status | Commit |
|------|--------|--------|
| Task 1: Estendi MigrationStage per Export | ✅ COMPLETATO | `74e8e5b` |
| Task 2: Crea DTO per Serializzazione Export | ✅ COMPLETATO | `429773c` |
| Task 3: Implementa Serializzatori | ✅ COMPLETATO | `ad690bf` |
| Task 4: Implementa Pipeline Export con Progress Tracking | ✅ COMPLETATO | `58ae12b` |
| Task 5: Helper per Path di Export | ✅ COMPLETATO | `bcb54d4` |
| Task 6: Test di Integrazione Export | ✅ COMPLETATO | `174215e` |

**Ultima sessione:** Batch 2 completato (Task 4-6). Piano completato.

---

**Goal:** Implementare l'export delle password decryptate in formati JSON, CSV e XML con progress tracking usando lo stesso pattern di `stored_passwords_migration_pipeline_with_progress`.

**Architecture:** Estendere `MigrationStage` esistente con nuovi stage per l'export e riutilizzare `ProgressSender`/`ProgressMessage` da `migration_types.rs`. Creare un modulo `export.rs` che implementa la pipeline di export seguendo esattamente il pattern della migrazione password.

**Tech Stack:** Rust, tokio (async, fs, mpsc), serde (JSON), csv crate, quick-xml, secrecy (SecretString)

---

## Task 1: Estendi MigrationStage per Export

**Files:**
- Modify: `src/backend/migration_types.rs`

**Step 1: Aggiungi dipendenze CSV e XML**

In `Cargo.toml`, aggiungi nel section `[dependencies]`:

```toml
csv = "1.3"
quick-xml = { version = "0.37", features = ["serialize"] }
chrono = "0.4"
```

**Step 2: Esegui cargo check**

Run: `cargo check`
Expected: Compila senza errori

**Step 3: Estendi MigrationStage con stage per export**

In `src/backend/migration_types.rs`, modifica l'enum `MigrationStage`:

```rust
/// Rappresenta lo stage corrente della migrazione password o export.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MigrationStage {
    #[default]
    Idle,
    Decrypting,
    Encrypting,
    Serializing,  // Nuovo: serializzazione per export
    Writing,      // Nuovo: scrittura file
    Finalizing,
    Completed,
    Failed,
}
```

**Step 4: Esegui cargo check**

Run: `cargo check`
Expected: Compila senza errori (gli stage nuovi sono retrocompatibili)

**Step 5: Commit**

```bash
git add Cargo.toml src/backend/migration_types.rs
git commit -m "feat(export): add Serializing and Writing stages to MigrationStage"
```

---

## Task 2: Crea DTO per Serializzazione Export

**Files:**
- Create: `src/backend/export_types.rs`
- Modify: `src/backend/mod.rs`

**Step 1: Crea export_types.rs con DTO**

Crea `src/backend/export_types.rs`:

```rust
//! Tipi per l'export delle password in vari formati.

use serde::Serialize;
use secrecy::ExposeSecret;

/// Formato di export supportato.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Xml,
}

impl ExportFormat {
    /// Restituisce l'estensione file per il formato.
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Xml => "xml",
        }
    }

    /// Restituisce il MIME type per il formato.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Csv => "text/csv",
            ExportFormat::Xml => "application/xml",
        }
    }
}

/// DTO per la serializzazione di una password in export.
///
/// Questo tipo "apre" i SecretString tramite `.expose_secret()`
/// per consentire la serializzazione in chiaro nel file di export.
#[derive(Debug, Serialize)]
pub struct ExportablePassword {
    pub location: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

impl ExportablePassword {
    /// Crea un ExportablePassword da una StoredRawPassword.
    ///
    /// Usa `.expose_secret()` per convertire i SecretString in String.
    pub fn from_stored_raw(stored: &pwd_types::StoredRawPassword) -> Self {
        Self {
            location: stored.location.expose_secret().to_string(),
            password: stored.password.expose_secret().to_string(),
            notes: stored.notes.as_ref().map(|n| n.expose_secret().to_string()),
            score: stored.score.map(|s| s.value()),
            created_at: stored.created_at.clone(),
        }
    }
}

/// Wrapper per la serializzazione XML con elemento root.
#[derive(Debug, Serialize)]
#[serde(rename = "passwords")]
pub struct XmlExportRoot {
    #[serde(rename = "password")]
    pub passwords: Vec<ExportablePassword>,
}
```

**Step 2: Registra il modulo in mod.rs**

In `src/backend/mod.rs`, aggiungi dopo `pub mod migration_types;`:

```rust
pub mod export_types;
```

**Step 3: Esegui cargo check**

Run: `cargo check`
Expected: Compila senza errori

**Step 4: Commit**

```bash
git add src/backend/export_types.rs src/backend/mod.rs
git commit -m "feat(export): add ExportFormat and ExportablePassword DTO"
```

---

## Task 3: Implementa Serializzatori

**Files:**
- Create: `src/backend/export.rs`
- Modify: `src/backend/mod.rs`

**Step 1: Crea export.rs con funzioni di serializzazione**

Crea `src/backend/export.rs`:

```rust
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

use crate::backend::export_types::{ExportFormat, ExportablePassword, XmlExportRoot};
use pwd_types::StoredRawPassword;
use quick_xml::se::to_string as xml_to_string;

/// Serializza le password in formato JSON (pretty-printed).
pub fn serialize_to_json(passwords: &[ExportablePassword]) -> Result<String, String> {
    serde_json::to_string_pretty(passwords)
        .map_err(|e| format!("JSON serialization error: {}", e))
}

/// Serializza le password in formato CSV.
pub fn serialize_to_csv(passwords: &[ExportablePassword]) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    for pwd in passwords {
        wtr.serialize(pwd)
            .map_err(|e| format!("CSV serialization error: {}", e))?;
    }

    wtr.into_inner()
        .map_err(|e| format!("CSV writer error: {}", e))?
        .into_string()
        .map_err(|e| format!("CSV UTF-8 error: {:?}", e))
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
        assert!(csv.contains("location"));
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
```

**Step 2: Registra il modulo in mod.rs**

In `src/backend/mod.rs`, aggiungi dopo `pub mod export_types;`:

```rust
pub mod export;
```

**Step 3: Esegui i test**

Run: `cargo test serialize_to --lib -- --nocapture`
Expected: Tutti e 3 i test passano

**Step 4: Commit**

```bash
git add src/backend/export.rs src/backend/mod.rs
git commit -m "feat(export): add serialization functions for JSON, CSV, XML"
```

---

## Task 4: Implementa Pipeline Export con Progress Tracking

**Files:**
- Modify: `src/backend/export.rs`

**Step 1: Aggiungi la pipeline di export**

In `src/backend/export.rs`, aggiungi prima di `#[cfg(test)]`:

```rust
use crate::backend::db_backend::fetch_user_auth_from_id;
use std::path::PathBuf;

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
    // Invia stato iniziale
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Decrypting, 0, 0))
            .await;
    }

    // 1. Fetch StoredPassword crittografate dal database
    let stored_passwords = fetch_all_stored_passwords_for_user(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    let total = stored_passwords.len();

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
    let user_auth = fetch_user_auth_from_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Decrypt con progress tracking (stesso pattern della migrazione)
    // Passiamo progress_tx direttamente a decrypt_bulk_stored_data
    let raw_passwords = decrypt_bulk_stored_data(user_auth, stored_passwords, progress_tx.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Invia cambio stage - Serializing
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Serializing, 0, total))
            .await;
    }

    // 4. Converti in ExportablePassword con progress tracking
    // ExportablePassword::from_stored_raw() chiama .expose_secret()
    let exportable_passwords =
        convert_to_exportable_with_progress(raw_passwords, progress_tx.clone(), total);

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
                    let _ = tx.blocking_send(ProgressMessage::new(
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
```

**Step 2: Esegui cargo check**

Run: `cargo check`
Expected: Compila senza errori

**Step 3: Commit**

```bash
git add src/backend/export.rs
git commit -m "feat(export): add export pipeline with progress tracking"
```

---

## Task 5: Helper per Path di Export

**Files:**
- Modify: `src/backend/export.rs`

**Step 1: Aggiungi funzioni helper**

In `src/backend/export.rs`, aggiungi prima di `#[cfg(test)]`:

```rust
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
pub async fn validate_export_path(path: &Path) -> Result<(), String> {
    // Verifica che la directory padre esista
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "Directory does not exist: {}",
                parent.display()
            ));
        }
    }

    // Verifica che non sia una directory
    if path.exists() && path.is_dir() {
        return Err(format!("Path is a directory, not a file: {}", path.display()));
    }

    Ok(())
}
```

**Step 2: Esegui cargo check**

Run: `cargo check`
Expected: Compila senza errori

**Step 3: Commit**

```bash
git add src/backend/export.rs
git commit -m "feat(export): add helper functions for export path generation"
```

---

## Task 6: Test di Integrazione Export

**Files:**
- Create: `src/backend/export_tests.rs`
- Modify: `src/backend/mod.rs`

**Step 1: Crea i test di integrazione**

Crea `src/backend/export_tests.rs`:

```rust
//! Test di integrazione per il modulo export.

#[cfg(test)]
mod tests {
    use crate::backend::export::{serialize_to_csv, serialize_to_json, serialize_to_xml};
    use crate::backend::export_types::{ExportFormat, ExportablePassword};

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
    fn test_json_format_contains_all_fields() {
        let passwords = create_test_passwords();
        let json = serialize_to_json(&passwords).unwrap();

        assert!(json.contains("site1.com"));
        assert!(json.contains("pass1"));
        assert!(json.contains("note1"));
        assert!(json.contains("80"));
    }

    #[test]
    fn test_csv_format_has_header() {
        let passwords = create_test_passwords();
        let csv = serialize_to_csv(&passwords).unwrap();

        assert!(csv.contains("location"));
        assert!(csv.contains("password"));
        assert!(csv.contains("site1.com"));
    }

    #[test]
    fn test_xml_format_has_root_element() {
        let passwords = create_test_passwords();
        let xml = serialize_to_xml(&passwords).unwrap();

        assert!(xml.contains("<passwords>"));
        assert!(xml.contains("</passwords>"));
        assert!(xml.contains("<location>site1.com</location>"));
    }

    #[test]
    fn test_empty_passwords_serialization() {
        let empty: Vec<ExportablePassword> = vec![];

        assert!(serialize_to_json(&empty).is_ok());
        assert!(serialize_to_csv(&empty).is_ok());
        assert!(serialize_to_xml(&empty).is_ok());
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Xml.extension(), "xml");
    }

    #[test]
    fn test_export_format_mime_type() {
        assert_eq!(ExportFormat::Json.mime_type(), "application/json");
        assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
        assert_eq!(ExportFormat::Xml.mime_type(), "application/xml");
    }
}
```

**Step 2: Registra il modulo test in mod.rs**

In `src/backend/mod.rs`, aggiungi dopo `#[cfg(test)] mod db_settings_tests;`:

```rust
#[cfg(test)]
mod export_tests;
```

**Step 3: Esegui i test**

Run: `cargo test export --lib -- --nocapture`
Expected: Tutti i test passano

**Step 4: Commit**

```bash
git add src/backend/export_tests.rs src/backend/mod.rs
git commit -m "test(export): add integration tests for export serialization"
```

---

## Summary

Il piano implementa l'export delle password decryptate seguendo **esattamente lo stesso pattern** della migrazione:

### Pattern Riutilizzato
- **`ProgressSender`** e **`ProgressMessage`** da `migration_types.rs` (nessun tipo duplicato)
- **`MigrationStage`** esteso con `Serializing` e `Writing`
- Passaggio diretto di `progress_tx` a `decrypt_bulk_stored_data`
- `Arc<AtomicUsize>` per progress tracking durante operazioni CPU-bound

### Flusso Dati
```
Database (StoredPassword criptate)
         ↓ fetch_all_stored_passwords_for_user
StoredPassword[]
         ↓ decrypt_bulk_stored_data (con progress_tx)
StoredRawPassword[] (SecretString)
         ↓ ExportablePassword::from_stored_raw()
         ↓ chiama .expose_secret() sui SecretString
ExportablePassword[] (String in chiaro)
         ↓ serialize_passwords
String (JSON/CSV/XML)
         ↓ fs::write
File
```

### Stages Export
1. `Decrypting` - durante `decrypt_bulk_stored_data`
2. `Serializing` - durante conversione in `ExportablePassword`
3. `Writing` - durante scrittura file
4. `Completed` - fine

---

## Files Creati/Modificati

| File | Azione |
|------|--------|
| `Cargo.toml` | Aggiunge `csv`, `quick-xml`, `chrono` |
| `src/backend/migration_types.rs` | Estende `MigrationStage` con `Serializing`, `Writing` |
| `src/backend/export_types.rs` | Nuovo - `ExportFormat`, `ExportablePassword`, `XmlExportRoot` |
| `src/backend/export.rs` | Nuovo - serializzazione e pipeline export |
| `src/backend/export_tests.rs` | Nuovo - test |
| `src/backend/mod.rs` | Registra nuovi moduli |
