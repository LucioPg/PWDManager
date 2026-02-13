use crate::backend::db_backend::{
    fetch_password_created_at_from_id, save_or_update_stored_password,
};
use crate::backend::user_auth_helper::{
    DbSecretString, DbSecretVec, PasswordStrength, StoredPassword, UserAuth,
};
use aes_gcm::aead::{Aead, AeadCore, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use argon2::password_hash::Salt;
use argon2::{Argon2, PasswordHash};
use custom_errors::DBError;
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;

async fn calc_strength(password: &str) -> PasswordStrength {
    if password.len() < 8 {
        return PasswordStrength::WEAK;
    };
    if password.len() >= 8 && password.len() < 16 {
        return PasswordStrength::MEDIUM;
    };
    PasswordStrength::STRONG
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

fn get_salt(hash_password: &DbSecretString) -> Salt<'_> {
    let hash_password = hash_password.0.expose_secret();
    let parsed_hash = PasswordHash::new(hash_password).unwrap();
    parsed_hash.salt.unwrap()
}

fn create_nonce() -> Nonce<Aes256Gcm> {
    Aes256Gcm::generate_nonce(&mut OsRng)
    // (nonce, nonce.to_vec())
}

fn get_nonce_from_vec(nonce_vec: &Vec<u8>) -> Result<Nonce<Aes256Gcm>, DBError> {
    if nonce_vec.len() != 12 {
        return Err(DBError::new_nonce_corruption_error());
    }
    Ok(*Nonce::<Aes256Gcm>::from_slice(&nonce_vec))
}

fn create_cipher(salt: Salt<'_>, master_password: &DbSecretString) -> Result<Aes256Gcm, DBError> {
    let mut derived_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(
            master_password.0.expose_secret().as_bytes(),
            salt.as_str().as_bytes(),
            &mut derived_key,
        )
        .map_err(|e| DBError::new_cipher_create_error(e.to_string()))?;
    Ok(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_key)))
}
async fn create_cipher_password(
    new_password: &SecretString,
    salt: Salt<'_>,
    master_password: &DbSecretString,
    nonce: &Nonce<Aes256Gcm>,
) -> Result<Vec<u8>, DBError> {
    let cipher = create_cipher(salt, master_password)?;
    cipher
        .encrypt(nonce, new_password.expose_secret().as_bytes())
        .map_err(|e| DBError::new_password_save_error(e.to_string()))
}

async fn create_stored_password_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
) -> Result<(), DBError> {
    /*
    /// 0. fare fetch della master password e del created_at dell'utente.
    /// 1. prendere il salt della master password.
    /// 2. prendere il campo created_at dell'utente.
    /// 3. creare il nonce.
    /// Se possibile, parallelamente*:
    /// 4. derivare la criptazione con aes*
    /// 4. fare valutazione forza password*
    /// 5. creare struct StoredPassword
    /// 6. fare l'insert
     */
    let user_auth: UserAuth = fetch_password_created_at_from_id(&pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let nonce = create_nonce();
    let (password, strength) = tokio::join!(
        create_cipher_password(&raw_password, salt, &user_auth.password, &nonce),
        calc_strength(&raw_password.expose_secret())
    );
    if let Ok(password) = password {
        let stored_password = StoredPassword::new(
            None,
            user_id,
            location,
            password,
            notes,
            strength,
            None,
            nonce.to_vec(),
        );
        save_or_update_stored_password(&pool, stored_password).await?;
        Ok(())
    } else {
        Err(DBError::new_password_save_error("Errore generale".into()))
    }
}

async fn decrypt_stored_password(
    pool: &SqlitePool,
    stored_password: &StoredPassword,
) -> Result<String, DBError> {
    let user_auth: UserAuth =
        fetch_password_created_at_from_id(&pool, stored_password.user_id).await?;
    let salt = get_salt(&user_auth.password);
    let nonce = get_nonce_from_vec(&stored_password.nonce)?;
    let cipher = create_cipher(salt, &user_auth.password)?;
    let plaintext_bytes = cipher
        .decrypt(&nonce, stored_password.password.expose_secret().as_ref())
        .map_err(|e| DBError::new_password_fetch_error(e.to_string()))?;
    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
    Ok(plaintext)
}
