//! Modulo per la criptazione, decriptazione e salvataggio delle password.
//!
//! Fornisce funzioni per:
//! - Calcolare la forze di una password
//! - Criptare le password con AES-256-GCM usando Argon2 come KDF
//! - Decriptare le password salvate
//! - Salvare le password nel database
#![allow(dead_code)]

use crate::backend::db_backend::{
    fetch_all_passwords_for_user_with_filter, fetch_all_stored_passwords_for_user,
    fetch_passwords_paginated, fetch_user_auth_from_id, remove_temp_old_password,
    upsert_stored_passwords_batch,
};
use crate::backend::evaluate_password_strength;
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use aes_gcm::aead::{Aead, Nonce, OsRng};
use chrono::Utc;
use aes_gcm::{Aes256Gcm, KeyInit};
use argon2::password_hash::{PasswordHash, Salt};
use custom_errors::DBError;
use pwd_crypto::{
    create_cipher as crypto_create_cipher, create_nonce,
    decrypt_optional_to_string as crypto_decrypt_optional_to_string,
    decrypt_to_string as crypto_decrypt_to_string,
    encrypt_optional_string as crypto_encrypt_optional_string,
    encrypt_string as crypto_encrypt_string, nonce_from_vec,
};
use pwd_types::{
    AegisPasswordConfig, DbSecretString, PasswordGeneratorConfig, PasswordPreset, PasswordScore,
    PasswordStrength, StoredPassword, StoredRawPassword, UserAuth,
};
use rayon::prelude::*;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task;
use uuid::Uuid;

