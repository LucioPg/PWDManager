// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#[cfg(target_os = "windows")]
pub mod auto_start;

pub mod avatar_utils;
pub mod db_backend;
pub mod export;
pub mod export_data;
pub mod export_types;
pub mod import;
pub mod import_data;
pub mod init_queries;
pub mod migration_types;
pub mod password_utils;
pub mod settings_types;
pub mod ui_utils;
pub mod vault_utils;
pub mod updater;
pub mod updater_types;
pub mod utils;

#[cfg(feature = "desktop")]
pub mod db_key;

#[cfg(feature = "desktop")]
pub mod hello_auth;

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

#[cfg(test)]
mod db_backend_coverage_tests;

#[cfg(test)]
mod vault_utils_tests;

// Re-export pwd-strength for backward compatibility
pub use pwd_strength::{
    evaluate_password_strength, evaluate_password_strength_tx, init_blacklist_from_path,
};

// Re-export convenience functions
pub use db_backend::get_system_username;
