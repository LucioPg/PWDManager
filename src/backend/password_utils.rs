//! Modulo per la criptazione, decriptazione e salvataggio delle password.
//!
//! Fornisce funzioni per:
//! - Calcolare la forze di una password
//! - Criptare le password con AES-256-GCM usando Argon2 come KDF
//! - Decriptare le password salvate
//! - Salvare le password nel database
#![allow(dead_code)]

use crate::backend::db_backend::{
    fetch_all_stored_passwords_for_user, fetch_user_auth_from_id, save_or_update_stored_password,
};
use pwd_types::{
    AegisPasswordConfig, DbSecretString, PasswordGeneratorConfig, PasswordPreset, PasswordScore,
    StoredPassword, StoredRawPassword, UserAuth,
};
use crate::backend::evaluate_password_strength;
use pwd_crypto::{
    create_cipher as crypto_create_cipher,
    create_nonce, nonce_from_vec,
    encrypt_string as crypto_encrypt_string,
    encrypt_optional_string as crypto_encrypt_optional_string,
    decrypt_to_string as crypto_decrypt_to_string,
    decrypt_optional_to_string as crypto_decrypt_optional_to_string,
};
use aes_gcm::aead::{Aead, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit};
use argon2::password_hash::{Salt, PasswordHash};
use custom_errors::DBError;
use rayon::prelude::*;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::task;

pub fn generate_suggested_password(custom_config: Option<PasswordGeneratorConfig>) -> SecretString {
    // Se non c'è config, usa il preset God come default
    let config = custom_config.unwrap_or_else(|| PasswordPreset::God.to_config(0));

    let config_adapter: AegisPasswordConfig = config.clone().into();
    let password: String = loop {
        if let Ok(pwd) = config_adapter.generate() {
            // 1. Conta i simboli
            let sym_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();

            // 2. Controlla se contiene simboli vietati
            let has_excluded = pwd
                .chars()
                .any(|c| config.excluded_symbols.contains(&c));

            if sym_count == config.symbols as usize && !has_excluded {
                break pwd;
            }
        }
    };
    SecretString::new(password.into())
}

/// Estrae il sale da una password hash Argon2.
///
/// Il sale è necessario per derivare la chiave AES della password utente.
fn get_salt(hash_password: &DbSecretString) -> Salt<'_> {
    let hash_password = hash_password.0.expose_secret();
    let parsed_hash = PasswordHash::new(hash_password).unwrap();
    parsed_hash.salt.unwrap()
}

/// Converte CryptoError in DBError per compatibilità con il codice esistente.
fn crypto_error_to_db_error(e: pwd_crypto::CryptoError) -> DBError {
    use pwd_crypto::CryptoError;
    match e {
        CryptoError::EncryptionError(msg) => DBError::new_cipher_encryption_error(msg),
        CryptoError::DecryptionError(msg) => DBError::new_password_conversion_error(msg),
        CryptoError::NonceCorruption(_) => DBError::new_nonce_corruption_error(),
        CryptoError::CipherCreationError(msg) => DBError::new_cipher_create_error(msg),
        CryptoError::Utf8Error(msg) => DBError::new_password_conversion_error(msg),
        _ => DBError::new_password_conversion_error(e.to_string()),
    }
}

/// Cripta una stringa con AES-256-GCM.
fn encrypt_string(
    plaintext: &str,
    cipher: &Aes256Gcm,
) -> Result<(SecretBox<[u8]>, Nonce<Aes256Gcm>), DBError> {
    crypto_encrypt_string(plaintext, cipher).map_err(crypto_error_to_db_error)
}

/// Cripta una stringa opzionale con AES-256-GCM.
fn encrypt_optional_string(
    plaintext: Option<&str>,
    cipher: &Aes256Gcm,
) -> Result<(Option<SecretBox<[u8]>>, Option<Nonce<Aes256Gcm>>), DBError> {
    crypto_encrypt_optional_string(plaintext, cipher).map_err(crypto_error_to_db_error)
}

