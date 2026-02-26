//! Unified error types for password cryptography operations.

use thiserror::Error;

/// Unified error type for all crypto operations.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Error during password hashing
    #[error("Password hashing error: {0}")]
    HashingError(String),

    /// Error during password verification
    #[error("Password verification failed")]
    VerificationFailed,

    /// Password is empty or invalid
    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    /// Error during encryption
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Error during decryption
    #[error("Decryption error: {0}")]
    DecryptionError(String),

    /// Nonce is corrupted (not 12 bytes)
    #[error("Nonce corruption: expected 12 bytes, got {0}")]
    NonceCorruption(usize),

    /// Cipher creation failed
    #[error("Cipher creation failed: {0}")]
    CipherCreationError(String),

    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDerivationError(String),

    /// UTF-8 conversion error
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashing_error_display() {
        let err = CryptoError::HashingError("test error".to_string());
        assert!(err.to_string().contains("hashing"));
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_verification_failed_display() {
        let err = CryptoError::VerificationFailed;
        assert!(err.to_string().contains("verification"));
    }

    #[test]
    fn test_nonce_corruption_display() {
        let err = CryptoError::NonceCorruption(8);
        assert!(err.to_string().contains("8"));
        assert!(err.to_string().contains("12"));
    }

    #[test]
    fn test_error_is_send_sync() {
        // Verify that CryptoError is Send + Sync for async usage
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CryptoError>();
    }
}
