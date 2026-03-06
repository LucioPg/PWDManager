//! Test di integrazione per il modulo export.

#[cfg(test)]
mod tests {
    use crate::backend::export::{serialize_passwords, validate_export_path};
    use crate::backend::export_types::{ExportFormat, ExportablePassword};
    use std::path::PathBuf;

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
    fn test_json_format_contains_all_fields() {
        let passwords = create_test_passwords();
        let json = serialize_passwords(&passwords, ExportFormat::Json).unwrap();

        assert!(json.contains("site1.com"));
        assert!(json.contains("pass1"));
        assert!(json.contains("note1"));
        assert!(json.contains("80"));
    }

    #[test]
    fn test_csv_format_has_header() {
        let passwords = create_test_passwords();
        let csv = serialize_passwords(&passwords, ExportFormat::Csv).unwrap();

        assert!(csv.contains("location"));
        assert!(csv.contains("password"));
        assert!(csv.contains("site1.com"));
    }

    #[test]
    fn test_xml_format_has_root_element() {
        let passwords = create_test_passwords();
        let xml = serialize_passwords(&passwords, ExportFormat::Xml).unwrap();

        assert!(xml.contains("<passwords>"));
        assert!(xml.contains("</passwords>"));
        assert!(xml.contains("<location>site1.com</location>"));
    }

    #[test]
    fn test_empty_passwords_serialization() {
        let empty: Vec<ExportablePassword> = vec![];

        assert!(serialize_passwords(&empty, ExportFormat::Json).is_ok());
        assert!(serialize_passwords(&empty, ExportFormat::Csv).is_ok());
        assert!(serialize_passwords(&empty, ExportFormat::Xml).is_ok());
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

    #[test]
    fn test_validate_export_path_directory_not_exists() {
        let path = PathBuf::from("/nonexistent/directory/file.json");
        let result = validate_export_path(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Directory does not exist"));
    }

    #[test]
    fn test_validate_export_path_is_directory() {
        // Test con una directory esistente come target (dovrebbe fallire)
        let path = std::env::current_dir().unwrap();
        let result = validate_export_path(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is a directory"));
    }

    #[test]
    fn test_validate_export_path_valid() {
        // Test con un path valido nella directory corrente
        let path = std::env::current_dir().unwrap().join("test_export.json");
        let result = validate_export_path(&path);
        assert!(result.is_ok());
    }
}
