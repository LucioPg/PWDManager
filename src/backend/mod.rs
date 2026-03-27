pub mod avatar_utils;
pub mod db_backend;
pub mod export;
pub mod export_types;
pub mod import;
pub mod init_queries;
pub mod migration_types;
pub mod password_utils;
pub mod settings_types;
pub mod ui_utils;
pub mod updater;
pub mod updater_types;
pub mod utils;

#[cfg(feature = "desktop")]
pub mod db_key;

#[cfg(feature = "desktop")]
pub mod setup;

#[cfg(test)]
mod password_utils_tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod db_backend_tests;

#[cfg(test)]
mod db_settings_tests;

#[cfg(test)]
mod export_tests;

#[cfg(test)]
mod import_tests;

// Re-export pwd-strength for backward compatibility
pub use pwd_strength::{
    evaluate_password_strength, evaluate_password_strength_tx, init_blacklist,
    init_blacklist_from_path,
};
