//! Password strength evaluation library
//!
//! This library provides password strength evaluation functionality
//! with configurable blacklist support.
//!
//! # Features
//!
//! - `async` (default): Enables async evaluation with cancellation support
//! - `tracing`: Enables logging via tracing crate
//!
//! # Environment Variables
//!
//! - `PWD_BLACKLIST_PATH`: Custom path to blacklist file
//!   (default: `./assets/10k-most-common.txt`)
//!
//! # Example
//!
//! ```rust,no_run
//! use pwd_strength::{init_blacklist, evaluate_password_strength};
//! use secrecy::SecretString;
//!
//! // Initialize blacklist (call once at startup)
//! init_blacklist().expect("Failed to load blacklist");
//!
//! // Evaluate a password
//! let password = SecretString::new("MyP@ssw0rd!".to_string().into());
//!
//! #[cfg(feature = "async")]
//! let evaluation = evaluate_password_strength(&password, None);
//!
//! #[cfg(not(feature = "async"))]
//! let evaluation = evaluate_password_strength(&password);
//!
//! println!("Score: {:?}", evaluation.score);
//! println!("Strength: {:?}", evaluation.strength());
//! ```

// Re-export types from pwd-types for convenience
pub use pwd_types::{PasswordEvaluation, PasswordScore, PasswordStrength};

// Internal modules
mod blacklist;
mod evaluator;
mod sections;

// Public API
pub use blacklist::{init_blacklist, get_blacklist, is_blacklisted, BlacklistError};
pub use evaluator::evaluate_password_strength;

#[cfg(feature = "async")]
pub use evaluator::evaluate_password_strength_tx;
