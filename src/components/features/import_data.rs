//! Context per l'import delle password.
//!
//! Contiene i dati necessari per eseguire l'import:
//! - user_id: ID dell'utente corrente
//! - input_path: Path del file da importare
//! - format: Formato di import (JSON, CSV, XML)

use crate::backend::export_types::ExportFormat;
use std::path::PathBuf;

/// Dati di contesto per l'import delle password.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportData {
    pub user_id: i64,
    pub input_path: PathBuf,
    pub format: ExportFormat,
}

impl ImportData {
    pub fn new(user_id: i64, input_path: PathBuf, format: ExportFormat) -> Self {
        Self {
            user_id,
            input_path,
            format,
        }
    }
}
