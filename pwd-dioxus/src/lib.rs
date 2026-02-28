//! Reusable Dioxus components for password handling.

// Icons: incluso se qualsiasi feature icons* è attiva
#[cfg(any(feature = "icons", feature = "icons-visibility",
          feature = "icons-action", feature = "icons-alert"))]
pub mod icons;

#[cfg(feature = "spinner")]
pub mod spinner;

#[cfg(feature = "modal")]
pub mod modal;

#[cfg(feature = "form")]
pub mod form;

#[cfg(feature = "secret-display")]
pub mod secret;

// password module: incluso per analyzer o handler
#[cfg(any(feature = "analyzer", feature = "handler"))]
pub mod password;

// === RE-EXPORTS per convenienza ===

#[cfg(feature = "form")]
pub use form::{FormSecret, FormValue, InputType};

#[cfg(feature = "analyzer")]
pub use password::StrengthAnalyzer;

#[cfg(feature = "handler")]
pub use password::{PasswordHandler, EvaluationResult};

// Re-export tipi pwd-types usati nelle API pubbliche
// I consumer possono usarli senza aggiungere pwd-types alle dipendenze
#[cfg(any(feature = "analyzer", feature = "handler"))]
pub use pwd_types::{PasswordScore, PasswordStrength};

#[cfg(feature = "handler")]
pub use pwd_types::PasswordChangeResult;
