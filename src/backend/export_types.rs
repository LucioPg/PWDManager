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
