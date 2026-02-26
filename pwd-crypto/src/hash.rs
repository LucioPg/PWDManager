//! Password hashing using Argon2.
//!
//! Provides secure password hashing and verification.

use crate::error::CryptoError;
use secrecy::{ExposeSecret, SecretString};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use argon2::password_hash::rand_core::OsRng;

/// Generates a random salt for password hashing.
pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

/// Hashes a password using Argon2.
///
/// # Errors
///
/// Returns `CryptoError::InvalidPassword` if the password is empty.
/// Returns `CryptoError::HashingError` if hashing fails.
pub fn encrypt(raw_password: SecretString) -> Result<String, CryptoError> {
    let password_str = raw_password.expose_secret();

    if password_str.trim().is_empty() {
        return Err(CryptoError::InvalidPassword(
            "The password cannot be empty".to_string(),
        ));
    }

    let salt = generate_salt();
    let password_bytes = password_str.as_bytes();
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password_bytes, &salt)
        .map_err(|e| CryptoError::HashingError(e.to_string()))?;

    Ok(hash.to_string())
}

/// Verifies a password against a hash.
///
/// # Errors
///
/// Returns `CryptoError::VerificationFailed` if verification fails.
pub fn verify_password(raw_password: SecretString, hash: &str) -> Result<(), CryptoError> {
    let argon2 = Argon2::default();
    let password_bytes = raw_password.expose_secret().as_bytes();

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;

    argon2
        .verify_password(password_bytes, &parsed_hash)
        .map_err(|_| CryptoError::VerificationFailed)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_salt() {
        let salt = generate_salt();
        // Salt should be a non-empty string
        assert!(!salt.as_str().is_empty());
    }

    #[test]
    fn test_encrypt_success() {
        let password = SecretString::new("password123".into());
        let result = encrypt(password);

        assert!(result.is_ok(), "encrypt should return Ok(...)");
        let hash = result.unwrap();

        // Hash should be non-empty
        assert!(!hash.is_empty(), "hash should not be empty");

        // Argon2 hashes start with "$argon2"
        assert!(
            hash.starts_with("$argon2"),
            "hash should start with $argon2, got: {hash}"
        );
    }

    #[test]
    fn test_encrypt_empty_password() {
        let password = SecretString::new("".into());
        let result = encrypt(password);

        assert!(result.is_err());
        match result {
            Err(CryptoError::InvalidPassword(_)) => {}
            _ => panic!("Expected InvalidPassword error"),
        }
    }

    #[test]
    fn test_encrypt_whitespace_only() {
        let password = SecretString::new("   ".into());
        let result = encrypt(password);

        assert!(result.is_err());
        match result {
            Err(CryptoError::InvalidPassword(_)) => {}
            _ => panic!("Expected InvalidPassword error"),
        }
    }

    #[test]
    fn test_verify_password_success() {
        let password = SecretString::new("password123".into());
        let password_clone = password.clone();
        let hash = encrypt(password).unwrap();

        let result = verify_password(password_clone, &hash);
        assert!(result.is_ok(), "verify_password should succeed for correct password");
    }

    #[test]
    fn test_verify_password_wrong_password() {
        let password = SecretString::new("password123".into());
        let hash = encrypt(password).unwrap();

        let wrong_password = SecretString::new("wrongpassword".into());
        let result = verify_password(wrong_password, &hash);

        assert!(result.is_err());
        match result {
            Err(CryptoError::VerificationFailed) => {}
            _ => panic!("Expected VerificationFailed error"),
        }
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = SecretString::new("password123".into());
        let result = verify_password(password, "invalid-hash");

        assert!(result.is_err());
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let password1 = SecretString::new("password1".into());
        let password2 = SecretString::new("password2".into());

        let hash1 = encrypt(password1).unwrap();
        let hash2 = encrypt(password2).unwrap();

        assert_ne!(hash1, hash2, "Different passwords should have different hashes");
    }

    #[test]
    fn test_same_password_different_salts() {
        let password1 = SecretString::new("password123".into());
        let password2 = SecretString::new("password123".into());

        let hash1 = encrypt(password1).unwrap();
        let hash2 = encrypt(password2).unwrap();

        // Same password should produce different hashes due to random salts
        assert_ne!(hash1, hash2, "Same password should have different hashes (different salts)");
    }
}