pub fn generate_suggested_password(custom_config: Option<PasswordGeneratorConfig>) -> SecretString {
    // Se non c'è config, usa il preset God come default
    let config = custom_config.unwrap_or_else(|| PasswordPreset::God.to_config(0));

    let config_adapter: AegisPasswordConfig = config.clone().into();
    let password: String = loop {
        if let Ok(pwd) = config_adapter.generate() {
            // 1. Conta i simboli
            let sym_count = pwd.chars().filter(|c| !c.is_alphanumeric()).count();

            // 2. Controlla se contiene simboli vietati
            let has_excluded = pwd.chars().any(|c| config.excluded_symbols.contains(&c));

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
pub(crate) fn get_salt(hash_password: &DbSecretString) -> Salt<'_> {
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

/// Pipeline completa per salvare le passwords nel database in bulk/batch.
pub async fn create_stored_data_pipeline_bulk(
    pool: &SqlitePool,
    user_id: i64,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<(), DBError> {
    // 1. Recupero credenziali e setup crittografico
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    // 2. Creazione StoredPassword
    let stored_passwords =
        create_stored_data_records(cipher, user_auth, stored_raw_passwords, None).await?; // None = no progress
    // 3. Salvataggio in batch
    upsert_stored_passwords_batch(&pool, stored_passwords).await?;

    Ok(())
}

/// Crea record StoredPassword criptando location, password e notes in parallelo.
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<Vec<StoredPassword>, DBError> {
    if stored_raw_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let cipher = Arc::new(cipher);
    let user_auth = Arc::new(user_auth);
    let total = stored_raw_passwords.len();
    let completed = Arc::new(AtomicUsize::new(0));
    let progress_tx_clone = progress_tx.clone();

    task::spawn_blocking(move || {
        stored_raw_passwords
            .into_par_iter()
            .map(|srp| {
                // Cripta location
                let (encrypted_location, location_nonce) =
                    encrypt_string(srp.location.expose_secret(), &cipher)?;

                // Cripta password
                let password_nonce = create_nonce();
                let encrypted_password =
                    create_password_with_cipher_sync(&srp.password, &password_nonce, &cipher)
                        .map_err(|_| {
                            DBError::new_cipher_encryption_error("Cipher error".to_string())
                        })?;

                // Cripta notes
                let notes_str = srp.notes.as_ref().map(|n| n.expose_secret().to_string());
                let (encrypted_notes, notes_nonce) =
                    encrypt_optional_string(notes_str.as_deref(), &cipher)?;

                // Calcola score
                let score_evaluation: PasswordScore = srp.score.unwrap_or_else(|| {
                    evaluate_password_strength(&srp.password, None)
                        .score
                        .unwrap_or(PasswordScore::new(0))
                });

                // Aggiorna progress
                if let Some(tx) = &progress_tx_clone {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    // Usa try_send invece di blocking_send per evitare deadlock
                    let _ = tx.try_send(ProgressMessage::new(
                        MigrationStage::Encrypting,
                        current,
                        total,
                    ));
                }

                // Se created_at è None, calcola il timestamp corrente (formato SQLite)
                let created_at = srp.created_at.or_else(|| {
                    Some(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
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
                    created_at,
                    password_nonce.to_vec(),
                ))
            })
            .collect::<Result<Vec<StoredPassword>, DBError>>()
    })
    .await
    .map_err(|e| DBError::new_password_save_error(format!("Join error: {}", e)))?
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
        None, // Nessun progress tracking
    )
    .await?;
    Ok(stored_raw_passwords)
}

/// Recupera e decifra TUTTE le password dell'utente con filtro opzionale.
///
/// Questa funzione è usata per l'ordinamento frontend che richiede
/// tutti i dati decifrati (location è cifrata nel DB).
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
///
/// # Returns
/// * `Ok(Vec<StoredRawPassword>)` - Tutte le password decifrate
/// * `Err(DBError)` - Errore database o decriptazione
pub async fn get_all_stored_raw_passwords_with_filter(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords =
        fetch_all_passwords_for_user_with_filter(pool, user_id, filter).await?;

    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None, // Nessun progress tracking
    )
    .await?;

    Ok(stored_raw_passwords)
}

/// Recupera e decifra le password in modo paginato.
///
/// Combina `fetch_passwords_paginated` con `decrypt_bulk_stored_data`
/// per restituire password decifrate con supporto al filtro per strength.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
/// * `page` - Pagina (0-indexed)
/// * `page_size` - Numero di elementi per pagina
///
/// # Returns
/// * `Ok((Vec<StoredRawPassword>, u64))` - Password decifrate e totale count
pub async fn get_stored_raw_passwords_paginated(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredRawPassword>, u64), DBError> {
    let (stored_passwords, total_count) =
        fetch_passwords_paginated(pool, user_id, filter, page, page_size).await?;

    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None,
    )
    .await?;

    Ok((stored_raw_passwords, total_count))
}

/// Decripta in parallelo un batch di StoredPassword.
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    tracing::info!("decrypt_bulk_stored_data: starting with {} passwords, progress_tx={:?}", stored_passwords.len(), progress_tx.is_some());

    if stored_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    let cipher = Arc::new(cipher);
    let total = stored_passwords.len();
    let completed = Arc::new(AtomicUsize::new(0));
    let progress_tx_clone = progress_tx.clone();

    let result = task::spawn_blocking(move || {
        tracing::info!("decrypt_bulk_stored_data: spawn_blocking task started");
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

                // Aggiorna progress
                if let Some(tx) = &progress_tx_clone {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    tracing::debug!("decrypt_bulk_stored_data: sending progress {}/{}", current, total);
                    // Usa try_send invece di blocking_send per evitare deadlock
                    let _ = tx.try_send(ProgressMessage::new(
                        MigrationStage::Decrypting,
                        current,
                        total,
                    ));
                }

                Ok(StoredRawPassword {
                    uuid: Uuid::new_v4(),
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
    });

    tracing::info!("decrypt_bulk_stored_data: spawn_blocking task completed, awaiting result");

    result
    .await
    .map_err(|e| {
        tracing::error!("decrypt_bulk_stored_data: task failed with error: {}", e);
        DBError::new_password_conversion_error(format!("Join error: {}", e))
    })?
}

/// Deprecated: Use `decrypt_bulk_stored_data` instead
pub async fn decrypt_bulk_stored_passwords(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    decrypt_bulk_stored_data(user_auth, stored_passwords, None).await
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

pub async fn stored_passwords_migration_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    old_password: String,
) -> Result<(), DBError> {
    let data = fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let old_password = SecretString::new(old_password.into());
    let user_auth: UserAuth = UserAuth {
        id: user_id,
        password: old_password.into(),
    };
    let decrypted_data = decrypt_bulk_stored_data(user_auth, data, None).await?;
    let _ = create_stored_data_pipeline_bulk(pool, user_id, decrypted_data).await?;
    let _ = remove_temp_old_password(pool, user_id).await?;
    Ok(())
}

/// Pipeline di migrazione password con feedback di progresso.
/// Invia aggiornamenti tramite il canale mpsc fornito.
pub async fn stored_passwords_migration_pipeline_with_progress(
    pool: &SqlitePool,
    user_id: i64,
    old_password: String,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<(), DBError> {
    // Invia stato iniziale
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Decrypting, 0, 0))
            .await;
    }

    // 1. Fetch tutte le password salvate
    let data = fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let total = data.len();

    // 2. Prepara UserAuth con vecchia password
    let old_password = SecretString::new(old_password.into());
    let user_auth = UserAuth {
        id: user_id,
        password: old_password.into(),
    };

    // 3. Decrypt con progress tracking
    let decrypted_data = decrypt_bulk_stored_data(user_auth, data, progress_tx.clone()).await?;

    // Invia cambio stage
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Encrypting, 0, total))
            .await;
    }

    // 4. Recupera cipher con NUOVA password (dal DB aggiornato)
    let new_user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&new_user_auth.password);
    let cipher = create_cipher(&salt, &new_user_auth)?;

    // 5. Encrypt con progress tracking
    let encrypted_data =
        create_stored_data_records(cipher, new_user_auth, decrypted_data, progress_tx.clone())
            .await?;

    // Invia finalizzazione
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Finalizing, 0, 0))
            .await;
    }

    // 6. Salvataggio in batch
    upsert_stored_passwords_batch(pool, encrypted_data).await?;

    // 7. Rimuovi temp_old_password
    remove_temp_old_password(pool, user_id).await?;

    // Invia completamento
    if let Some(tx) = &progress_tx {
        let _ = tx
            .send(ProgressMessage::new(MigrationStage::Completed, 100, 100))
            .await;
    }

    Ok(())
}