/// Decripta bytes in una stringa UTF-8.
fn decrypt_to_string(
    encrypted: &[u8],
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<String, DBError> {
    crypto_decrypt_to_string(encrypted, nonce, cipher).map_err(crypto_error_to_db_error)
}

/// Decripta bytes opzionali in una stringa opzionale.
fn decrypt_optional_to_string(
    encrypted: Option<&[u8]>,
    nonce: Option<&Nonce<Aes256Gcm>>,
    cipher: &Aes256Gcm,
) -> Result<Option<String>, DBError> {
    crypto_decrypt_optional_to_string(encrypted, nonce, cipher).map_err(crypto_error_to_db_error)
}

/// Converte un vettore di 12 byte in un [`Nonce<Aes256Gcm>`].
fn get_nonce_from_vec(nonce_vec: &Vec<u8>) -> Result<Nonce<Aes256Gcm>, DBError> {
    nonce_from_vec(nonce_vec).map_err(crypto_error_to_db_error)
}

/// Crea un cipher AES-256-GCM usando la password utente come KDF.
pub fn create_cipher(salt: &Salt<'_>, user_auth: &UserAuth) -> Result<Aes256Gcm, DBError> {
    crypto_create_cipher(salt, user_auth).map_err(crypto_error_to_db_error)
}

async fn create_password_with_cipher(
    new_password: &SecretString,
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<SecretBox<[u8]>, DBError> {
    let cipher_vec = cipher
        .encrypt(nonce, new_password.expose_secret().as_bytes())
        .map_err(|e| DBError::new_cipher_encryption_error(e.to_string()))?;
    Ok(SecretBox::new(cipher_vec.into()))
}

fn create_password_with_cipher_sync(
    new_password: &SecretString,
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<SecretBox<[u8]>, DBError> {
    let cipher_vec = cipher
        .encrypt(nonce, new_password.expose_secret().as_bytes())
        .map_err(|e| DBError::new_cipher_encryption_error(e.to_string()))?;
    Ok(SecretBox::new(cipher_vec.into()))
}

/// Pipeline completa per salvare una nuova password nel database.
pub async fn create_stored_data_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    score: Option<PasswordScore>,
) -> Result<(), DBError> {
    // 1. Recupero credenziali e setup crittografico
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;

    // 2. Cripta location
    let (encrypted_location, location_nonce) = encrypt_string(&location, &cipher)?;

    // 3. Cripta password
    let password_nonce = create_nonce();
    let encrypted_password = create_password_with_cipher(&raw_password, &password_nonce, &cipher)
        .await
        .map_err(|_| DBError::new_password_save_error("Errore durante la criptazione".into()))?;

    // 4. Cripta notes
    let (encrypted_notes, notes_nonce) = encrypt_optional_string(notes.as_deref(), &cipher)?;

    // 5. Determinazione del punteggio
    let password_score = score.unwrap_or_else(|| {
        evaluate_password_strength(&raw_password, None)
            .score
            .unwrap_or(PasswordScore::new(0))
    });

    // 6. Creazione della struct
    let stored_password = StoredPassword::new(
        None,
        user_id,
        encrypted_location,
        location_nonce.to_vec(),
        encrypted_password,
        encrypted_notes,
        notes_nonce.map(|n| n.to_vec()),
        password_score,
        None,
        password_nonce.to_vec(),
    );

    // 7. Persistenza
    save_or_update_stored_password(pool, stored_password).await?;

    Ok(())
}

/// Deprecated: Use `create_stored_data_pipeline` instead
pub async fn create_stored_password_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    score: Option<PasswordScore>,
) -> Result<(), DBError> {
    create_stored_data_pipeline(pool, user_id, location, raw_password, notes, score).await
}

/// Crea record StoredPassword criptando location, password e notes in parallelo.
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
    if stored_raw_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let cipher = Arc::new(cipher);
    let user_auth = Arc::new(user_auth);

    task::spawn_blocking(move || {
        stored_raw_passwords
            .into_par_iter()
            .map(|srp| {
                // Cripta location
                let (encrypted_location, location_nonce) =
                    encrypt_string(srp.location.expose_secret(), &cipher)?;

                // Cripta password
                let password_nonce = create_nonce();
                let encrypted_password = create_password_with_cipher_sync(
                    &srp.password, &password_nonce, &cipher
                ).map_err(|_| {
                    DBError::new_cipher_encryption_error("Cipher error".to_string())
                })?;

                // Cripta notes
                let notes_str = srp.notes.as_ref().map(|n| n.expose_secret().to_string());
                let (encrypted_notes, notes_nonce) = encrypt_optional_string(
                    notes_str.as_deref(), &cipher
                )?;

                // Calcola score
                let score_evaluation: PasswordScore = srp.score.unwrap_or_else(|| {
                    evaluate_password_strength(&srp.password, None)
                        .score
                        .unwrap_or(PasswordScore::new(0))
                });

                Ok(StoredPassword::new(
                    srp.id,
                    user_auth.id,
                    encrypted_location,
                    location_nonce.to_vec(),
                    encrypted_password,
                    encrypted_notes,
                    notes_nonce.map(|n| n.to_vec()),
                    score_evaluation,
                    None,
                    password_nonce.to_vec(),
                ))
            })
            .collect::<Result<Vec<StoredPassword>, DBError>>()
    })
    .await
    .map_err(|e| DBError::new_password_save_error(format!("Join error: {}", e)))?
}

