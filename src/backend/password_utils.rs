// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Modulo per la criptazione, decriptazione e salvataggio delle password.
//!
//! Fornisce funzioni per:
//! - Calcolare la forze di una password
//! - Criptare le password con AES-256-GCM usando Argon2 come KDF
//! - Decriptare le password salvate
//! - Salvare le password nel database

use crate::backend::db_backend::{
    fetch_all_passwords_for_user_with_filter, fetch_all_passwords_for_vault_with_filter,
    fetch_all_stored_passwords_for_user, fetch_passwords_paginated, fetch_user_auth_from_id,
    remove_temp_old_password, upsert_stored_passwords_batch,
};
use crate::backend::evaluate_password_strength;
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use crate::backend::settings_types::{DicewareGenerationSettings, DicewareLanguage};
use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, Nonce};
use argon2::password_hash::{PasswordHash, Salt};
use chrono::Utc;
use custom_errors::DBError;
use diceware::{self, EmbeddedList};
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

/// Configuration for Diceware passphrase generation.
pub struct DicewareGenConfig {
    pub word_count: usize,
    pub add_special_char: bool,
    pub numbers: u8,
    pub language: EmbeddedList,
}

impl From<DicewareGenerationSettings> for DicewareGenConfig {
    fn from(s: DicewareGenerationSettings) -> Self {
        Self {
            word_count: s.word_count as usize,
            add_special_char: s.add_special_char,
            numbers: s.numbers as u8,
            language: s.language.into(),
        }
    }
}

const MAX_DICEWARE_RETRIES: usize = 200;

/// Generate a Diceware passphrase using the given configuration.
/// Retries up to MAX_DICEWARE_RETRIES times to satisfy all criteria.
pub fn generate_diceware_password(config: DicewareGenConfig) -> Result<SecretString, String> {
    for _ in 0..MAX_DICEWARE_RETRIES {
        let lang = config.language.clone();
        let mut dw_config = diceware::Config::new()
            .with_embedded(lang)
            .with_words(config.word_count)
            .with_camel_case(true);

        if config.add_special_char {
            dw_config = dw_config.with_special_chars(true);
        }

        if let Ok(passphrase) = diceware::make_passphrase(dw_config)
            && is_valid_diceware(&passphrase, &config)
        {
            return Ok(SecretString::new(passphrase.into()));
        }
    }
    Err(format!(
        "Cannot generate a valid Diceware passphrase with these settings \
         (word_count={}, add_special_char={}, numbers={}). \
         Try disabling special chars, reducing numbers, or increasing word_count.",
        config.word_count, config.add_special_char, config.numbers
    ))
}

/// Validate a Diceware passphrase against the configuration criteria.
fn is_valid_diceware(passphrase: &str, config: &DicewareGenConfig) -> bool {
    // Split CamelCase into words
    let words: Vec<&str> = split_camel_case(passphrase);

    // Count special characters in the entire passphrase
    let special_count = passphrase.chars().filter(|c| !c.is_alphanumeric()).count();

    // Count purely numeric words
    let numeric_word_count = words
        .iter()
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_numeric()))
        .count();

    // Validate special chars
    if !config.add_special_char && special_count > 0 {
        return false;
    }
    if config.add_special_char && special_count < 1 {
        return false;
    }

    // Validate numbers
    if config.numbers == 0 && numeric_word_count > 0 {
        return false;
    }
    if config.numbers >= 1 && numeric_word_count < config.numbers as usize {
        return false;
    }

    true
}

/// Split a CamelCase string into words at uppercase boundaries.
fn split_camel_case(s: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = s.chars().collect();

    for i in 1..chars.len() {
        if chars[i].is_uppercase() {
            words.push(&s[start..s.char_indices().nth(i).unwrap().0]);
            start = s.char_indices().nth(i).unwrap().0;
        }
    }
    if start < s.len() {
        words.push(&s[start..]);
    }

    words
}

