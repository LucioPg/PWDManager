//! Modulo per la criptazione, decriptazione e salvataggio delle password.
//!
//! Fornisce funzioni per:
//! - Calcolare la forze di una password
//! - Criptare le password con AES-256-GCM usando Argon2 come KDF
//! - Decriptare le password salvate
//! - Salvare le password nel database

use crate::backend::db_backend::{
    fetch_password_created_at_from_id, save_or_update_stored_password,
};
use crate::backend::user_auth_helper::{
    DbSecretString, DbSecretVec, PasswordStrength, StoredPassword, StoredRawPassword, UserAuth,
};
use aes_gcm::aead::{Aead, AeadCore, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use argon2::password_hash::Salt;
use argon2::{Argon2, PasswordHash};
use custom_errors::DBError;
use rayon::prelude::*;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::task;
use tokio_util::sync::CancellationToken;

/// Calcola la forze di una password in base alla sua lunghezza.
///
/// # Parametri
///
/// * `password` - La password di cui calcolare la forze (in chiaro)
///
/// # Valore Restituito
///
/// Return `PasswordStrength`:
/// - `WEAK` - Meno di 8 caratteri
/// - `MEDIUM` - Tra 8 e 15 caratteri
/// - `STRONG` - 16 o più caratteri
pub async fn calc_strength(password: &SecretString) -> PasswordStrength {
    if password.expose_secret().len() < 8 {
        return PasswordStrength::WEAK;
    };
    if password.expose_secret().len() >= 8 && password.expose_secret().len() < 16 {
        return PasswordStrength::MEDIUM;
    };
    PasswordStrength::STRONG
}

pub fn calc_strength_sync(password: &str) -> PasswordStrength {
    if password.len() < 8 {
        return PasswordStrength::WEAK;
    };
    if password.len() >= 8 && password.len() < 16 {
        return PasswordStrength::MEDIUM;
    };
    PasswordStrength::STRONG
}

pub async fn calc_strength_channel(
    password: &str,
    token: CancellationToken,
    tx: Sender<usize>,
) -> Result<PasswordStrength, ()> {
    let password = password.to_string();

    let result = task::spawn_blocking(move || {
        if token.is_cancelled() {
            return Err(());
        }
        let stregth = calc_strength_sync(&password);
        let _ = tx.send(1);
        Ok(stregth)
    })
    .await
    .map_err(|_| ())??;
    Ok(result)
}

/*
esempio per usare la conversione enum -> text di sqlx
sqlx::query!(
    "INSERT INTO users (name, strength) VALUES (?1, ?2)",
    "Lucio",
    Strength::Strong as Strength
)
.execute(&pool)
.await?;
 */

/// Estrae il sale da una password hash Argon2.
///
/// Il sale è necessario per derivare la chiave AES della password utente.
///
/// # Parametri
///
/// * `hash_password` - Password hash (Argon2) della cui estrae il sale
///
/// # Valore Restituito
///
/// Return `Salt<'_>` - Il sale Argon2 estratto
fn get_salt(hash_password: &DbSecretString) -> Salt<'_> {
    let hash_password = hash_password.0.expose_secret();
    let parsed_hash = PasswordHash::new(hash_password).unwrap();
    parsed_hash.salt.unwrap()
}

/// Genera un nuovo nonce casuale per AES-256-GCM.
///
/// Il nonce è un vettore di 12 byte che deve essere unico per ogni password
/// per garantire la sicurezza della criptazione.
///
/// # Valore Restituito
///
/// Return `Nonce<Aes256Gcm>` - Un nuovo nonce casuale
fn create_nonce() -> Nonce<Aes256Gcm> {
    Aes256Gcm::generate_nonce(&mut OsRng)
    // (nonce, nonce.to_vec())
}

/// Converte un vettore di 12 byte in un [`Nonce<Aes256Gcm>`].
///
/// # Parametri
///
/// * `nonce_vec` - Vettore da convertire (deve essere esattamente 12 byte)
///
/// # Valore Restituito
///
/// Return `Nonce<Aes256Gcm>` - Il nonce estratto
///
/// # Errori
///
/// - `DBError::new_nonce_corruption_error` - Se il vettore non è 12 byte
fn get_nonce_from_vec(nonce_vec: &Vec<u8>) -> Result<Nonce<Aes256Gcm>, DBError> {
    if nonce_vec.len() != 12 {
        return Err(DBError::new_nonce_corruption_error());
    }
    Ok(*Nonce::<Aes256Gcm>::from_slice(&nonce_vec))
}

