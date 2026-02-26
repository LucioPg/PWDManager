# pwd-crypto Library Extraction Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Estrarre le funzioni crittografiche (hashing Argon2 + cipher AES-256-GCM) in una libreria indipendente `pwd-crypto` con approccio TDD, separando prima le funzioni avatar che rimangono nel progetto padre.

**Architecture:** Libreria con feature flags per hash/cipher/base64, dipendenza opzionale da `pwd-types` per `create_cipher`, error types unificati con `CryptoError`.

**Tech Stack:** Rust, argon2, aes-gcm, secrecy, base64, thiserror

---

## Prerequisiti

- [x] Step 1 (pwd-types) completato
- [x] Step 2 (pwd-strength) completato
- [ ] `cargo test --workspace` passa

---

## Analisi Funzioni

### Da `utils.rs`

| Funzione | Azione | Feature |
|----------|--------|---------|
| `base64_encode` | ➡️ pwd-crypto | `base64` |
| `generate_salt` | ➡️ pwd-crypto | `hash` |
| `encrypt` | ➡️ pwd-crypto | `hash` |
| `verify_password` | ➡️ pwd-crypto | `hash` |
| `get_user_avatar_with_default` | ⬅️ avatar_utils.rs | - |
| `format_avatar_url` | ⬅️ avatar_utils.rs | - |
| `scale_avatar` | ⬅️ avatar_utils.rs | - |
| `image_to_vec` | ⬅️ avatar_utils.rs | - |

### Da `password_utils.rs`

| Funzione | Azione | Feature |
|----------|--------|---------|
| `create_nonce` | ➡️ pwd-crypto | `cipher` |
| `get_nonce_from_vec` | ➡️ pwd-crypto | `cipher` |
| `encrypt_string` | ➡️ pwd-crypto | `cipher` |
| `encrypt_optional_string` | ➡️ pwd-crypto | `cipher` |
| `decrypt_to_string` | ➡️ pwd-crypto | `cipher` |
| `decrypt_optional_to_string` | ➡️ pwd-crypto | `cipher` |
| `create_cipher` | ➡️ pwd-crypto | `cipher` + pwd-types |
| `get_salt` | ➡️ pwd-crypto | `cipher` |
| Pipeline DB functions | ❌ Rimangono | - |

---

## Struttura Crate Finale

```
pwd-crypto/
├── Cargo.toml
└── src/
    ├── lib.rs                 # Public API
    ├── error.rs               # CryptoError enum
    ├── hash.rs                # Argon2 hashing (feature: hash)
    ├── cipher.rs              # AES-256-GCM (feature: cipher)
    ├── nonce.rs               # Nonce utilities (feature: cipher)
    └── encoding.rs            # base64 utilities (feature: base64)
```

> **Nota:** I test sono inline nei moduli (`#[cfg(test)]`) come in pwd-strength.

---

## Task 1: Pre-refactoring - Separare Avatar Utils

**Obiettivo:** Spostare funzioni avatar in un file separato prima di estrarre la libreria.

**Files:**
- Create: `src/backend/avatar_utils.rs`
- Modify: `src/backend/mod.rs`
- Modify: `src/backend/utils.rs`

**Step 1: Create avatar_utils.rs**

Create `src/backend/avatar_utils.rs`:

```rust
//! Avatar utilities for user profile images.
//!
//! These functions handle avatar loading, scaling, and formatting.
//! They remain in the PWDManager project (not extracted to pwd-crypto).

use base64::{Engine, prelude::BASE64_STANDARD};
use custom_errors::GeneralError;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Encodes bytes to base64 string.
pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

/// Returns user avatar with fallback to default.
///
/// If `avatar_from_db` is `None` or empty, returns the default avatar.
pub fn get_user_avatar_with_default(avatar_from_db: Option<Vec<u8>>) -> String {
    let avatar: Vec<u8> = match avatar_from_db {
        Some(avatar_) if !avatar_.is_empty() => avatar_,
        _ => include_bytes!("../../assets/default_avatar.png").to_vec(),
    };
    format_avatar_url(base64_encode(&avatar))
}

/// Formats avatar bytes as data URL.
pub fn format_avatar_url(avatar_b64: String) -> String {
    format!("data:image/png;base64,{}", avatar_b64)
}

/// Scales avatar to 128x128 pixels.
pub fn scale_avatar(bytes: &[u8]) -> Result<Vec<u8>, GeneralError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| GeneralError::new_scaling_error(e.to_string()))?;
    image_to_vec(&img.thumbnail(128, 128))
}

fn image_to_vec(img: &DynamicImage) -> Result<Vec<u8>, GeneralError> {
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| GeneralError::new_encode_error(e.to_string()))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_expected_default_avatar() -> String {
        let default_bytes = include_bytes!("../../assets/default_avatar.png");
        format!("data:image/png;base64,{}", base64_encode(default_bytes))
    }

    #[test]
    fn test_avatar_present() {
        let data = Some(vec![1, 2, 3]);
        let result = get_user_avatar_with_default(data);
        assert_eq!(result, "data:image/png;base64,AQID");
    }

    #[test]
    fn test_avatar_empty() {
        let data = Some(vec![]);
        let result = get_user_avatar_with_default(data);
        let expected = get_expected_default_avatar();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_avatar_none() {
        let result = get_user_avatar_with_default(None);
        let expected = get_expected_default_avatar();
        assert_eq!(result, expected);
    }
}
```

**Step 2: Update mod.rs**

Add to `src/backend/mod.rs`:

```rust
pub mod avatar_utils;
```

**Step 3: Remove avatar functions from utils.rs**

Remove from `src/backend/utils.rs`:
- `get_user_avatar_with_default`
- `format_avatar_url`
- `scale_avatar`
- `image_to_vec`
- Related test `test_avatar_*`

Also remove unused imports:
```rust
// Remove:
// use std::io::Cursor;
// use image::{DynamicImage, ImageFormat};
```

**Step 4: Verify compilation**

```bash
cargo check --workspace
```

Expected: No errors

---

## Task 2: Setup pwd-crypto Directory Structure

**Files:**
- Create: `pwd-crypto/Cargo.toml`
- Create: `pwd-crypto/src/lib.rs` (placeholder)

**Step 1: Create directory**

```bash
mkdir -p pwd-crypto/src
```

**Step 2: Create Cargo.toml**

Create `pwd-crypto/Cargo.toml`:

```toml
[package]
name = "pwd-crypto"
version = "0.1.0"
edition = "2024"

[features]
default = ["hash"]

# Argon2 password hashing
hash = ["dep:argon2", "dep:secrecy"]

# AES-256-GCM encryption (richiede pwd-types per create_cipher)
cipher = ["dep:aes-gcm", "dep:secrecy", "dep:pwd-types"]

# Full crypto suite
full = ["hash", "cipher"]

# Base64 utilities
base64 = ["dep:base64"]

[dependencies]
# Core
thiserror = "2.0"
secrecy = { version = "0.10", optional = true }

# Password hashing (optional)
argon2 = { version = "0.5", features = ["std", "zeroize"], optional = true }

# Encryption (optional)
aes-gcm = { version = "0.10", features = ["zeroize"], optional = true }

# Encoding (optional)
base64 = { version = "0.22", optional = true }

# For create_cipher (optional, requires pwd-types with sqlx)
pwd-types = { path = "../pwd-types", features = ["sqlx"], optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["test-util", "macros"] }
# Nota: serial_test non serve qui (nessun test modifica env vars)
```

**Step 3: Create placeholder lib.rs**

Create `pwd-crypto/src/lib.rs`:

```rust
//! Password cryptography library
//!
//! Provides password hashing (Argon2) and encryption (AES-256-GCM) utilities.

// Placeholder - will be filled in later tasks
```

**Step 4: Verify structure**

```bash
ls -la pwd-crypto/
```

---

## Task 3: Add to Workspace

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Add pwd-crypto to workspace members**

Edit `Cargo.toml` (root), update workspace members:

```toml
[workspace]
members = ["gui_launcher", ".", "custom_errors", "pwd-types", "pwd-strength", "pwd-crypto"]
```

**Step 2: Verify cargo sees the new crate**

```bash
cargo check -p pwd-crypto
```

Expected: Compiles without errors

---

## Task 4: TDD - Write CryptoError Tests First

