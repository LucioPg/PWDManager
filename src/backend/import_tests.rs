//! Integration tests for import functionality.

#[cfg(test)]
mod tests {
    use crate::backend::export_types::{ExportFormat, ExportablePassword};
    use crate::backend::import::{
        deduplicate_passwords, parse_from_csv, parse_from_json, parse_from_xml,
        validate_import_path,
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
                notes: Some("note2".to_string()),
                score: Some(90),
                created_at: Some("2024-01-02".to_string()),
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
