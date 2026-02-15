mod component;
mod strength_analyzer;

pub use component::PasswordHandler;
pub use strength_analyzer::StrengthAnalyzer;

// Re-export types per convenienza
pub use crate::backend::strength_utils::{PasswordEvaluation, PasswordStrength};