**Files:**
- Create: `pwd-crypto/src/error.rs`

**Step 1: Write error types with tests**

Create `pwd-crypto/src/error.rs`:

```rust
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
```

**Step 2: Run tests to verify they pass**

```bash
cargo test -p pwd-crypto --lib error
```

Expected: All 4 tests pass

---

## Task 5: TDD - Write Hash Tests First

**Files:**
- Create: `pwd-crypto/src/hash.rs`

**Step 1: Write failing tests for hash functions**

Create `pwd-crypto/src/hash.rs`:

```rust
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
    todo!("Implement after writing tests")
}

/// Hashes a password using Argon2.
///
/// # Errors
///
/// Returns `CryptoError::InvalidPassword` if the password is empty.
/// Returns `CryptoError::HashingError` if hashing fails.
pub fn encrypt(raw_password: SecretString) -> Result<String, CryptoError> {
    todo!("Implement after writing tests")
}

/// Verifies a password against a hash.
///
/// # Errors
///
/// Returns `CryptoError::VerificationFailed` if verification fails.
pub fn verify_password(raw_password: SecretString, hash: &str) -> Result<(), CryptoError> {
    todo!("Implement after writing tests")
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
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p pwd-crypto --lib hash
```

Expected: Tests fail with "not yet implemented" panic

---

## Task 6: TDD - Implement Hash Functions

**Files:**
- Modify: `pwd-crypto/src/hash.rs`

**Step 1: Implement generate_salt**

```rust
/// Generates a random salt for password hashing.
pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}
```

**Step 2: Implement encrypt**

```rust
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
```

**Step 3: Implement verify_password**

```rust
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
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p pwd-crypto --lib hash
```

Expected: All 10 tests pass

---

## Task 7: TDD - Write Nonce Tests First

**Files:**
- Create: `pwd-crypto/src/nonce.rs`

**Step 1: Write failing tests for nonce functions**

Create `pwd-crypto/src/nonce.rs`:

```rust
//! Nonce utilities for AES-256-GCM encryption.

use crate::error::CryptoError;
use aes_gcm::aead::{AeadCore, Nonce};
use aes_gcm::Aes256Gcm;
use aes_gcm::aead::OsRng;

/// Creates a new random nonce for AES-256-GCM.
pub fn create_nonce() -> Nonce<Aes256Gcm> {
    todo!("Implement after writing tests")
}

/// Converts a byte vector to a nonce.
///
/// # Errors
///
/// Returns `CryptoError::NonceCorruption` if the vector is not exactly 12 bytes.
pub fn nonce_from_vec(nonce_vec: &[u8]) -> Result<Nonce<Aes256Gcm>, CryptoError> {
    todo!("Implement after writing tests")
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
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p pwd-crypto --lib nonce
```

Expected: Tests fail with "not yet implemented" panic

---

## Task 8: TDD - Implement Nonce Functions

**Files:**
- Modify: `pwd-crypto/src/nonce.rs`

**Step 1: Implement create_nonce**

```rust
/// Creates a new random nonce for AES-256-GCM.
pub fn create_nonce() -> Nonce<Aes256Gcm> {
    Aes256Gcm::generate_nonce(&mut OsRng)
}
```

**Step 2: Implement nonce_from_vec**

```rust
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
```

**Step 3: Run tests to verify they pass**

```bash
cargo test -p pwd-crypto --lib nonce
```

Expected: All 6 tests pass

---

## Task 9: TDD - Write Cipher Tests First

**Files:**
- Create: `pwd-crypto/src/cipher.rs`

**Step 1: Write failing tests for cipher functions**

Create `pwd-crypto/src/cipher.rs`:

