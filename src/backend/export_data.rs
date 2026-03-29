//! Context per l'export delle password.
//!
//! Contiene i dati necessari per eseguire l'export:
//! - user_id: ID dell'utente corrente
//! - output_path: Path dove salvare il file
//! - format: Formato di export (JSON, CSV, XML)

use super::export_types::ExportFormat;
use std::path::PathBuf;

/// Dati di contesto per l'export delle password.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExportData {
    pub user_id: i64,
    pub output_path: PathBuf,
    pub format: ExportFormat,
}

impl ExportData {
    pub fn new(user_id: i64, output_path: PathBuf, format: ExportFormat) -> Self {
        Self {
            user_id,
            output_path,
            format,
        }
    }
}
