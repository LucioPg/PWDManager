use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use secrecy::{ExposeSecret, SecretString};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use custom_errors::{DecryptionError, EncryptionError};

pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

pub fn encrypt(raw_password: SecretString) -> Result<String, EncryptionError> {
    if raw_password.expose_secret().trim().is_empty() {
        return Err(EncryptionError::new_encryption_error(
            "The password cannot be empty".to_string(),
        ));
    }
    let salt = generate_salt();
    let password = raw_password.expose_secret().as_bytes();
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password, &salt)
        .map_err(|e| EncryptionError::new_encryption_error(e.to_string()))?;
    let hash_string = hash.to_string();
    Ok(hash_string)
}

pub fn verify_password(raw_password: SecretString, hash: &str) -> Result<(), DecryptionError> {
    let argon2 = Argon2::default();
    let password = raw_password.expose_secret().as_bytes();
    let hash =
        PasswordHash::new(hash).map_err(|e| DecryptionError::new_rotten_password(e.to_string()))?;
    argon2
        .verify_password(password, &hash)
        .map_err(|_| DecryptionError::new_wrong_password())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt() {
        let text = SecretString::new("password123".into());

        let result = encrypt(text);

        assert!(result.is_ok(), "encrypt should return Ok(...)");
        let hash = result.unwrap();

        // Il risultato deve essere una stringa non vuota
        assert!(!hash.is_empty(), "hash should not be empty");

        // Argon2 produce hash che iniziano con "$argon2"
        assert!(
            hash.starts_with("$argon2"),
            "hash should start with $argon2, got: {hash}"
        );
    }
    #[test]
    fn test_decrypt() {
        let text = SecretString::new("password123".into());
        let text_clone = text.clone();
        let hash = encrypt(text).unwrap();
        let result = verify_password(text_clone, &hash);
        assert!(result.is_ok(), "decrypt should return Ok(...)");
    }
}