```rust
//! AES-256-GCM encryption for password storage.

use crate::error::CryptoError;
use crate::nonce::{create_nonce, nonce_from_vec};
use aes_gcm::aead::Nonce;
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use argon2::password_hash::Salt;
use argon2::Argon2;

#[cfg(feature = "pwd-types")]
use pwd_types::UserAuth;

/// Creates an AES-256-GCM cipher from a salt and user credentials.
#[cfg(feature = "pwd-types")]
pub fn create_cipher(salt: &Salt<'_>, user_auth: &UserAuth) -> Result<Aes256Gcm, CryptoError> {
    todo!("Implement after writing tests")
}

/// Encrypts a string using AES-256-GCM.
///
/// Returns a tuple of (encrypted_bytes, nonce).
pub fn encrypt_string(
    plaintext: &str,
    cipher: &Aes256Gcm,
) -> Result<(SecretBox<[u8]>, Nonce<Aes256Gcm>), CryptoError> {
    todo!("Implement after writing tests")
}

/// Encrypts an optional string.
///
/// Returns `None` if the input is `None`.
pub fn encrypt_optional_string(
    plaintext: Option<&str>,
    cipher: &Aes256Gcm,
) -> Result<(Option<SecretBox<[u8]>>, Option<Nonce<Aes256Gcm>>), CryptoError> {
    todo!("Implement after writing tests")
}

/// Decrypts bytes to a UTF-8 string.
pub fn decrypt_to_string(
    encrypted: &[u8],
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<String, CryptoError> {
    todo!("Implement after writing tests")
}

/// Decrypts optional bytes to an optional string.
pub fn decrypt_optional_to_string(
    encrypted: Option<&[u8]>,
    nonce: Option<&Nonce<Aes256Gcm>>,
    cipher: &Aes256Gcm,
) -> Result<Option<String>, CryptoError> {
    todo!("Implement after writing tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::Aead;

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

        let (enc1, nonce1) = encrypt_string("password", &cipher).unwrap();
        let (enc2, nonce2) = encrypt_string("password", &cipher).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(enc1.expose_secret(), enc2.expose_secret());
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p pwd-crypto --lib cipher
```

Expected: Tests fail with "not yet implemented" panic

---

## Task 10: TDD - Implement Cipher Functions

**Files:**
- Modify: `pwd-crypto/src/cipher.rs`

**Step 1: Implement encrypt_string**

```rust
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
```

**Step 2: Implement encrypt_optional_string**

```rust
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
```

**Step 3: Implement decrypt_to_string**

```rust
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
```

**Step 4: Implement decrypt_optional_to_string**

```rust
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
```

**Step 5: Implement create_cipher (with pwd-types feature)**

```rust
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
```

**Step 6: Run tests to verify they pass**

```bash
cargo test -p pwd-crypto --lib cipher
```

Expected: All 12 tests pass

---

## Task 11: TDD - Write Encoding Tests First

**Files:**
- Create: `pwd-crypto/src/encoding.rs`

**Step 1: Write failing tests for encoding functions**

Create `pwd-crypto/src/encoding.rs`:

```rust
//! Base64 encoding utilities.

use base64::{Engine, prelude::BASE64_STANDARD};
use crate::error::CryptoError;

/// Encodes bytes to a base64 string.
pub fn base64_encode(bytes: &[u8]) -> String {
    todo!("Implement after writing tests")
}

/// Decodes a base64 string to bytes.
///
/// # Errors
///
/// Returns `CryptoError::DecryptionError` if the input is not valid base64.
pub fn base64_decode(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    todo!("Implement after writing tests")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_empty() {
        let result = base64_encode(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_base64_encode_simple() {
        let result = base64_encode(&[1, 2, 3]);
        assert_eq!(result, "AQID");
    }

    #[test]
    fn test_base64_encode_hello() {
        let result = base64_encode(b"Hello");
        assert_eq!(result, "SGVsbG8=");
    }

    #[test]
    fn test_base64_decode_empty() {
        let result = base64_decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_base64_decode_simple() {
        let result = base64_decode("AQID").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_base64_decode_hello() {
        let result = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello".to_vec());
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = base64_decode("!!invalid!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip() {
        let original = b"This is a test string with various bytes: \x00\x01\x02\xff";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded.as_slice(), original.as_slice());
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p pwd-crypto --lib encoding
```

Expected: Tests fail with "not yet implemented" panic

---

## Task 12: TDD - Implement Encoding Functions

**Files:**
- Modify: `pwd-crypto/src/encoding.rs`

**Step 1: Implement base64_encode**

```rust
/// Encodes bytes to a base64 string.
pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}
```

**Step 2: Implement base64_decode**

```rust
/// Decodes a base64 string to bytes.
///
/// # Errors
///
/// Returns `CryptoError::DecryptionError` if the input is not valid base64.
pub fn base64_decode(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))
}
```

