use std::io::Cursor;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2
};
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
use image::{DynamicImage, ImageFormat};
use custom_errors::{EncryptionError, DecryptionError, GeneralError};

const CUSTOM_B64_ENGINE: engine::GeneralPurpose = engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

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


pub fn set_user_avatar(avatar_from_db: Option<Vec<u8>>) -> String {
    let avatar: Vec<u8> = match avatar_from_db {
        Some(avatar_) => {
            if !avatar_.is_empty() { avatar_}
            else { include_bytes!("../../assets/default_avatar.png").to_vec()}
        }
        _ => { include_bytes!("../../assets/default_avatar.png").to_vec() }
    };

    let b64 = CUSTOM_B64_ENGINE.encode(&avatar);
    format!("data:image/png;base64,{}", b64)

}

pub fn scale_avatar(bytes: &[u8]) -> Result<Vec<u8>, GeneralError>{
    let img = image::load_from_memory(bytes).map_err(|e| GeneralError::new_scaling_error(e.to_string()))?;
    image_to_vec(&img.resize(128, 128, image::imageops::FilterType::Triangle))
}

fn image_to_vec(img: &DynamicImage) -> Result<Vec<u8>, GeneralError> {
    let mut buffer = Cursor::new(Vec::new());

    // Specifica il formato. PNG è ideale per mantenere la qualità
    // o se hai angoli trasparenti.
    img.write_to(&mut buffer, ImageFormat::Png).map_err(|e| GeneralError::new_encode_error(e.to_string()))?;

    Ok((buffer.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_expected_default_avatar() -> String {
        let default_bytes = include_bytes!("../../assets/default_avatar.png");

        format!("data:image/png;base64,{}", CUSTOM_B64_ENGINE.encode(default_bytes))
    }
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

    #[test]
    fn test_avatar_presente() {
        // Test con dati validi
        let dati = Some(vec![1, 2, 3]);
        let risultato = set_user_avatar(dati);
        assert!(!risultato.is_empty());
        // L'encoding base64 di [1, 2, 3] URL_SAFE NO_PAD è "AQID"
        // println!("PRESENTE: {risultato}");
        assert_eq!(risultato, "data:image/png;base64,AQID");
    }

    #[test]
    fn test_avatar_vuoto() {
        // Test con Some ma vettore vuoto
        let dati = Some(vec![]);
        let risultato = set_user_avatar(dati);
        let expected = get_expected_default_avatar();
        // Verifichiamo che non sia vuoto (perché deve esserci il default)
        // println!("VUOTO: {risultato}");
        assert!(!risultato.is_empty());
        assert_eq!(risultato, expected);
    }

    #[test]
    fn test_avatar_none() {
        // Test con None
        let risultato = set_user_avatar(None);
        // println!("NULLO: {risultato}");
        // Dovrebbe restituire l'encoding del file di default
        let expected = get_expected_default_avatar();

        assert_eq!(risultato, expected);
    }

    // #[test]
    // fn test_scale_default_avatar() {
    //     assert!(scale_default_avatar());
    // }
}