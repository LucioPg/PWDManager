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
