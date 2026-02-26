pub mod settings_types;
pub mod db_backend;
pub mod init_queries;
pub(crate) mod password_types_helper;
pub(crate) mod password_utils;
pub mod ui_utils;
pub mod utils;

#[cfg(test)]
mod password_utils_tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod db_backend_tests;

#[cfg(test)]
mod db_settings_tests;

// Re-export pwd-strength for backward compatibility
pub use pwd_strength::{
    init_blacklist, is_blacklisted, evaluate_password_strength,
    evaluate_password_strength_tx,
};