**Step 3: Run tests to verify they pass**

```bash
cargo test -p pwd-crypto --lib encoding
```

Expected: All 8 tests pass

---

## Task 13: Wire Up lib.rs Public API

**Files:**
- Modify: `pwd-crypto/src/lib.rs`

**Step 1: Create complete lib.rs**

```rust
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

#[cfg(feature = "cipher")]
mod nonce;
#[cfg(feature = "cipher")]
pub use nonce::{create_nonce, nonce_from_vec};

#[cfg(feature = "base64")]
mod encoding;
#[cfg(feature = "base64")]
pub use encoding::{base64_encode, base64_decode};

// Re-export secrecy for convenience
#[cfg(any(feature = "hash", feature = "cipher"))]
pub use secrecy::SecretString;

#[cfg(feature = "cipher")]
pub use secrecy::SecretBox;
```

**Step 2: Verify crate compiles with all features**

```bash
cargo check -p pwd-crypto --all-features
```

Expected: No errors

---

## Task 14: Run All Tests

**Files:**
- Test: `pwd-crypto/` (all tests)

**Step 1: Run full test suite with all features**

```bash
cargo test -p pwd-crypto --all-features
```

Expected: All tests pass

**Step 2: Run with default features only**

```bash
cargo test -p pwd-crypto
```

Expected: All hash tests pass

**Step 3: Run with specific features**

```bash
cargo test -p pwd-crypto --features cipher
cargo test -p pwd-crypto --features base64
```

Expected: All respective tests pass

---

## Task 15: Update PWDManager Dependencies

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Add pwd-crypto dependency to PWDManager**

Add to `[dependencies]` section in root `Cargo.toml`:

```toml
pwd-crypto = { path = "pwd-crypto", features = ["full", "base64", "pwd-types"] }
```

**Step 2: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: No errors (may have warnings about unused imports)

---

## Task 16: Update PWDManager Code to Use Library

**Files:**
- Modify: `src/backend/mod.rs`
- Modify: `src/backend/utils.rs`
- Modify: `src/backend/password_utils.rs`

**Step 1: Update mod.rs to re-export from library**

Add to `src/backend/mod.rs`:

```rust
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
```

**Step 2: Remove extracted functions from utils.rs**

Remove from `src/backend/utils.rs`:
- `base64_encode`
- `generate_salt`
- `encrypt`
- `verify_password`
- Related imports (`argon2::*`, `base64::*`, `secrecy::*`)
- Related tests (`test_encrypt`, `test_decrypt`)

**Step 3: Remove extracted functions from password_utils.rs**

Remove from `src/backend/password_utils.rs`:
- `get_salt`
- `create_nonce`
- `get_nonce_from_vec` (renamed to `nonce_from_vec`)
- `encrypt_string`
- `encrypt_optional_string`
- `decrypt_to_string`
- `decrypt_optional_to_string`
- `create_cipher`
- `create_password_with_cipher` (internal helper)
- `create_password_with_cipher_sync` (internal helper)
- Related imports

**Step 4: Update password_utils.rs imports**

Update imports in `src/backend/password_utils.rs`:

```rust
use pwd_crypto::{
    create_cipher, create_nonce, nonce_from_vec,
    encrypt_string, encrypt_optional_string,
    decrypt_to_string, decrypt_optional_to_string,
};
use argon2::password_hash::Salt;
// Keep other imports for pipeline functions
```

**Step 5: Update get_salt usage**

The `get_salt` function is now internal. Update password_utils.rs to extract salt inline:

```rust
fn get_salt_from_hash(hash_password: &DbSecretString) -> Salt<'_> {
    let hash_password = hash_password.0.expose_secret();
    let parsed_hash = PasswordHash::new(hash_password).unwrap();
    parsed_hash.salt.unwrap()
}
```

**Step 6: Verify compilation**

```bash
cargo check --workspace
```

Expected: No errors

---

## Task 16.5: Remove Duplicate base64_encode from avatar_utils

**Obiettivo:** Eliminare la duplicazione di `base64_encode` usando la versione estratta in `pwd-crypto`.

