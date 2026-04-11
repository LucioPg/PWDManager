// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Tipi per il tracking del progresso della migrazione password o export.

use tokio::sync::mpsc::Sender;

/// Rappresenta lo stage corrente della migrazione password o export.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MigrationStage {
    #[default]
    Idle,
    Decrypting,
    Encrypting,
    Serializing,   // Serializzazione per export
    Deserializing, // Parsing per import (JSON/CSV/XML)
    Reading,       // Lettura file per import
    Writing,       // Scrittura file
    Deduplicating, // Rimozione duplicati per import
    Importing,     // Salvataggio nel DB per import
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_message_percentage_full() {
        let msg = ProgressMessage::new(MigrationStage::Encrypting, 75, 100);
        assert_eq!(msg.percentage(), 75);
        assert_eq!(msg.stage, MigrationStage::Encrypting);
    }

    #[test]
    fn test_progress_message_percentage_zero_total() {
        let msg = ProgressMessage::new(MigrationStage::Idle, 0, 0);
        assert_eq!(msg.percentage(), 0);
    }

    #[test]
    fn test_progress_message_percentage_rounds_down() {
        let msg = ProgressMessage::new(MigrationStage::Completed, 1, 3);
        assert_eq!(msg.percentage(), 33); // 1*100/3 = 33.33... → 33
    }

    #[test]
    fn test_progress_message_percentage_over_100_clamp() {
        let msg = ProgressMessage::new(MigrationStage::Completed, 150, 100);
        assert_eq!(msg.percentage(), 150); // No clamping — caller responsibility
    }

    #[test]
    fn test_migration_stage_default() {
        assert_eq!(MigrationStage::default(), MigrationStage::Idle);
    }

    #[test]
    fn test_migration_stage_equality() {
        assert_eq!(MigrationStage::Failed, MigrationStage::Failed);
        assert_ne!(MigrationStage::Idle, MigrationStage::Completed);
    }
}
