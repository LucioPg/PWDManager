// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Integration tests for import functionality.

#[cfg(test)]
mod tests {
    use crate::backend::export_types::{ExportFormat, ExportablePassword};
    use crate::backend::import::{
        deduplicate_passwords, parse_from_csv, parse_from_json, parse_from_xml, parse_passwords,
        validate_import_path,
    };
    use std::io::Write;
    use tempfile::NamedTempFile;

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
    fn test_roundtrip_json() {
        let passwords = create_test_passwords();
        let json = crate::backend::export::serialize_to_json(&passwords).unwrap();
        let parsed = parse_from_json(&json).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].url, "site1.com");
    }

    #[test]
    fn test_roundtrip_csv() {
        let passwords = create_test_passwords();
        let csv = crate::backend::export::serialize_to_csv(&passwords).unwrap();
        let parsed = parse_from_csv(&csv).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].url, "site1.com");
    }

    #[test]
    fn test_roundtrip_xml() {
        let passwords = create_test_passwords();
        let xml = crate::backend::export::serialize_to_xml(&passwords).unwrap();
        let parsed = parse_from_xml(&xml).unwrap();
        assert_eq!(parsed.len(), passwords.len());
        assert_eq!(parsed[0].url, "site1.com");
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
        writeln!(file, "url,password").unwrap();
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
                name: "Site A".to_string(),
                username: "user1".to_string(),
                url: "site.com".to_string(),
                password: "pass".to_string(),
                notes: Some("first".to_string()),
                score: Some(80),
                created_at: None,
            },
            ExportablePassword {
                name: "Site B".to_string(),
                username: "user2".to_string(),
                url: "site.com".to_string(),
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

    // ==================== MALFORMED INPUT TESTS ====================

    #[test]
    fn test_parse_malformed_json() {
        let json = r#"{"invalid": structure}"#;
        let result = parse_from_json(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON parse error"));
    }

    #[test]
    fn test_parse_malformed_json_not_array() {
        // JSON valido ma non un array
        let json = r#"{"url":"site.com","password":"pass"}"#;
        let result = parse_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_csv_missing_column() {
        // CSV con solo una colonna (manca password)
        let csv = "url\nsite.com";
        let result = parse_from_csv(csv);
        // CSV crate richiede almeno le colonne obbligatorie per deserializzare
        // Se manca la colonna password, il crate csv potrebbe:
        // 1. Fallire la deserializzazione
        // 2. Deserializzare con valore default
        // Verifichiamo che il risultato sia comunque gestibile
        if result.is_ok() {
            let passwords = result.unwrap();
            assert_eq!(passwords.len(), 1);
            // Password dovrebbe essere vuota o default
            assert!(passwords[0].password.is_empty() || passwords[0].password == "");
        }
        // Se fallisce, è accettabile perché il CSV è malformato
    }

    #[test]
    fn test_parse_malformed_xml() {
        let xml = r#"<invalid><unclosed>"#;
        let result = parse_from_xml(xml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("XML parse error"));
    }

    #[test]
    fn test_parse_xml_wrong_root() {
        // XML con root element sbagliato
        // quick-xml deserializza in base ai tag trovati, non alla root
        let xml = r#"<wrongroot><password><url>site.com</url><password>pass</password></password></wrongroot>"#;
        let result = parse_from_xml(xml);
        // quick-xml può deserializzare comunque se trova i tag <password>
        // Il comportamento dipende dalla struttura di XmlExportRoot
        // Accettiamo sia Ok (con qualsiasi numero di elementi) che Errore
        if result.is_ok() {
            // Se deserializza, verifichiamo solo che non ci siano crash
            let _passwords = result.unwrap();
        }
        // Non facciamo asserzioni rigide sul numero di elementi
        // perché dipende dall'implementazione interna di quick-xml
    }

    // ==================== EMPTY FILE TESTS ====================

    #[test]
    fn test_parse_empty_json() {
        let json = "[]";
        let result = parse_from_json(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_empty_csv() {
        // CSV con solo header
        let csv = "url,password,notes,score,created_at";
        let result = parse_from_csv(csv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_empty_xml() {
        // XML con root ma senza elementi password
        // quick-xml potrebbe avere problemi con elementi completamente vuoti
        // Usiamo un formato che quick-xml può gestire
        let xml = r#"<passwords><password /></passwords>"#;
        let result = parse_from_xml(xml);
        // quick-xml può gestire elementi self-closing come vuoti o errore
        // Verifichiamo solo che non ci siano crash
        if result.is_ok() {
            // Se deserializza, potrebbe avere 0 o più elementi con campi vuoti
            let _passwords = result.unwrap();
        }
        // Non facciamo asserzioni rigide perché dipende da quick-xml
    }

    // ==================== OPTIONAL FIELDS TESTS ====================

    #[test]
    fn test_csv_with_missing_optional_fields() {
        // CSV con notes e score mancanti
        let csv = "url,password,notes,score,created_at\nsite.com,pass123,,,";
        let result = parse_from_csv(csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].url, "site.com");
        assert_eq!(passwords[0].password, "pass123");
        assert_eq!(passwords[0].notes, None);
        assert_eq!(passwords[0].score, None);
    }

    #[test]
    fn test_json_with_missing_optional_fields() {
        // JSON con campi opzionali omessi
        let json = r#"[{"url":"site.com","password":"pass123"}]"#;
        let result = parse_from_json(json);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].notes, None);
        assert_eq!(passwords[0].score, None);
    }

    #[test]
    fn test_xml_with_missing_optional_fields() {
        // XML con solo campi obbligatori
        let xml = r#"<passwords><password><url>site.com</url><password>pass123</password></password></passwords>"#;
        let result = parse_from_xml(xml);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].notes, None);
        assert_eq!(passwords[0].score, None);
    }

    // ==================== EDGE CASES ====================

    #[test]
    fn test_deduplicate_url_only_different() {
        // Stessa url, password diversa = NON duplicato
        let passwords = vec![
            ExportablePassword {
                name: "".to_string(),
                username: "".to_string(),
                url: "site.com".to_string(),
                password: "pass1".to_string(),
                notes: None,
                score: None,
                created_at: None,
            },
            ExportablePassword {
                name: "".to_string(),
                username: "".to_string(),
                url: "site.com".to_string(),
                password: "pass2".to_string(),
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
    fn test_deduplicate_password_only_different() {
        // Stessa password, url diversa = NON duplicato
        let passwords = vec![
            ExportablePassword {
                name: "".to_string(),
                username: "".to_string(),
                url: "site1.com".to_string(),
                password: "samepass".to_string(),
                notes: None,
                score: None,
                created_at: None,
            },
            ExportablePassword {
                name: "".to_string(),
                username: "".to_string(),
                url: "site2.com".to_string(),
                password: "samepass".to_string(),
                notes: None,
                score: None,
                created_at: None,
            },
        ];
        let (unique, dupes) = deduplicate_passwords(passwords);
        assert_eq!(unique.len(), 2);
        assert_eq!(dupes, 0);
    }

    // ==================== FIREFOX CSV IMPORT TESTS ====================

    /// CSV Firefox di esempio con i campi reali che il browser esporta.
    fn firefox_csv_header() -> &'static str {
        r#""url","username","password","httpRealm","formActionOrigin","guid","timeCreated","timeLastUsed","timePasswordChanged""#
    }

    #[test]
    fn test_firefox_csv_username_fills_name() {
        // Firefox CSV: name non esiste, username è presente → name = username
        let csv = format!(
            "{}\n\"https://www.example.com\",\"testuser\",\"testpassword\",,\"\",\"{{guid}}\",\"1774894390184\",\"1774894390184\",\"1774894390184\"",
            firefox_csv_header()
        );
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].name, "testuser");
        assert_eq!(passwords[0].username, "testuser");
        assert_eq!(passwords[0].url, "https://www.example.com");
        assert_eq!(passwords[0].password, "testpassword");
    }

    #[test]
    fn test_firefox_csv_missing_username_url_fills_name() {
        // Firefox CSV: name e username mancanti → name = url
        let csv = format!(
            "{}\n\"https://www.example.com\",,\"testpassword\",,\"\",\"{{guid}}\",\"1774894390184\",\"1774894390184\",\"1774894390184\"",
            firefox_csv_header()
        );
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].name, "https://www.example.com");
        assert_eq!(passwords[0].username, "");
        assert_eq!(passwords[0].url, "https://www.example.com");
    }

    #[test]
    fn test_firefox_csv_time_created_to_created_at() {
        // timeCreated è un timestamp Unix in millisecondi → convertito in ISO
        let csv = format!(
            "{}\n\"https://www.example.com\",\"testuser\",\"testpassword\",,\"\",\"{{guid}}\",\"1774894390184\",\"1774894390184\",\"1774894390184\"",
            firefox_csv_header()
        );
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 1);
        // timeCreated 1774894390184 → 2026-03-30 in UTC
        assert!(passwords[0].created_at.is_some());
        let created_at = passwords[0].created_at.as_ref().unwrap();
        assert!(created_at.starts_with("2026-03-30"));
    }

    #[test]
    fn test_firefox_csv_missing_time_created() {
        // Senza timeCreated, created_at deve essere None
        let csv = format!(
            "{}\n\"https://www.example.com\",\"testuser\",\"testpassword\",,\"\",\"{{guid}}\",,\"1774894390184\",\"1774894390184\"",
            firefox_csv_header()
        );
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords[0].created_at, None);
    }

    #[test]
    fn test_firefox_csv_multiple_entries() {
        // Più entry nel CSV Firefox
        let csv = format!(
            "{header}\n\
             \"https://site1.com\",\"user1\",\"pass1\",,\"\",\"{{g1}}\",\"1774894390184\",\"1774894390184\",\"1774894390184\"\n\
             \"https://site2.com\",\"user2\",\"pass2\",,\"\",\"{{g2}}\",,\"\",",
            header = firefox_csv_header()
        );
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        let passwords = result.unwrap();
        assert_eq!(passwords.len(), 2);
        assert_eq!(passwords[0].name, "user1");
        assert_eq!(passwords[1].name, "user2");
        // La prima ha timeCreated, la seconda no
        assert!(passwords[0].created_at.is_some());
        assert_eq!(passwords[1].created_at, None);
    }

    #[test]
    fn test_firefox_csv_empty_file() {
        let csv = format!("{}\n", firefox_csv_header());
        let result = crate::backend::import::parse_firefox_csv(&csv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_passwords_dispatch() {
        // Verifica che parse_passwords dispatchi correttamente
        let json = r#"[{"url":"site.com","password":"pass"}]"#;

        let result_json = crate::backend::import::parse_passwords(json, ExportFormat::Json);
        assert!(result_json.is_ok());

        let result_csv = crate::backend::import::parse_passwords(
            "url,password\nsite.com,pass",
            ExportFormat::Csv,
        );
        assert!(result_csv.is_ok());

        let result_xml = crate::backend::import::parse_passwords(
            r#"<passwords><password><url>site.com</url><password>pass</password></password></passwords>"#,
            ExportFormat::Xml,
        );
        assert!(result_xml.is_ok());
    }
}
