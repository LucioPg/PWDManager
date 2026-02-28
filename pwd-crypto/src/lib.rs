//! Password cryptography library
//!
//! Provides password hashing (Argon2) and encryption (AES-256-GCM) utilities.
//!
//! # Features
//!
//! - `hash` (default): Argon2 password hashing
//! - `cipher`: AES-256-GCM encryption
//! - `full`: All features enabled
//! - `base64`: Base64 encoding utilities
//!
//! # Example
//!
//! ```rust,ignore
//! use pwd_crypto::{encrypt, verify_password};
//! use secrecy::SecretString;
//!
//! // Hash a password
//! let password = SecretString::new("my_password".into());
//! let hash = encrypt(password.clone())?;
//!
//! // Verify the password
//! verify_password(password, &hash)?;
//! # Ok::<(), pwd_crypto::CryptoError, ()>
//! ```

mod error;
pub use error::CryptoError;

#[cfg(feature = "hash")]
mod hash;
#[cfg(feature = "hash")]
pub use hash::{encrypt, verify_password, generate_salt};

#[cfg(feature = "cipher")]
mod nonce;
#[cfg(feature = "cipher")]
pub use nonce::{create_nonce, nonce_from_vec};

#[cfg(feature = "cipher")]
mod cipher;
#[cfg(feature = "cipher")]
pub use cipher::{
    encrypt_string,
    encrypt_optional_string,
    decrypt_to_string,
    decrypt_optional_to_string,
};

#[cfg(all(feature = "cipher", feature = "pwd-types"))]
pub use cipher::create_cipher;

#[cfg(feature = "base64")]
mod encoding;
#[cfg(feature = "base64")]
pub use encoding::{base64_encode, base64_decode};

// Re-export secrecy for convenience
#[cfg(any(feature = "hash", feature = "cipher"))]
pub use secrecy::SecretString;

#[cfg(feature = "cipher")]
pub use secrecy::SecretBox;

// Re-export tipi esterni usati nelle API pubbliche
// I consumer possono usarli senza aggiungere dipendenze esplicite
#[cfg(feature = "hash")]
pub use argon2::password_hash::SaltString;

#[cfg(feature = "cipher")]
pub use aes_gcm::Aes256Gcm;

#[cfg(feature = "cipher")]
pub use aes_gcm::aead::Nonce;
