use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2
};


use custom_errors::{EncryptionError, DecryptionError};

fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

pub fn encrypt(text: &str) -> Result<String, EncryptionError> {
    let salt = generate_salt();
    let password = text.as_bytes();
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(
        password,
        &salt
    ).map_err(|e| EncryptionError::new_encryption_error(e.to_string()))?;
    let hash_string = hash.to_string();
    print!("password: {text}\nsalt: {salt}\nhash: {hash_string}\n");
    Ok(hash_string)

}

pub fn verify_password(text: &str, hash: &str) -> Result<(), DecryptionError> {
    let argon2 = Argon2::default();
    let password = text.as_bytes();
    let hash = PasswordHash::new(hash).map_err(|e| DecryptionError::new_rotten_password(e.to_string()))?;
    argon2.verify_password(password, &hash).map_err(|_| DecryptionError::new_wrong_password())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encrypt()
    {
        let text = "password123";

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
    fn test_decrypt(){
        let text = "password123";
        let hash = encrypt(text).unwrap();
        let result = verify_password(text, &hash);
        assert!(result.is_ok(), "decrypt should return Ok(...)");
    }
}