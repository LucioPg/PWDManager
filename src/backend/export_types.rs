// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Tipi per l'export delle password in vari formati.

use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

/// Formato di export supportato.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ExportFormat {
    #[default]
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

}

/// DTO per la serializzazione di una password in export.
///
/// Questo tipo "apre" i SecretString tramite `.expose_secret()`
/// per consentire la serializzazione in chiaro nel file di export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportablePassword {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub username: String,
    pub url: String,
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

impl ExportablePassword {
    /// Crea un ExportablePassword da una StoredRawPassword.
    ///
    /// Usa `.expose_secret()` per convertire i SecretString in String.
    pub fn from_stored_raw(stored: &pwd_types::StoredRawPassword) -> Self {
        Self {
            name: stored.name.clone(),
            username: stored.username.expose_secret().to_string(),
            url: stored.url.expose_secret().to_string(),
            password: stored.password.expose_secret().to_string(),
            notes: stored.notes.as_ref().map(|n| n.expose_secret().to_string()),
            score: stored.score.map(|s| s.value()),
            created_at: stored.created_at.clone(),
        }
    }

    /// Converte un ExportablePassword in StoredRawPassword per l'import.
    ///
    /// Crea un nuovo UUID e assegna lo user_id e vault_id forniti.
    /// `id` è None (nuovo record, sarà assegnato dal DB).
    /// `created_at` preserva il timestamp originale dal file di import.
    pub fn to_stored_raw(&self, user_id: i64, vault_id: i64) -> pwd_types::StoredRawPassword {
        use pwd_types::PasswordScore;
        use secrecy::SecretString;
        use uuid::Uuid;

        pwd_types::StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None, // Nuovo record, sarà assegnato dal DB
            user_id,
            vault_id,
            name: self.name.clone(),
            username: SecretString::new(self.username.clone().into()),
            url: SecretString::new(self.url.clone().into()),
            password: SecretString::new(self.password.clone().into()),
            notes: self
                .notes
                .as_ref()
                .map(|n| SecretString::new(n.clone().into())),
            score: self.score.map(PasswordScore::new),
            created_at: self.created_at.clone(),
        }
    }
}

/// Wrapper per la serializzazione XML con elemento root.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "passwords")]
pub struct XmlExportRoot {
    #[serde(rename = "password")]
    pub passwords: Vec<ExportablePassword>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    fn make_stored_raw(url: &str, password: &str, score: Option<u8>) -> pwd_types::StoredRawPassword {
        pwd_types::StoredRawPassword {
            uuid: uuid::Uuid::new_v4(),
            id: None,
            user_id: 42,
            vault_id: 1,
            name: format!("Name_{}", url),
            username: SecretString::new(format!("user@{}", url).into()),
            url: SecretString::new(url.into()),
            password: SecretString::new(password.into()),
            notes: Some(SecretString::new("test notes".into())),
            score: score.map(pwd_types::PasswordScore::new),
            created_at: Some("2024-06-15".to_string()),
        }
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Xml.extension(), "xml");
    }

    #[test]
    fn test_export_format_default() {
        assert_eq!(ExportFormat::default(), ExportFormat::Json);
    }

    #[test]
    fn test_from_stored_raw_exposes_secrets() {
        let stored = make_stored_raw("site.com", "secret123", Some(85));
        let exportable = ExportablePassword::from_stored_raw(&stored);

        assert_eq!(exportable.url, "site.com");
        assert_eq!(exportable.password, "secret123");
        assert_eq!(exportable.username, "user@site.com");
        assert_eq!(exportable.notes, Some("test notes".to_string()));
        assert_eq!(exportable.score, Some(85));
        assert_eq!(exportable.created_at, Some("2024-06-15".to_string()));
        assert_eq!(exportable.name, "Name_site.com");
    }

    #[test]
    fn test_from_stored_raw_none_optional_fields() {
        let mut stored = make_stored_raw("site.com", "pass", None);
        stored.notes = None;
        stored.score = None;
        stored.created_at = None;

        let exportable = ExportablePassword::from_stored_raw(&stored);

        assert_eq!(exportable.notes, None);
        assert_eq!(exportable.score, None);
        assert_eq!(exportable.created_at, None);
    }

    #[test]
    fn test_to_stored_raw_roundtrip() {
        let stored = make_stored_raw("site.com", "secret123", Some(85));
        let exportable = ExportablePassword::from_stored_raw(&stored);
        let back = exportable.to_stored_raw(99, 7);

        assert_eq!(back.user_id, 99);
        assert_eq!(back.vault_id, 7);
        assert_eq!(back.url.expose_secret(), "site.com");
        assert_eq!(back.password.expose_secret(), "secret123");
        assert_eq!(back.username.expose_secret(), "user@site.com");
        assert!(back.id.is_none());
        assert!(back.uuid != stored.uuid); // New UUID generated
    }

    #[test]
    fn test_to_stored_raw_preserves_optional_fields() {
        let mut stored = make_stored_raw("site.com", "pass", None);
        stored.notes = None;
        stored.created_at = None;

        let exportable = ExportablePassword::from_stored_raw(&stored);
        let back = exportable.to_stored_raw(1, 1);

        assert!(back.notes.is_none());
        assert!(back.created_at.is_none());
        assert!(back.score.is_none());
    }

    #[test]
    fn test_serialize_deserialize_json_roundtrip() {
        let exportable = ExportablePassword {
            name: "Test".to_string(),
            username: "user".to_string(),
            url: "site.com".to_string(),
            password: "pass".to_string(),
            notes: Some("notes".to_string()),
            score: Some(90),
            created_at: Some("2024-01-01".to_string()),
        };

        let json = serde_json::to_string(&exportable).unwrap();
        let back: ExportablePassword = serde_json::from_str(&json).unwrap();

        assert_eq!(back.url, exportable.url);
        assert_eq!(back.password, exportable.password);
        assert_eq!(back.notes, exportable.notes);
    }

    #[test]
    fn test_serialize_json_skips_none_fields() {
        let exportable = ExportablePassword {
            name: String::new(),
            username: String::new(),
            url: "site.com".to_string(),
            password: "pass".to_string(),
            notes: None,
            score: None,
            created_at: None,
        };

        let json = serde_json::to_string(&exportable).unwrap();
        assert!(!json.contains("notes"));
        assert!(!json.contains("score"));
        assert!(!json.contains("created_at"));
    }
}
