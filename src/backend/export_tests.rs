// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Test di integrazione per il modulo export.

#[cfg(test)]
mod tests {
    use crate::backend::export::serialize_passwords;
    use crate::backend::export_types::{ExportFormat, ExportablePassword};

    fn create_test_passwords() -> Vec<ExportablePassword> {
        vec![
            ExportablePassword {
                name: "Site 1".to_string(),
                username: "user1@site.com".to_string(),
                url: "site1.com".to_string(),
                password: "pass1".to_string(),
                notes: Some("note1".to_string()),
                score: Some(80),
                created_at: Some("2024-01-01".to_string()),
            },
            ExportablePassword {
                name: "Site 2".to_string(),
                username: "user2@site.com".to_string(),
                url: "site2.com".to_string(),
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

        assert!(csv.contains("url"));
        assert!(csv.contains("password"));
        assert!(csv.contains("site1.com"));
    }

    #[test]
    fn test_xml_format_has_root_element() {
        let passwords = create_test_passwords();
        let xml = serialize_passwords(&passwords, ExportFormat::Xml).unwrap();

        assert!(xml.contains("<passwords>"));
        assert!(xml.contains("</passwords>"));
        assert!(xml.contains("<url>site1.com</url>"));
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
}