/// Crea un cipher AES-256-GCM usando la password utente come KDF.
///
/// La chiave AES viene derivata usando Argon2 con:
/// - Sale: estratto dalla password hash dell'utente
/// - Diversificatore: la data di creazione dell'utente
/// - Password: la password hash dell'utente
///
/// Questo garantisce che ogni utente abbia una chiave AES unica anche se
/// la password dell'utente cambia (perché sale + diversificatore rimangono uguali).
///
/// # Parametri
///
/// * `salt` - Sale Argon2 della password utente
/// * `user_auth` - Credenziali utente (password hash + data creazione)
///
/// # Valore Restituito
///
/// Return `Aes256Gcm>` - Il cipher AES-256-GCM inizializzato
///
/// # Errori
///
/// - `DBError::new_cipher_create_error` - Errore nella derivazione della chiave
pub fn create_cipher(salt: &Salt<'_>, user_auth: &UserAuth) -> Result<Aes256Gcm, DBError> {
    let mut derived_key = [0u8; 32];
    let diversificator = user_auth.created_at.to_string();
    // let new_salt = format!("{}{}", salt.as_str(), diversificator);
    Argon2::default()
        .hash_password_into(
            user_auth.password.expose_secret().as_bytes(),
            salt.as_str().as_bytes(),
            &mut derived_key,
        )
        .map_err(|e| DBError::new_cipher_create_error(e.to_string()))?;
    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key)))
}
async fn create_password_with_cipher(
    new_password: &SecretString,
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<SecretBox<[u8]>, DBError> {
    // let cipher = create_cipher(salt, user_auth)?;
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
    // let cipher = create_cipher(salt, user_auth)?;
    let cipher_vec = cipher
        .encrypt(nonce, new_password.expose_secret().as_bytes())
        .map_err(|e| DBError::new_cipher_encryption_error(e.to_string()))?;
    Ok(SecretBox::new(cipher_vec.into()))
}

/// Pipeline completa per salvare una nuova password nel database.
///
/// Esegue tutte le operazioni necessarie per salvare una password:
/// 1. Recupera le credenziali utente (password hash + created_at)
/// 2. Estrae il sale dalla password utente
/// 3. Genera un nuovo nonce per AES-GCM
/// 4. Cripta la nuova password con AES-256-GCM
/// 5. Calcola la forze della password
/// 6. Salva la password criptata nel database
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente proprietario della password
/// * `location` - Luogo/nome dove è salvata la password
/// * `raw_password` - Password in chiaro da criptare
/// * `notes` - Note opzionali
///
/// # Valore Restituito
///
/// Return `Ok(())` se il salvataggio ha successo
///
/// # Errori
///
/// - `DBError::new_password_save_error` - Errore in qualsiasi fase della pipeline
pub async fn create_stored_password_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    strength: Option<PasswordStrength>,
) -> Result<(), DBError> {
    let user_auth: UserAuth = fetch_password_created_at_from_id(&pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let nonce = create_nonce();
    let cipher = create_cipher(&salt, &user_auth)?;
    let (password, strength_result) = if strength.is_none() {
        let task_encrypt = create_password_with_cipher(&raw_password, &nonce, &cipher);
        let task_calc_strength = calc_strength(&raw_password);
        tokio::join!(task_encrypt, task_calc_strength)
    } else {
        let encrypted = create_password_with_cipher(&raw_password, &nonce, &cipher).await;
        (encrypted, strength.unwrap())
    };
    if let Ok(password) = password {
        let stored_password = StoredPassword::new(
            None,
            user_id,
            location,
            password,
            notes,
            strength_result,
            None,
            nonce.to_vec(),
        );
        save_or_update_stored_password(&pool, stored_password).await?; // questa potrebbe accettare un vec di stored password per fare in modo che vengano fatte in bulk, inoltre andrebbe estratta da questa funzione
        Ok(())
    } else {
        Err(DBError::new_password_save_error("Errore generale".into()))
    }
}

pub async fn create_stored_passwords(
    cipher: Aes256Gcm, // Assumendo che Aes256Gcm sia Send + Sync
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
    if stored_raw_passwords.is_empty() {
        return Ok(Vec::new());
    }

    // Avvolgiamo cipher e user_auth in Arc per passarli ai thread di Rayon
    let cipher = Arc::new(cipher);
    let user_auth = Arc::new(user_auth);

    // Spostiamo il calcolo pesante su un thread pool dedicato alla CPU
    task::spawn_blocking(move || {
        stored_raw_passwords
            .into_par_iter()
            .map(|spr| {
                let nonce = create_nonce();

                // Usiamo il cipher condiviso
                let encryption = create_password_with_cipher_sync(&spr.password, &nonce, &cipher)
                    .map_err(|_| {
                    DBError::new_cipher_encryption_error("Cipher error".to_string())
                })?;
                let strength_result: PasswordStrength = if spr.strength.is_none() {
                    calc_strength_sync(&spr.password.expose_secret())
                } else {
                    spr.strength.unwrap()
                };

                Ok(StoredPassword::new(
                    spr.id,
                    user_auth.id,
                    spr.location,
                    encryption, // Assunto che encryption sia il tipo corretto
                    spr.notes,
                    strength_result,
                    None,
                    nonce.to_vec(),
                ))
            })
            .collect::<Result<Vec<StoredPassword>, DBError>>() // Trasforma Vec<Result> in Result<Vec>
    })
    .await
    .map_err(|e| DBError::new_password_save_error(format!("Join error: {}", e)))?
}

async fn helper_upsert_stored_passwords(
    pool: &SqlitePool,
    stored_passwords: Vec<StoredPassword>,
) -> Result<(), DBError> {
    todo!();
}

/// Decripta una password salvata nel database.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `stored_password` - Password salvata contenente nonce e password criptata
///
/// # Valore Restituito
///
/// Return `String` - La password in chiaro
///
/// # Errori
///
/// - `DBError::new_password_fetch_error` - Errore nel recupero credenziali utente
/// - `DBError::new_password_conversion_error` - Errore nella decriptazione
/// - `DBError::new_nonce_corruption_error` - Nonce non valido (non 12 byte)
pub async fn decrypt_stored_password(
    pool: &SqlitePool,
    stored_password: &StoredPassword,
) -> Result<String, DBError> {
    let user_auth: UserAuth =
        fetch_password_created_at_from_id(&pool, stored_password.user_id).await?;
    let salt = get_salt(&user_auth.password);
    let nonce = get_nonce_from_vec(&stored_password.nonce)?;
    let cipher = create_cipher(&salt, &user_auth)?;
    let plaintext_bytes = cipher
        .decrypt(&nonce, stored_password.password.expose_secret().as_ref())
        .map_err(|e| DBError::new_password_fetch_error(e.to_string()))?;
    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
    Ok(plaintext)
}

pub async fn create_stored_raw_password_pipeline() {}

/*
PASSWORD MIGRATION:
le password salvate sono in formato vec<u8>
e non possono essere decriptate senza la master password dell'utente usata al momento della criptazione.
Si rende necessario riconvertire le password salvate in chiaro attraverso la master password precedente a quella in sostituzione
e quindi ripetere la criptazione con la master password nuova.
Quando un utente cambia la master password, la precedente viene salvata in "temp_old_password".
Riassumendo per punti il procedimento per eseguire la migrazione è questo:
1. ottenere la master password vecchia interrogando o riceverla come argomento.
2. estrarre il salt dalla master password vecchia.
3. creare il cipher con la master password vecchia.
4. estrarre il salt dalla master password nuova.
5. creare il cipher con la master password nuova.
5. creare il nonce con il salt della master password nuova.
4. decriptare le password salvate con il cipher vecchio.
5. salvare le nuove password criptate nel database.
6. update delle nuove password criptate nel database.
7. ripetere passi 4-5-6.
8. al termine rimuovere il campo "temp_old_password" dal database.

dato il grosso potenziale di carico di questo processo è necessario eseguire il processo in parallelo usando rayon all'interno di un spawn_blocking di tokio.
L'ideale sarebbe quello di fare gli update in batch di un certo numero per evitare di bloccare il thread principale.

Sarebbe una buona cosa notificare il frontend dell'avanzamento del processo di migrazione, in modo da poter mostrare un indicatore di avanzamento.
questo implicherebbe usare un contatore condiviso all'interno di un segnale e quindi aggiornare la progress bar nel frontend.

quindi dopo la modifica della password è sempre possibile eseguire il processo di migrazione, fintanto che esiste il campo "temp_old_password" nel database.
si potrebbe creare un sistema di checkpoint creando una tabella che conserva l'ultimo id processato e l'ultimo id processabile.

la re-criptazione dovrebbe accadere dopo aver modificato la password principale.
L'utente dovrebbe essere avvisato che modificando la master password potrebbe cominciare un processo di migrazione lungo ma che può essere messo in pausa.

 */
