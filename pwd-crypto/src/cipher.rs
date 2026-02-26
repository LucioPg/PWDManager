//! AES-256-GCM encryption for password storage.

use crate::error::CryptoError;
use crate::nonce::create_nonce;
use aes_gcm::aead::{Aead, Nonce};
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use secrecy::{ExposeSecret, SecretBox};
use argon2::password_hash::Salt;
use argon2::Argon2;

#[cfg(feature = "pwd-types")]
use pwd_types::UserAuth;

/// Encrypts a string using AES-256-GCM.
///
/// Returns a tuple of (encrypted_bytes, nonce).
pub fn encrypt_string(
    plaintext: &str,
    cipher: &Aes256Gcm,
) -> Result<(SecretBox<[u8]>, Nonce<Aes256Gcm>), CryptoError> {
    let nonce = create_nonce();
    let encrypted = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

    Ok((SecretBox::new(encrypted.into()), nonce))
}

/// Encrypts an optional string.
///
/// Returns `None` if the input is `None`.
pub fn encrypt_optional_string(
    plaintext: Option<&str>,
    cipher: &Aes256Gcm,
) -> Result<(Option<SecretBox<[u8]>>, Option<Nonce<Aes256Gcm>>), CryptoError> {
    match plaintext {
        Some(text) => {
            let (encrypted, nonce) = encrypt_string(text, cipher)?;
            Ok((Some(encrypted), Some(nonce)))
        }
        None => Ok((None, None)),
    }
}

/// Decrypts bytes to a UTF-8 string.
pub fn decrypt_to_string(
    encrypted: &[u8],
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<String, CryptoError> {
    let plaintext_bytes = cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;

    String::from_utf8(plaintext_bytes)
        .map_err(|e| CryptoError::Utf8Error(e.to_string()))
}

/// Decrypts optional bytes to an optional string.
pub fn decrypt_optional_to_string(
    encrypted: Option<&[u8]>,
    nonce: Option<&Nonce<Aes256Gcm>>,
    cipher: &Aes256Gcm,
) -> Result<Option<String>, CryptoError> {
    match (encrypted, nonce) {
        (Some(enc), Some(n)) => {
            let decrypted = decrypt_to_string(enc, n, cipher)?;
            Ok(Some(decrypted))
        }
        _ => Ok(None),
    }
}

/// Creates an AES-256-GCM cipher from a salt and user credentials.
///
/// The AES key is derived using Argon2 with:
/// - Salt: extracted from user's password hash
/// - Password: the user's hashed password
///
/// # Errors
///
/// Returns `CryptoError::CipherCreationError` if key derivation fails.
#[cfg(feature = "pwd-types")]
pub fn create_cipher(salt: &Salt<'_>, user_auth: &UserAuth) -> Result<Aes256Gcm, CryptoError> {
    let mut derived_key = [0u8; 32];

    Argon2::default()
        .hash_password_into(
            user_auth.password.expose_secret().as_bytes(),
            salt.as_str().as_bytes(),
            &mut derived_key,
        )
        .map_err(|e| CryptoError::CipherCreationError(e.to_string()))?;

    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_cipher() -> Aes256Gcm {
        // Create a test cipher with a known key
        let key = [0u8; 32];
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key))
    }

    #[test]
    fn test_encrypt_string_success() {
        let cipher = get_test_cipher();
        let plaintext = "test password";

        let result = encrypt_string(plaintext, &cipher);

        assert!(result.is_ok());
        let (encrypted, nonce) = result.unwrap();

        // Encrypted data should be different from plaintext
        assert!(!encrypted.expose_secret().is_empty());

        // Nonce should be 12 bytes
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_encrypt_string_empty() {
        let cipher = get_test_cipher();
        let plaintext = "";

        let result = encrypt_string(plaintext, &cipher);

        // Empty string should still encrypt (AES-GCM handles this)
        assert!(result.is_ok());
    }

    #[test]
    fn test_encrypt_optional_string_some() {
        let cipher = get_test_cipher();
        let plaintext = Some("test notes");

        let result = encrypt_optional_string(plaintext, &cipher);

        assert!(result.is_ok());
        let (encrypted, nonce) = result.unwrap();

        assert!(encrypted.is_some());
        assert!(nonce.is_some());
    }

    #[test]
    fn test_encrypt_optional_string_none() {
        let cipher = get_test_cipher();
        let plaintext: Option<&str> = None;

        let result = encrypt_optional_string(plaintext, &cipher);

        assert!(result.is_ok());
        let (encrypted, nonce) = result.unwrap();

        assert!(encrypted.is_none());
        assert!(nonce.is_none());
    }

    #[test]
    fn test_decrypt_to_string_success() {
        let cipher = get_test_cipher();
        let original = "my secret password";

        let (encrypted, nonce) = encrypt_string(original, &cipher).unwrap();
        let result = decrypt_to_string(encrypted.expose_secret(), &nonce, &cipher);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), original);
    }

    #[test]
    fn test_decrypt_to_string_wrong_nonce() {
        let cipher = get_test_cipher();
        let original = "my secret password";

        let (encrypted, _nonce) = encrypt_string(original, &cipher).unwrap();
        let wrong_nonce = create_nonce();

        let result = decrypt_to_string(encrypted.expose_secret(), &wrong_nonce, &cipher);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_optional_to_string_some() {
        let cipher = get_test_cipher();
        let original = "my notes";

        let (encrypted, nonce) = encrypt_string(original, &cipher).unwrap();
        let result = decrypt_optional_to_string(
            Some(encrypted.expose_secret()),
            Some(&nonce),
            &cipher,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(original.to_string()));
    }

    #[test]
    fn test_decrypt_optional_to_string_none() {
        let cipher = get_test_cipher();

        let result = decrypt_optional_to_string(None, None, &cipher);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_roundtrip_unicode() {
        let cipher = get_test_cipher();
        let original = "密码密码 🔐";  // Chinese + emoji

        let (encrypted, nonce) = encrypt_string(original, &cipher).unwrap();
        let decrypted = decrypt_to_string(encrypted.expose_secret(), &nonce, &cipher).unwrap();

        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_different_plaintexts_different_ciphertexts() {
        let cipher = get_test_cipher();

        let (enc1, nonce1) = encrypt_string("password1", &cipher).unwrap();
        let (enc2, nonce2) = encrypt_string("password2", &cipher).unwrap();

        // Different plaintexts should produce different ciphertexts
        assert_ne!(enc1.expose_secret(), enc2.expose_secret());

        // Nonces should also be different
        assert_ne!(nonce1.as_slice(), nonce2.as_slice());
    }

    #[test]
    fn test_same_plaintext_different_ciphertexts() {
        let cipher = get_test_cipher();

        let (enc1, _nonce1) = encrypt_string("password", &cipher).unwrap();
        let (enc2, _nonce2) = encrypt_string("password", &cipher).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(enc1.expose_secret(), enc2.expose_secret());
    }
}