/// Detect the system language and return the corresponding Diceware language.
pub fn detect_system_language() -> DicewareLanguage {
    let locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    if locale.starts_with("it") {
        DicewareLanguage::IT
    } else if locale.starts_with("fr") {
        DicewareLanguage::FR
    } else {
        DicewareLanguage::EN
    }
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

type SecretNoteResult = (Option<SecretBox<[u8]>>, Option<Nonce<Aes256Gcm>>);

/// Cripta una stringa opzionale con AES-256-GCM.
fn encrypt_optional_note_string(
    plaintext: Option<&str>,
    cipher: &Aes256Gcm,
) -> Result<SecretNoteResult, DBError> {
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
fn get_nonce_from_vec(nonce_vec: &[u8]) -> Result<Nonce<Aes256Gcm>, DBError> {
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
    upsert_stored_passwords_batch(pool, stored_passwords).await?;

    Ok(())
}

/// Crea record StoredPassword criptando url, password e notes in parallelo.
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
                // Cripta username
                let (encrypted_username, username_nonce) =
                    encrypt_string(srp.username.expose_secret(), &cipher)?;

                // Cripta url
                let (encrypted_url, url_nonce) = encrypt_string(srp.url.expose_secret(), &cipher)?;

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
                    encrypt_optional_note_string(notes_str.as_deref(), &cipher)?;

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
                let created_at = srp
                    .created_at
                    .or_else(|| Some(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()));

                Ok(StoredPassword::new(
                    srp.id,
                    user_auth.id,
                    srp.vault_id,
                    srp.name.clone(),
                    encrypted_username,
                    username_nonce.to_vec(),
                    encrypted_url,
                    url_nonce.to_vec(),
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

/// Clona password tra vault: decripta dal vault sorgente e re-cripta nel vault target.
///
/// Il re-encryption è necessario perché ogni record ha nonce unici.
/// Copiare il ciphertext con nonce diversi produrrebbe dati illeggibili.
pub async fn clone_passwords_to_vault(
    pool: &SqlitePool,
    user_id: i64,
    password_ids: Vec<i64>,
    target_vault_id: i64,
) -> Result<(), DBError> {
    // 1. Fetch password sorgente per ID
    let all_passwords = fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let to_clone: Vec<StoredPassword> = all_passwords
        .into_iter()
        .filter(|p| p.id.is_some_and(|id| password_ids.contains(&id)))
        .collect();

    if to_clone.is_empty() {
        return Ok(());
    }

    // 2. Decripta (consuma UserAuth)
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let raw_passwords = decrypt_bulk_stored_data(user_auth, to_clone, None).await?;

    // 3. Imposta vault_id target e id: None (nuovi record)
    let cloned: Vec<StoredRawPassword> = raw_passwords
        .into_iter()
        .map(|mut rp| {
            rp.id = None;
            rp.vault_id = target_vault_id;
            rp
        })
        .collect();

    // 4. Re-cripta (genera nuovi nonce) e salva
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    let stored = create_stored_data_records(cipher, user_auth, cloned, None).await?;
    upsert_stored_passwords_batch(pool, stored).await?;

    Ok(())
}

#[cfg(test)]
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
/// tutti i dati decifrati (url è cifrata nel DB).
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
    order: &str,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords =
        fetch_all_passwords_for_user_with_filter(pool, user_id, filter, order).await?;

    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None, // Nessun progress tracking
    )
    .await?;

    Ok(stored_raw_passwords)
}

/// Recupera e decifra le password di un vault con filtro opzionale per strength.
pub async fn get_all_stored_raw_passwords_for_vault_with_filter(
    pool: &SqlitePool,
    user_id: i64,
    vault_id: i64,
    filter: Option<PasswordStrength>,
    order: &str,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords =
        fetch_all_passwords_for_vault_with_filter(pool, vault_id, filter, order).await?;

    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None,
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
#[allow(dead_code)]
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
    tracing::info!(
        "decrypt_bulk_stored_data: starting with {} passwords, progress_tx={:?}",
        stored_passwords.len(),
        progress_tx.is_some()
    );

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
                // Decripta username
                let username_nonce = get_nonce_from_vec(&sp.username_nonce)?;
                let username =
                    decrypt_to_string(sp.username.expose_secret(), &username_nonce, &cipher)?;

                // Decripta url
                let url_nonce = get_nonce_from_vec(&sp.url_nonce)?;
                let url = decrypt_to_string(sp.url.expose_secret(), &url_nonce, &cipher)?;

                // Decripta password
                let password_nonce = get_nonce_from_vec(&sp.password_nonce)?;
                let password_bytes = cipher
                    .decrypt(&password_nonce, sp.password.expose_secret())
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
                let password = String::from_utf8(password_bytes)
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;

                // Decripta notes
                let notes = match (&sp.notes, &sp.notes_nonce) {
                    (Some(enc_notes), Some(nn)) => {
                        let notes_nonce = get_nonce_from_vec(nn)?;
                        decrypt_optional_to_string(
                            Some(enc_notes.expose_secret()),
                            Some(&notes_nonce),
                            &cipher,
                        )?
                    }
                    _ => None,
                };

                // Aggiorna progress
                if let Some(tx) = &progress_tx_clone {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    tracing::debug!(
                        "decrypt_bulk_stored_data: sending progress {}/{}",
                        current,
                        total
                    );
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
                    vault_id: sp.vault_id,
                    name: sp.name.clone(),
                    username: SecretString::new(username.into()),
                    url: SecretString::new(url.into()),
                    password: SecretString::new(password.into()),
                    notes: notes.map(|n| SecretString::new(n.into())),
                    score: Some(sp.score),
                    created_at: sp.created_at,
                })
            })
            .collect::<Result<Vec<StoredRawPassword>, DBError>>()
    });

    tracing::info!("decrypt_bulk_stored_data: spawn_blocking task completed, awaiting result");

    result.await.map_err(|e| {
        tracing::error!("decrypt_bulk_stored_data: task failed with error: {}", e);
        DBError::new_password_conversion_error(format!("Join error: {}", e))
    })?
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
