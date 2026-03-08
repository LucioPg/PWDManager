mod table;
mod table_row;

use pwd_types::StoredRawPassword;

pub use table::*;
pub use table_row::{StoredRawPasswordRow, StoredRawPasswordRowProps};

/// Stato del tooltip delle note, gestito a livello di tabella.
/// Contiene i dati della password e le coordinate dove mostrare il tooltip.
#[derive(Clone, Debug, Default)]
pub struct TooltipState {
    pub password: Option<StoredRawPassword>,
    pub x: f64,
    pub y: f64,
}

impl TooltipState {
    pub fn new(password: StoredRawPassword, x: f64, y: f64) -> Self {
        Self {
            password: Some(password),
            x,
            y,
        }
    }

    pub fn is_open(&self) -> bool {
        self.password.is_some()
    }

    pub fn close(&mut self) {
        self.password = None;
    }
}
