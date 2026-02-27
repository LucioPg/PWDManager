//! Tipi puri per la gestione delle password.
//!
//! Questo crate fornisce tipi condivisi per:
//! - Valutazione forza password (score, strength)
//! - Statistiche password
//! - Wrapper secrecy per SQLx
//! - Configurazione generazione password
//! - Risultato cambio password per callback UI

// Core types (sempre disponibili)
mod score;
pub use score::{PasswordScore, PasswordStrength, PasswordEvaluation};

mod stats;
pub use stats::PasswordStats;

// Optional: secrecy support
#[cfg(feature = "secrecy")]
pub use secrecy::{SecretBox, SecretString};

// Optional: Password change result (richiede secrecy)
#[cfg(feature = "secrecy")]
mod change_result;
#[cfg(feature = "secrecy")]
pub use change_result::PasswordChangeResult;

// Optional: SQLx database types
#[cfg(feature = "sqlx")]
mod secrets;
#[cfg(feature = "sqlx")]
pub use secrets::{DbSecretString, DbSecretVec, SecretSliceU8};

#[cfg(feature = "sqlx")]
mod stored;
#[cfg(feature = "sqlx")]
pub use stored::{UserAuth, StoredPassword, StoredRawPassword};

// Optional: password generator config (richiede sqlx per SqlxTemplate)
#[cfg(all(feature = "generator", feature = "sqlx"))]
mod generator;
#[cfg(all(feature = "generator", feature = "sqlx"))]
pub use generator::{PasswordGeneratorConfig, PasswordPreset, ExcludedSymbolSet, AegisPasswordConfig};
