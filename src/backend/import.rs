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
use std::path::Path;

/// Parse JSON content into ExportablePassword list.
pub fn parse_from_json(content: &str) -> Result<Vec<ExportablePassword>, String> {
    serde_json::from_str(content).map_err(|e| format!("JSON parse error: {}", e))
}

/// Parse CSV content into ExportablePassword list.
pub fn parse_from_csv(content: &str) -> Result<Vec<ExportablePassword>, String> {
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