**Files:**
- Modify: `src/backend/avatar_utils.rs`

**Problema:** In Task 1 abbiamo creato `avatar_utils.rs` con una funzione `base64_encode` locale perché `pwd-crypto` non esisteva ancora. Ora che la libreria è estratta, dobbiamo rimuovere la duplicazione.

**Step 1: Update imports in avatar_utils.rs**

Change from:
```rust
use base64::{Engine, prelude::BASE64_STANDARD};
```

To:
```rust
use pwd_crypto::base64_encode;
```

**Step 2: Remove local base64_encode function**

Remove the function definition:
```rust
// RIMUOVERE questa funzione:
pub fn base64_encode(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}
```

**Step 3: Update function that uses it**

The function `get_user_avatar_with_default` already calls `base64_encode`, so no changes needed there - it will now use the imported version.

**Step 4: Update test helper**

In the test module, update the helper function:
```rust
fn get_expected_default_avatar() -> String {
    let default_bytes = include_bytes!("../../assets/default_avatar.png");
    format!("data:image/png;base64,{}", base64_encode(default_bytes))
}
```

**Step 5: Verify compilation**

```bash
cargo check --workspace
```

Expected: No errors

---

## Task 17: Run Full Test Suite

**Files:**
- Test: All workspace tests

**Step 1: Run workspace tests**

```bash
cargo test --workspace
```

Expected: All tests pass

**Step 2: Run with all features**

```bash
cargo test --workspace --all-features
```

Expected: All tests pass

---

## Task 18: Commit Changes

**Files:**
- All modified files

**Step 1: Stage all changes**

```bash
git add pwd-crypto/
git add Cargo.toml
git add src/backend/mod.rs
git add src/backend/utils.rs
git add src/backend/password_utils.rs
git add src/backend/avatar_utils.rs
```

**Step 2: Create commit**

```bash
git commit -m "$(cat <<'EOF'
feat: extract pwd-crypto library

- Create pwd-crypto crate with TDD approach
- Extract Argon2 hashing (encrypt, verify_password, generate_salt)
- Extract AES-256-GCM cipher functions (encrypt_string, decrypt_to_string)
- Extract nonce utilities (create_nonce, nonce_from_vec)
- Extract base64 utilities (base64_encode, base64_decode)
- Add CryptoError unified error type
- Separate avatar functions into avatar_utils.rs (remain in PWDManager)
- Add feature flags: hash (default), cipher, full, base64
- Update PWDManager to use new library via re-exports

Breaking changes: None (backward compatible via re-exports)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

**Step 3: Verify commit**

```bash
git log -1 --oneline
```

Expected: Commit created successfully

---

## Task 19: Update Orchestrator Document

**Files:**
- Modify: `docs/plans/2026-02-26-library-extraction-orchestrator.md`

**Step 1: Update Step 3 status**

Change line 34 from:
```markdown
| 3    | `pwd-crypto`   | ⏳ NON INIZIATO | `docs/plans/2026-02-26-extract-pwd-crypto.md`   | -          |
```

To:
```markdown
| 3    | `pwd-crypto`   | ✅ COMPLETATO  | `docs/plans/2026-02-26-extract-pwd-crypto.md`   | 2026-02-26 |
```

**Step 2: Update Lezioni Apprese section**

Add new entry in "Dopo Step 3 (pwd-crypto)" section.

**Step 3: Update Changelog**

Add entry for Step 3 completion.

---

## Task 20: Update Reference Document

**Files:**
- Modify: `docs/library-extraction-analysis.md`

**Step 1: Update Step 3 checklist**

Mark all items as completed.

**Step 2: Add problems encountered section**

Document any issues found during implementation.

---

## Verification Checklist

Before marking complete:

- [ ] `cargo test -p pwd-crypto --all-features` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo check --workspace` completes without errors
- [ ] All imports updated in PWDManager
- [ ] Avatar functions moved to avatar_utils.rs
- [ ] Old functions removed from utils.rs and password_utils.rs
- [ ] Commit created with descriptive message
- [ ] Orchestrator document updated
- [ ] Reference document updated

---

## Next Steps

After completing this plan:
1. Verify Step 3 checkpoint with human review
2. Proceed to Step F (Finalization) to verify workspace integrity
