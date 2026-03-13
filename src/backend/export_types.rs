//! Tipi per l'export delle password in vari formati.

use serde::{Deserialize, Serialize};
use secrecy::ExposeSecret;

/// Formato di export supportato.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Xml,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::Json
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportablePassword {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub username: String,
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

impl ExportablePassword {
    /// Crea un ExportablePassword da una StoredRawPassword.
    ///
    /// Usa `.expose_secret()` per convertire i SecretString in String.
    pub fn from_stored_raw(stored: &pwd_types::StoredRawPassword) -> Self {
        Self {
            name: stored.name.clone(),
            username: stored.username.expose_secret().to_string(),
            location: stored.location.expose_secret().to_string(),
            password: stored.password.expose_secret().to_string(),
            notes: stored.notes.as_ref().map(|n| n.expose_secret().to_string()),
            score: stored.score.map(|s| s.value()),
            created_at: stored.created_at.clone(),
        }
    }

    /// Converte un ExportablePassword in StoredRawPassword per l'import.
    ///
    /// Crea un nuovo UUID e assegna lo user_id fornito.
    /// `id` è None (nuovo record, sarà assegnato dal DB).
    /// `created_at` preserva il timestamp originale dal file di import.
    pub fn to_stored_raw(&self, user_id: i64) -> pwd_types::StoredRawPassword {
        use pwd_types::PasswordScore;
        use secrecy::SecretString;
        use uuid::Uuid;

        pwd_types::StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None, // Nuovo record, sarà assegnato dal DB
            user_id,
            name: self.name.clone(),
            username: SecretString::new(self.username.clone().into()),
            location: SecretString::new(self.location.clone().into()),
            password: SecretString::new(self.password.clone().into()),
            notes: self.notes.as_ref().map(|n| SecretString::new(n.clone().into())),
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
