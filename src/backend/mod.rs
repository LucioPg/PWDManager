pub mod settings_types;
pub mod db_backend;
pub mod migration_types;
pub mod export_types;
pub mod export;
pub mod import;
pub mod init_queries;
pub(crate) mod password_types_helper;
pub mod password_utils;
pub mod ui_utils;
pub mod utils;
pub mod avatar_utils;
pub mod updater_types;
pub mod updater;

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
    init_blacklist, init_blacklist_from_path, is_blacklisted, evaluate_password_strength,
    evaluate_password_strength_tx,
};

// Re-export pwd-crypto for backward compatibility
pub use pwd_crypto::{
    CryptoError,
    encrypt, verify_password, generate_salt,
    create_nonce, nonce_from_vec,
    encrypt_string, encrypt_optional_string,
    decrypt_to_string, decrypt_optional_to_string,
    create_cipher,
    base64_encode,
};
