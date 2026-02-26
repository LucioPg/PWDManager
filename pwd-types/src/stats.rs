//! Statistiche sulle password salvate.

/// Enum per tenere traccia delle statistiche delle password (usato nel frontend).
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PasswordStats {
    pub weak: usize,
    pub medium: usize,
    pub strong: usize,
    pub epic: usize,
    pub god: usize,
    pub total: usize,
    pub not_evaluated: usize,
}
