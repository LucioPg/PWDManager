//! Nonce utilities for AES-256-GCM encryption.

use crate::error::CryptoError;
use aes_gcm::aead::{AeadCore, Nonce, OsRng};
use aes_gcm::Aes256Gcm;

/// Creates a new random nonce for AES-256-GCM.
pub fn create_nonce() -> Nonce<Aes256Gcm> {
    Aes256Gcm::generate_nonce(&mut OsRng)
}

/// Converts a byte vector to a nonce.
///
/// # Errors
///
/// Returns `CryptoError::NonceCorruption` if the vector is not exactly 12 bytes.
pub fn nonce_from_vec(nonce_vec: &[u8]) -> Result<Nonce<Aes256Gcm>, CryptoError> {
    if nonce_vec.len() != 12 {
        return Err(CryptoError::NonceCorruption(nonce_vec.len()));
    }
    Ok(*Nonce::<Aes256Gcm>::from_slice(nonce_vec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_nonce() {
        let nonce = create_nonce();

        // Nonce should be 12 bytes
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_create_nonce_unique() {
        let nonce1 = create_nonce();
        let nonce2 = create_nonce();

        // Each nonce should be unique
        assert_ne!(nonce1.as_slice(), nonce2.as_slice());
    }

    #[test]
    fn test_nonce_from_vec_success() {
        let original_nonce = create_nonce();
        let vec = original_nonce.to_vec();

        let result = nonce_from_vec(&vec);

        assert!(result.is_ok());
        let recovered = result.unwrap();
        assert_eq!(recovered.as_slice(), original_nonce.as_slice());
    }

    #[test]
    fn test_nonce_from_vec_too_short() {
        let short_vec = vec![0u8; 8];

        let result = nonce_from_vec(&short_vec);

        assert!(result.is_err());
        match result {
            Err(CryptoError::NonceCorruption(len)) => assert_eq!(len, 8),
            _ => panic!("Expected NonceCorruption error"),
        }
    }

    #[test]
    fn test_nonce_from_vec_too_long() {
        let long_vec = vec![0u8; 16];

        let result = nonce_from_vec(&long_vec);

        assert!(result.is_err());
        match result {
            Err(CryptoError::NonceCorruption(len)) => assert_eq!(len, 16),
            _ => panic!("Expected NonceCorruption error"),
        }
    }

    #[test]
    fn test_nonce_from_vec_empty() {
        let empty_vec: Vec<u8> = vec![];

        let result = nonce_from_vec(&empty_vec);

        assert!(result.is_err());
        match result {
            Err(CryptoError::NonceCorruption(len)) => assert_eq!(len, 0),
            _ => panic!("Expected NonceCorruption error"),
        }
    }
}
