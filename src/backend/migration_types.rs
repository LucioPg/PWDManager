//! Tipi per il tracking del progresso della migrazione password o export.

use tokio::sync::mpsc::Sender;

/// Rappresenta lo stage corrente della migrazione password o export.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MigrationStage {
    #[default]
    Idle,
    Decrypting,
    Encrypting,
    Serializing, // Serializzazione per export
    Writing,     // Scrittura file
    Finalizing,
    Completed,
    Failed,
}

/// Messaggio di progresso inviato tramite canale mpsc.
#[derive(Clone, Debug)]
pub struct ProgressMessage {
    pub stage: MigrationStage,
    pub current: usize,
    pub total: usize,
}

impl ProgressMessage {
    /// Crea un nuovo messaggio di progresso.
    pub fn new(stage: MigrationStage, current: usize, total: usize) -> Self {
        Self { stage, current, total }
    }

    /// Calcola la percentuale di completamento (0-100).
    pub fn percentage(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.current * 100) / self.total
        }
    }
}

/// Type alias per il sender del progresso.
pub type ProgressSender = Sender<ProgressMessage>;