/// Deprecated: Use `create_stored_data_records` instead
pub async fn create_stored_passwords(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
    create_stored_data_records(cipher, user_auth, stored_raw_passwords).await
}

pub async fn get_stored_raw_passwords(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords: Vec<StoredPassword> =
        fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
    )
    .await?;
    Ok(stored_raw_passwords)
}

/// Decripta in parallelo un batch di StoredPassword.
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    let cipher = Arc::new(cipher);

    task::spawn_blocking(move || {
        stored_passwords
            .into_par_iter()
            .map(|sp| {
                // Decripta location
                let location_nonce = get_nonce_from_vec(&sp.location_nonce)?;
                let location = decrypt_to_string(
                    sp.location.expose_secret().as_ref(),
                    &location_nonce,
                    &cipher,
                )?;

                // Decripta password
                let password_nonce = get_nonce_from_vec(&sp.password_nonce)?;
                let password_bytes = cipher
                    .decrypt(&password_nonce, sp.password.expose_secret().as_ref())
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
                let password = String::from_utf8(password_bytes)
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;

                // Decripta notes
                let notes = match (&sp.notes, &sp.notes_nonce) {
                    (Some(enc_notes), Some(nn)) => {
                        let notes_nonce = get_nonce_from_vec(nn)?;
                        decrypt_optional_to_string(
                            Some(enc_notes.expose_secret().as_ref()),
                            Some(&notes_nonce),
                            &cipher,
                        )?
                    }
                    _ => None,
                };

                Ok(StoredRawPassword {
                    id: sp.id,
                    user_id: user_auth.id,
                    location: SecretString::new(location.into()),
                    password: SecretString::new(password.into()),
                    notes: notes.map(|n| SecretString::new(n.into())),
                    score: Some(sp.score),
                    created_at: sp.created_at,
                })
            })
            .collect::<Result<Vec<StoredRawPassword>, DBError>>()
    })
    .await
    .map_err(|e| DBError::new_password_conversion_error(format!("Join error: {}", e)))?
}

/// Deprecated: Use `decrypt_bulk_stored_data` instead
pub async fn decrypt_bulk_stored_passwords(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    decrypt_bulk_stored_data(user_auth, stored_passwords).await
}

/// Decripta una password salvata nel database.
pub async fn decrypt_stored_password(
    pool: &SqlitePool,
    stored_password: &StoredPassword,
) -> Result<String, DBError> {
    let user_auth: UserAuth = fetch_user_auth_from_id(&pool, stored_password.user_id).await?;
    let salt = get_salt(&user_auth.password);
    let nonce = get_nonce_from_vec(&stored_password.password_nonce)?;
    let cipher = create_cipher(&salt, &user_auth)?;
    let plaintext_bytes = cipher
        .decrypt(&nonce, stored_password.password.expose_secret().as_ref())
        .map_err(|e| DBError::new_password_fetch_error(e.to_string()))?;
    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
    Ok(plaintext)
}

pub async fn create_stored_raw_password_pipeline() {}
