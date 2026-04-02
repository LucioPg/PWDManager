// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Context per l'import delle password.
//!
//! Contiene i dati necessari per eseguire l'import:
//! - user_id: ID dell'utente corrente
//! - vault_id: ID del vault in cui importare
//! - input_path: Path del file da importare
//! - format: Formato di import (JSON, CSV, XML)

use super::export_types::ExportFormat;
use std::path::PathBuf;

/// Dati di contesto per l'import delle password.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportData {
    pub user_id: i64,
    pub vault_id: i64,
    pub input_path: PathBuf,
    pub format: ExportFormat,
}

impl ImportData {
    pub fn new(user_id: i64, vault_id: i64, input_path: PathBuf, format: ExportFormat) -> Self {
        Self {
            user_id,
            vault_id,
            input_path,
            format,
        }
    }
}
