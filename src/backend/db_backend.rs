// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

#![allow(dead_code)]
#[cfg(feature = "desktop")]
use crate::backend::db_key;
use crate::backend::init_queries::QUERIES;
use crate::backend::settings_types::{DicewareGenerationSettings, UserSettings};
use crate::backend::utils::verify_password;
use custom_errors::{AuthError, DBError};
use dioxus::prelude::*;
use pwd_types::{
    PasswordGeneratorConfig, PasswordPreset, PasswordStats, PasswordStrength, StoredPassword,
    UserAuth,
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteRow};
use sqlx::{Row, query};
use std::str::FromStr;
#[cfg(feature = "desktop")]
use tracing::{debug, instrument, warn};

/// Struct per rappresentare un aggiornamento utente con field opzionali
#[derive(Debug, Clone)]
pub struct UserUpdate {
    pub username: Option<String>,
    pub password: Option<SecretString>,
    pub avatar: Option<Vec<u8>>,
    /// Vecchia password hash da salvare in temp_old_password per recovery
    pub temp_old_password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SaveUserResult {
    pub user_id: i64,
    pub temp_old_password: Option<String>,
}

impl UserUpdate {
    /// Verifica se c'è almeno un campo da aggiornare
    pub fn has_updates(&self) -> bool {
        self.username.is_some() || self.password.is_some() || self.avatar.is_some()
    }

    /// Costruisce la lista dei campi SQL da aggiornare (es. "username = ?, password = ?")
    pub fn build_sql_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.username.is_some() {
            fields.push("username = ?");
        }
        if self.password.is_some() {
            fields.push("password = ?");
        }
        if self.avatar.is_some() {
            fields.push("avatar = ?");
        }
        if self.temp_old_password.is_some() {
            fields.push("temp_old_password = ?");
        }
        fields
    }
}

/// Result of database initialization.
#[cfg(feature = "desktop")]
pub enum InitResult {
    /// Normal startup or recovery completed successfully.
    Ready(SqlitePool),
    /// First setup: DB created, passphrase generated, show to user.
    FirstSetup {
        pool: SqlitePool,
        recovery_phrase: SecretString,
    },
}

/// Builds SQLCipher connect options with pinned cipher parameters.
///
/// Pinning these ensures the DB remains readable across dependency updates.
/// Defaults match SQLCipher 4.x:
/// - `cipher_page_size`: 4096 (disk page layout)
/// - `cipher_hmac_algorithm`: HMAC_SHA512 (integrity check)
/// - `kdf_iter`: 256000 (only matters for passphrase-derived keys, pinned for defense-in-depth)
///
/// The `key` pragma MUST be set first — sqlx applies pragmas in chain order.
#[cfg(feature = "desktop")]
pub fn build_sqlcipher_options(
    db_path: &str,
    key_hex: &str,
) -> Result<SqliteConnectOptions, DBError> {
    let pragma_key = format!("\"x'{}'\"", key_hex);
    let connect_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .pragma("key", pragma_key)
        .pragma("cipher_page_size", "4096")
        .pragma("cipher_hmac_algorithm", "HMAC_SHA512")
        .pragma("kdf_iter", "256000")
        .pragma("foreign_keys", "ON")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    Ok(connect_options)
}

/// Returns the database file path (CWD-relative).
#[cfg(feature = "desktop")]
pub fn get_db_path() -> Result<String, DBError> {
    std::env::current_dir()
        .unwrap_or_default()
        .join("database.db")
        .to_str()
        .ok_or_else(|| DBError::new_general_error("Invalid DB path".into()))
        .map(|s| s.to_string())
}

/// Runs all initialization queries (table creation) on the given pool.
/// Extracted as shared helper for `init_db()` and `perform_setup()`.
#[cfg(feature = "desktop")]
async fn run_init_queries(pool: &SqlitePool) -> Result<(), DBError> {
    for init_query in QUERIES {
        query(init_query)
            .execute(pool)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    }
    Ok(())
}

/// Shared setup logic: derives key from passphrase, stores in keyring, creates DB with all tables.
/// Used by both `init_db()` (app startup) and `run_setup()` (NSIS installer).
///
/// Parameters:
/// - `passphrase`: the recovery passphrase (random in prod, fixed in dev)
/// - `service_name`: the keyring service name (from `keyring_service_name()` or `SERVICE_NAME`)
///
/// Returns: `(recovery_phrase as SecretString, SqlitePool)`
#[cfg(feature = "desktop")]
pub(crate) async fn perform_setup(
    passphrase: &str,
    service_name: &str,
) -> Result<(SecretString, SqlitePool), DBError> {
    let db_path = get_db_path()?;

    let db_key_value = tokio::task::spawn_blocking({
        let passphrase = passphrase.to_string();
        let db_path = db_path.clone();
        let service_name = service_name.to_string();
        move || db_key::generate_and_store_key(&passphrase, &service_name, &db_path)
    })
    .await
    .map_err(|e| DBError::new_general_error(format!("Key derivation task failed: {}", e)))?
    .map_err(|e| DBError::new_general_error(format!("Key setup failed: {}", e)))?;

    let connect_options = build_sqlcipher_options(&db_path, &db_key_value)?.create_if_missing(true);

    let pool = SqlitePool::connect_with(connect_options)
        .await
        .map_err(|e| DBError::new_general_error(format!("Failed to create database: {}", e)))?;

    run_init_queries(&pool).await?;

    Ok((SecretString::new(passphrase.to_string().into()), pool))
}

/// Initializes the encrypted SQLite database using SQLCipher with the OS keyring key.
///
/// On first run, generates a diceware recovery passphrase, derives a key via Argon2id,
/// stores it in the OS keyring, creates the encrypted DB, and returns `InitResult::FirstSetup`
/// so the UI can show the passphrase to the user.
///
/// On subsequent runs, tries to open the DB with the key from the OS keyring.
/// If the keyring key doesn't work, returns `DBError::DBKeyMissingWithDb` so the UI
/// can show the recovery dialog.
#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<InitResult, DBError> {
    let db_path = get_db_path()?;

    let db_exists = std::path::Path::new(&db_path).exists();
    let salt_path = db_key::salt_file_path(&db_path);
    let salt_exists = std::path::Path::new(&salt_path).exists();

    if !db_exists && !salt_exists {
        // FIRST SETUP — dev only: auto-create DB with fixed passphrase.
        // Release builds use --setup (NSIS installer) to create the DB and
        // display the recovery passphrase before the app launches.
        #[cfg(debug_assertions)]
        {
            debug!("First setup: generating recovery key and creating database (dev)");
            let passphrase = db_key::DEV_RECOVERY_PASSPHRASE.to_string();
            let (recovery_phrase, pool) =
                perform_setup(&passphrase, db_key::keyring_service_name()).await?;
            return Ok(InitResult::FirstSetup {
                pool,
                recovery_phrase,
            });
        }

        #[cfg(not(debug_assertions))]
        {
            return Err(DBError::new_general_error(
                "Database not initialized. Run with --setup to initialize.".into(),
            ));
        }
    }

    // DB exists or salt exists → try normal startup
    if !salt_exists {
        return Err(DBError::new_salt_file_error(
            "Salt file missing or corrupted. Database reset is required.".into(),
        ));
    }

    // Try to get key from keyring
    let keyring_result =
        db_key::retrieve_db_key(db_key::keyring_service_name(), db_key::KEY_USERNAME);

    match keyring_result {
        Ok(key) => {
            let connect_options = build_sqlcipher_options(&db_path, &key)?;

            match SqlitePool::connect_with(connect_options).await {
                Ok(pool) => Ok(InitResult::Ready(pool)),
                Err(e) => {
                    tracing::warn!("DB open failed with keyring key: {}", e);
                    Err(DBError::new_key_missing_with_db())
                }
            }
        }
        Err(db_key::DBKeyError::NoEntry) => Err(DBError::new_key_missing_with_db()),
        Err(e) => Err(DBError::new_general_error(format!("Keyring error: {}", e))),
    }
}

/// Re-encrypts the database with a new key derived from a fresh diceware passphrase.
///
/// Flow:
/// 1. Retrieve current key from keyring (to open the DB with the old key)
/// 2. Back up current salt (to restore if rekey fails)
/// 3. Generate new diceware passphrase + salt + derived key (spawn_blocking)
/// 4. Write new salt file
/// 5. Open temp connection with old key, execute `PRAGMA rekey` with new key
/// 6. On failure: restore old salt file
/// 7. On success: update keyring with new key, return new passphrase
#[cfg(feature = "desktop")]
pub async fn rekey_database() -> Result<secrecy::SecretString, DBError> {
    let db_path = get_db_path()?;
    let service_name = db_key::keyring_service_name();

    // 1. Get current key from keyring
    let old_key = db_key::retrieve_db_key(service_name, db_key::KEY_USERNAME)
        .map_err(|e| DBError::new_general_error(format!("Cannot retrieve current key: {}", e)))?;

    // 2. Back up current salt (to restore if rekey fails)
    let old_salt = db_key::read_salt(&db_path)
        .map_err(|e| DBError::new_general_error(format!("Cannot read current salt: {}", e)))?;

    // 3. Generate new passphrase + salt + key (CPU-bound)
    let (new_passphrase, new_salt, new_key) = tokio::task::spawn_blocking({
        move || {
            let passphrase = db_key::generate_recovery_passphrase()?;
            let salt = db_key::generate_db_salt();
            let key = db_key::derive_key(&passphrase, &salt)?;
            Ok::<(String, [u8; 16], String), db_key::DBKeyError>((passphrase, salt, key))
        }
    })
    .await
    .map_err(|e| DBError::new_general_error(format!("Key derivation panicked: {}", e)))?
    .map_err(|e| DBError::new_general_error(format!("Key derivation failed: {}", e)))?;

    // 4. Write new salt file
    db_key::write_salt(&db_path, &new_salt)
        .map_err(|e| DBError::new_general_error(format!("Cannot write new salt: {}", e)))?;

    // 5. Open temp connection with old key, execute PRAGMA rekey
    let pragma_rekey = format!("PRAGMA rekey = \"x'{}'\"", new_key);
    let connect_opts = build_sqlcipher_options(&db_path, &old_key)?;

    let rekey_result = SqlitePool::connect_with(connect_opts).await;
    match rekey_result {
        Ok(temp_pool) => match sqlx::query(&pragma_rekey).execute(&temp_pool).await {
            Ok(_) => {
                drop(temp_pool);
                // 7. Update keyring with new key
                db_key::store_db_key(service_name, db_key::KEY_USERNAME, &new_key).map_err(
                    |e| {
                        DBError::new_general_error(format!(
                            "Cannot store new key in keyring: {}",
                            e
                        ))
                    },
                )?;
                Ok(secrecy::SecretString::new(new_passphrase.into()))
            }
            Err(e) => {
                drop(temp_pool);
                // 6. Restore old salt file on failure
                let _ = db_key::write_salt(&db_path, &old_salt);
                Err(DBError::new_general_error(format!(
                    "PRAGMA rekey failed: {}",
                    e
                )))
            }
        },
        Err(e) => {
            // 6. Restore old salt file on failure
            let _ = db_key::write_salt(&db_path, &old_salt);
            Err(DBError::new_general_error(format!(
                "Failed to connect with old key: {}",
                e
            )))
        }
    }
}

/// Prepara l'aggiornamento utente recuperando la vecchia password se necessario.
///
/// Questa funzione gestisce la logica di preparazione per l'aggiornamento utente:
/// - Crea una struct `UserUpdate` con i campi forniti
/// - Se viene fornita una nuova password, prima recupera e salva quella corrente in `temp_old_password`
/// - Cripta la nuova password se fornita
///
/// # Parametri
///
/// * `pool` - Pool SQLite per le operazioni sul database
/// * `user_id` - ID dell'utente da aggiornare
/// * `username` - Nuovo username (se fornito)
/// * `password` - Nuova password opzionale (se fornita, viene criptata)
/// * `avatar` - Nuovo avatar opzionale come bytes
///
/// # Valore Restituito
///
/// Return `UserUpdate` con i campi da aggiornare
///
/// # Errori
///
/// - `DBError::new_save_error` - Errore durante la criptazione della password
async fn prepare_user_update(
    pool: &SqlitePool,
    user_id: i64,
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<UserUpdate, DBError> {
    let mut update = UserUpdate {
        username: Some(username),
        password: None,
        avatar,
        temp_old_password: None,
    };

    if let Some(psw) = password
        && !psw.expose_secret().trim().is_empty()
    {
        // Backup della vecchia password hash prima di sovrascriverla
        match fetch_user_auth_from_id(pool, user_id).await {
            Ok(user_auth) => {
                // Salva la vecchia password nella struct per includerla nell'UPDATE
                // password.0 è SecretBox<str>, convertiamo in String
                update.temp_old_password = Some(user_auth.password.0.expose_secret().to_string());
            }
            Err(e) => {
                // Non bloccare l'aggiornamento, ma logga il problema
                warn!(
                    user_id = user_id,
                    error = %e,
                    "Failed to backup old password during user update - recovery may be unavailable"
                );
            }
        }

        let hash_password = crate::backend::utils::encrypt(psw.clone())
            .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
        update.password = Some(SecretString::new(hash_password.into()));
    }

    Ok(update)
}

/// Salva un nuovo utente o aggiorna uno esistente nel database.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `id` - Se `Some(i64)` fa l'UPDATE dell'utente esistente. Se `None` fa l'INSERT di un nuovo utente.
/// * `username` - Username dell'utente
/// * `password` - Password opzionale (se fornita, viene criptata con Argon2)
/// * `avatar` - Avatar opzionale come bytes (immagine PNG)
///
/// # Valore Restituito
///
/// Return `Ok(())` se il salvataggio/aggiornamento ha successo
///
/// # Comportamento
///
/// - **INSERT** (`id = None`): Crea un nuovo utente con la password criptata
/// - **UPDATE** (`id = Some(user_id)`): Aggiorna solo i campi forniti (username, password, avatar)
///   - Se la password viene aggiornata, quella corrente viene prima salvata in `temp_old_password`
///   - Se nessun campo è stato fornito, restituisce `Ok(())` senza fare nulla
///
/// # Errori
///
/// - `DBError::new_save_error` - Errore durante l'INSERT/UPDATE
/// - `DBError::new_save_error` - Errore durante la criptazione della password
pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i64>, // Se Some, fa l'UPDATE. Se None, fa l'INSERT.
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<SaveUserResult, DBError> {
    debug!("Attempting to save/update user credentials");

    // 1. Criptazione comune a entrambi i casi

    match id {
        // --- CASO UPDATE ---
        Some(user_id) => {
            let update = prepare_user_update(pool, user_id, username, password, avatar).await?;
            let result = SaveUserResult {
                user_id,
                temp_old_password: update.temp_old_password.clone(),
            };
            if !update.has_updates() {
                return Ok(result);
            }

            let sql_fields = update.build_sql_fields();
            let sql = format!("UPDATE users SET {} WHERE id = ?", sql_fields.join(", "));

            let mut query = sqlx::query(&sql);

            // Binda in ordine: prima i campi dell'update, poi user_id
            if let Some(username) = update.username {
                query = query.bind(username);
            }
            if let Some(password) = update.password {
                query = query.bind(password.expose_secret().to_string());
            }
            if let Some(avatar) = update.avatar {
                query = query.bind(avatar);
            }
            if let Some(temp_old_password) = update.temp_old_password {
                query = query.bind(temp_old_password);
            }
            query = query.bind(user_id);

            query
                .execute(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;

            Ok(result)
        }
        // --- CASO INSERT ---
        None => {
            let psw = password.unwrap_or_default();
            if !psw.expose_secret().trim().is_empty() {
                let hash_password = crate::backend::utils::encrypt(psw)
                    .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;

                // query_scalar with fetch_one returns Result<i64, Error>
                // If INSERT fails, we get an error. RETURNING id guarantees a value if INSERT succeeds.
                let user_id: i64 = sqlx::query_scalar::<_, i64>(
                    "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id",
                )
                .bind(&username)
                .bind(&hash_password)
                .bind(&avatar)
                .fetch_one(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;
                let result = SaveUserResult {
                    user_id,
                    temp_old_password: None,
                };
                Ok(result)
            } else {
                Err(DBError::new_save_error("Password cannot be empty".into()))
            }
        }
    }
}

/// Crea i settings di default per un nuovo utente.
///
/// Usa una transazione per garantire atomicità tra i due INSERT.
/// Se la transazione fallisce, viene automaticamente rollbackata.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente per cui creare i settings
/// * `preset` - Preset di default per la generazione password
///
/// # Valore Restituito
///
/// Return `Ok(())` se i settings vengono creati con successo
///
/// # Errori
///
/// - `DBError::new_transaction_error` - Errore nell'avviare o committare la transazione
/// - `DBError::new_settings_error` - Errore durante l'INSERT dei settings
#[instrument(skip(pool))]
pub async fn create_user_settings(
    pool: &SqlitePool,
    user_id: i64,
    preset: PasswordPreset,
) -> Result<i64, DBError> {
    debug!("Creating default settings for user_id: {}", user_id);

    // Inizia transazione - verrà automaticamente rollbackata se droppata
    let mut tx = pool.begin().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to begin transaction: {}", e))
    })?;

    // 1. Inserisci user_settings e ottieni l'id con RETURNING
    let settings_id: i64 =
        sqlx::query_scalar::<_, i64>("INSERT INTO user_settings (user_id) VALUES (?) RETURNING id")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                DBError::new_settings_error(format!("Failed to insert user_settings: {}", e))
            })?;

    // 2. Inserisci passwords_generation_settings
    let config = preset.to_config(settings_id);
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(config.settings_id)
    .bind(config.length)
    .bind(config.symbols)
    .bind(config.numbers)
    .bind(config.uppercase)
    .bind(config.lowercase)
    .execute(&mut *tx)
    .await
    .map_err(|e| DBError::new_settings_error(format!("Failed to insert gen_settings: {}", e)))?;

    // Commit transazione
    tx.commit().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(settings_id)
}

/// Registra un nuovo utente con i settings di default in modo atomico.
///
/// Questa funzione garantisce atomicità usando una **singola transazione DB**.
/// Se qualsiasi operazione fallisce, il DB fa automaticamente rollback.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `username` - Username del nuovo utente
/// * `password` - Password (verrà criptata con Argon2)
/// * `avatar` - Avatar opzionale come bytes
/// * `preset` - Preset per i settings di generazione password
///
/// # Valore Restituito
///
/// Return `Ok(user_id)` se la registrazione ha successo
///
/// # Errori
///
/// - `DBError::new_registration_error` - Errore durante la registrazione
/// - `DBError::new_transaction_error` - Errore nell'avviare o committare la transazione
///
/// # Atomicità
///
/// Il pattern RAII di SQLx garantisce che se la funzione ritorna errore o panicca,
/// la transazione viene automaticamente rollbackata dal Drop del tipo Transaction.
#[instrument(skip(pool, password, avatar))]
pub async fn register_user_with_settings(
    pool: &SqlitePool,
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
    preset: PasswordPreset,
) -> Result<i64, DBError> {
    debug!("Attempting atomic user registration with single transaction");

    // 1. Inizia transazione - RAII: verrà rollbackata automaticamente se droppata senza commit
    let mut tx = pool.begin().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to begin transaction: {}", e))
    })?;

    // 2. Cripta la password
    let psw = password.unwrap_or_default();
    if psw.expose_secret().trim().is_empty() {
        return Err(DBError::new_registration_error(
            "Password cannot be empty".into(),
        ));
    }

    let hash_password = crate::backend::utils::encrypt(psw).map_err(|e| {
        DBError::new_registration_error(format!("Failed to encrypt password: {}", e))
    })?;

    // 3. Inserisci utente e ottieni l'id
    let user_id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(&username)
    .bind(&hash_password)
    .bind(&avatar)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| DBError::new_registration_error(format!("Failed to insert user: {}", e)))?;

    debug!(
        user_id = user_id,
        "User created in transaction, now creating settings"
    );

    // 4. Inserisci user_settings e ottieni l'id
    let settings_id: i64 =
        sqlx::query_scalar::<_, i64>("INSERT INTO user_settings (user_id) VALUES (?) RETURNING id")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                DBError::new_registration_error(format!("Failed to insert user_settings: {}", e))
            })?;

    // 5. Inserisci passwords_generation_settings
    let config = preset.to_config(settings_id);
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(config.settings_id)
    .bind(config.length)
    .bind(config.symbols)
    .bind(config.numbers)
    .bind(config.uppercase)
    .bind(config.lowercase)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        DBError::new_registration_error(format!("Failed to insert gen_settings: {}", e))
    })?;

    // 5b. Insert default Diceware generation settings
    let default_language = crate::backend::password_utils::detect_system_language();
    sqlx::query(
        "INSERT INTO diceware_generation_settings (settings_id, word_count, add_special_char, numbers, language)
         VALUES (?, 6, 0, 0, ?)"
    )
    .bind(settings_id)
    .bind(default_language)
    .execute(&mut *tx)
    .await
    .map_err(|e| DBError::new_registration_error(format!("Failed to insert diceware settings: {}", e)))?;

    // 6. Commit - solo se tutto è andato bene
    tx.commit().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to commit transaction: {}", e))
    })?;

    debug!(
        user_id = user_id,
        "Atomic registration completed successfully"
    );
    Ok(user_id)
}

/// Cancella un utente dal database.
///
/// La cancellazione elimina l'utente e tutte le password associate grazie
/// alla `FOREIGN KEY(user_id) ON DELETE CASCADE` sulla tabella passwords.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `id` - ID dell'utente da cancellare
///
/// # Valore Restituito
///
/// Return `Ok(())` se la cancellazione ha successo
///
/// # Errori
///
/// - `DBError::new_delete_error` - Errore durante la cancellazione
#[instrument(fields(user_id = id))]
pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<(), DBError> {
    debug!(user_id = id, "Attempting to delete user from database");
    let _ = query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| DBError::new_delete_error(format!("Failed to delete user: {}", e)))?;

    Ok(())
}

/// recupera i dati base di un utente dal database.
///
/// # Parametri
///
/// * `row` - La riga del database contenete i dati dell'utente
///
/// # Valòre Restituito
///
/// Return tupla con:
/// - `i64` - ID dell'utente
/// - `String` - Username
/// - `String` - Data di creazione (formato ISO 8601)
/// - `Option<Vec<u8>>` - Avatar come bytes (opzionale)
fn get_user_row(row: SqliteRow) -> (i64, String, String, Option<Vec<u8>>) {
    (
        row.get::<i64, _>("id"),
        row.get::<String, _>("username"),
        row.get::<String, _>("created_at"),
        row.get::<Option<Vec<u8>>, _>("avatar"),
    )
}

/// Recupera la password hash di un utente dal database.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `username` - Username dell'utente di cui recuperare la password
///
/// # Valore Restituito
///
/// Return `String` - La password hash (Argon2) dell'utente
///
/// # Errori
///
/// - `DBError::new_select_error` - Utente non trovato
/// - `DBError::new_fetch_error` - Errore durante la query
#[instrument(skip(pool))]
pub async fn fetch_user_password(pool: &SqlitePool, username: &str) -> Result<String, DBError> {
    debug!("Fetching user credentials in database");
    let row = query("SELECT password FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await;
    match row {
        Ok(Some(row)) => Ok(row.get(0)),
        Ok(None) => Err(DBError::new_select_error("User not found".into())),
        Err(e) => Err(DBError::new_fetch_error(format!(
            "Failed to fetch user credentials: {}",
            e
        ))),
    }
}

/// Recupera la password e la data di creazione di un utente dal database.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente di cui recuperare i dati
///
/// # Valore Restituito
///
/// Return `UserAuth` - Struct contenete password hash e data di creazione
///
/// # Errori
///
/// - `DBError::new_select_error` - Utente non trovato
/// - `DBError::new_select_error` - Errore durante la query
#[instrument(skip(pool))]
pub async fn fetch_user_auth_from_id(pool: &SqlitePool, user_id: i64) -> Result<UserAuth, DBError> {
    debug!("Fetching user credentials in database");
    let user_auth = sqlx::query_as::<_, UserAuth>("SELECT id, password FROM users WHERE id = ?")
        .bind(user_id) // SQLite preferisce i64 per gli ID
        .fetch_optional(pool) // Rimosso & perché pool è già un riferimento o clonabile
        .await
        .map_err(|e| DBError::new_select_error(e.to_string()))?; // Cattura l'errore reale del DB
    // Ora gestisci il caso in cui la query ha avuto successo ma non ha trovato righe
    user_auth.ok_or_else(|| DBError::new_select_error("User not found".into()))
}

type UserDataResult = (i64, String, String, Option<Vec<u8>>);

/// Recupera tutti i dati di un utente dal database.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `username` - Username dell'utente di cui recuperare i dati
///
/// # Valore Restituito
///
/// Return `tute (i64, String, String, Option<Vec<u8>>)` - (ID, username, created_at, avatar)
///
/// # Errori
///
/// - `DBError::new_select_error` - Utente non trovato
/// - `DBError::new_fetch_error` - Errore durante la query
#[instrument(skip(pool))]
pub async fn fetch_user_data(pool: &SqlitePool, username: &str) -> Result<UserDataResult, DBError> {
    debug!("Fetching user credentials in database");
    let row = query("SELECT id, username, created_at, avatar FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await;
    match row {
        Ok(Some(row)) => Ok(get_user_row(row)),
        Ok(None) => Err(DBError::new_select_error("User not found".into())),
        Err(e) => Err(DBError::new_fetch_error(format!(
            "Failed to fetch user data: {}",
            e
        ))),
    }
}

/// Verifica le credenziali di un utente.
///
/// Recupera la password hash dal database e la confronta con quella fornita
/// usando `verify_password` che usa Argon2.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `username` - Username dell'utente da verificare
/// * `password` - Password in chiaro da verificare (deve essere quella non criptata)
///
/// # Valore Restituito
///
/// Return `Ok(())` se le credenziali sono corrette
///
/// # Errori
///
/// - `AuthError::DB` - Errore nel recupero della password dal database
/// - `AuthError::Decryption` - Password errata o errore nella verifica
#[instrument(skip(pool))]
pub async fn check_user(
    pool: &SqlitePool,
    username: &str,
    password: &SecretString,
) -> Result<(), AuthError> {
    debug!("Checking user credentials in database");
    let password = SecretString::new(password.expose_secret().into());
    let hash = fetch_user_password(pool, username)
        .await
        .map_err(AuthError::DB)?;
    verify_password(password, hash.as_str()).map_err(AuthError::Decryption)?;
    Ok(())
}

/// Recupera tutte le password di un utente dal database.
///
/// Utilizza il builder pattern di sqlx-template per:
/// - Filtrare per `user_id`
/// - Ordinare per `created_at` decrescente
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente di cui recuperare le password
///
/// # Valore Restituito
///
/// Return `Vec<StoredPassword>` - Lista di tutte le password dell'utente
///
/// # Errori
///
/// - `DBError::new_list_error` - Errore nel builder o nella query
#[instrument(skip(pool))]
pub async fn fetch_all_stored_passwords_for_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!("Fetching all passwords for user_id: {}", user_id);

    // Builder pattern: filtra per user_id, ordina per created_at desc
    let builder = StoredPassword::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .order_by_created_at_desc()
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;

    builder
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch passwords: {}", e)))
}

/// Fetch passwords paginate con filtro opzionale per strength.
///
/// Restituisce `StoredPassword` (dati cifrati). Per ottenere password decifrate,
/// usare `get_stored_raw_passwords_paginated` da `password_utils`.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
/// * `page` - Pagina (0-indexed)
/// * `page_size` - Numero di elementi per pagina
///
/// # Returns
/// * `Ok((Vec<StoredPassword>, u64))` - Passwords cifrate e totale count
/// * `Err(DBError)` - Errore database
#[instrument(skip(pool))]
pub async fn fetch_passwords_paginated(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredPassword>, u64), DBError> {
    debug!(
        "Fetching passwords paginated: user_id={}, filter={:?}, page={}, page_size={}",
        user_id, filter, page, page_size
    );

    // Mappa filtro strength → range di score
    let (min_score, max_score) = match filter {
        None => (None, None), // Nessun filtro: tutte le password
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => {
            // Range impossibile: score >= 255 AND score <= 0 → nessun risultato
            (Some(255), Some(0))
        }
    };

    let offset = page as i64 * page_size as i64;

    // Query raw SQL con filtro score dinamico
    let results = match (min_score, max_score) {
        (None, None) => {
            // Nessun filtro: tutte le password dell'utente
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(user_id)
            .bind(page_size as i32)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch paginated passwords: {}", e))
            })?
        }
        (Some(min), Some(max)) => {
            // Filtro range score
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ? AND score >= ? AND score <= ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(user_id)
            .bind(min)
            .bind(max)
            .bind(page_size as i32)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch paginated passwords: {}", e))
            })?
        }
        _ => unreachable!("min_score e max_score sono sempre entrambi Some o entrambi None"),
    };

    // Count totale per la paginazione (con stesso filtro)
    let total: (i64,) = match (min_score, max_score) {
        (None, None) => sqlx::query_as("SELECT COUNT(*) FROM passwords WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| DBError::new_list_error(format!("Failed to count passwords: {}", e)))?,
        (Some(min), Some(max)) => sqlx::query_as(
            "SELECT COUNT(*) FROM passwords WHERE user_id = ? AND score >= ? AND score <= ?",
        )
        .bind(user_id)
        .bind(min)
        .bind(max)
        .fetch_one(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to count passwords: {}", e)))?,
        _ => unreachable!(),
    };

    Ok((results, total.0 as u64))
}

/// Recupera TUTTE le password di un utente con filtro opzionale per strength.
///
/// A differenza di `fetch_passwords_paginated`, questa funzione restituisce
/// tutti i record senza paginazione. L'ordinamento rimane `created_at DESC`.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
///
/// # Returns
/// * `Ok(Vec<StoredPassword>)` - Tutte le password cifrate che matchano il filtro
/// * `Err(DBError)` - Errore database
#[instrument(skip(pool))]
pub async fn fetch_all_passwords_for_user_with_filter(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    order: &str,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!(
        "Fetching all passwords for user_id={} with filter={:?}, order={}",
        user_id, filter, order
    );

    // Mappa filtro strength → range di score
    let (min_score, max_score) = match filter {
        None => (None, None),
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => (Some(255), Some(0)),
    };

    let results = match (min_score, max_score) {
        (None, None) => sqlx::query_as::<_, StoredPassword>(&format!(
            r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ?
                ORDER BY {order}
                "#
        ))
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch all passwords: {}", e)))?,
        (Some(min), Some(max)) => sqlx::query_as::<_, StoredPassword>(&format!(
            r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ? AND score >= ? AND score <= ?
                ORDER BY {order}
                "#
        ))
        .bind(user_id)
        .bind(min)
        .bind(max)
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch all passwords: {}", e)))?,
        _ => unreachable!("min_score e max_score sono sempre entrambi Some o entrambi None"),
    };

    Ok(results)
}

/// Fetch statistiche password per l'utente (conteggi per strength).
///
/// Questa query è sempre "fresca" perché viene eseguita separatamente
/// dalla paginazione e non viene cacheata.
#[instrument(skip(pool))]
pub async fn fetch_password_stats(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<PasswordStats, DBError> {
    debug!("Fetching password stats for user_id: {}", user_id);

    // Query con CASE per raggruppare per strength
    let rows = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            CASE
                WHEN score < 50 THEN 0
                WHEN score < 70 THEN 1
                WHEN score < 85 THEN 2
                WHEN score < 96 THEN 3
                ELSE 4
            END as strength_group,
            COUNT(*) as count
        FROM passwords
        WHERE user_id = ?
        GROUP BY strength_group
        ORDER BY strength_group
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| DBError::new_list_error(format!("Failed to fetch password stats: {}", e)))?;

    let mut stats = PasswordStats::default();

    for (group, count) in rows {
        match group {
            0 => stats.weak = count as usize,
            1 => stats.medium = count as usize,
            2 => stats.strong = count as usize,
            3 => stats.epic = count as usize,
            4 => stats.god = count as usize,
            _ => {}
        }
    }

    // not_evaluated rimane 0 perché la query raggruppa solo score esistenti.
    // Se necessario contarle, aggiungere branch per score IS NULL nella query.
    stats.total = stats.weak + stats.medium + stats.strong + stats.epic + stats.god;

    Ok(stats)
}

pub async fn fetch_user_settings(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<UserSettings>, DBError> {
    let user_settings = UserSettings::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_one(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch user settings: {}", e)))?;

    Ok(user_settings)
}

// a che serve questa funzione ??
// pub async fn fetch_all_user_settings(pool: &SqlitePool) -> Result<Vec<UserSettings>, DBError> {
//     let settings = UserSettings::builder_select()
//         .find_all(pool)
//         .await
//         .map_err(|e| {
//             DBError::new_list_error(format!("Failed to fetch all user settings: {}", e))
//         })?;
//
//     Ok(settings)
// }

pub async fn upsert_password_config(
    pool: &SqlitePool,
    password_config: PasswordGeneratorConfig,
) -> Result<(), DBError> {
    PasswordGeneratorConfig::upsert_by_id(&password_config, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Upsert failed: {}", e)))?;
    Ok(())
}

pub async fn fetch_user_passwords_generation_settings(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<PasswordGeneratorConfig, DBError> {
    let row = sqlx::query_as::<_, PasswordGeneratorConfig>(
        r#"SELECT
                pgs.id,
                pgs.settings_id,
                pgs.length,
                pgs.symbols,
                pgs.numbers,
                pgs.uppercase,
                pgs.lowercase,
                pgs.excluded_symbols
FROM passwords_generation_settings pgs
JOIN user_settings us ON pgs.settings_id = us.id
WHERE us.user_id = ?
                "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        DBError::new_fetch_error(format!(
            "Failed to fetch user password generation settings: {}",
            e
        ))
    })?;

    Ok(row)
}

/// Fetch Diceware generation settings for a user.
pub async fn fetch_diceware_settings(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<DicewareGenerationSettings, DBError> {
    let result = sqlx::query_as::<_, DicewareGenerationSettings>(
        "SELECT dgs.id, dgs.settings_id, dgs.word_count, dgs.add_special_char,
                dgs.numbers, dgs.language
         FROM diceware_generation_settings dgs
         JOIN user_settings us ON dgs.settings_id = us.id
         WHERE us.user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| DBError::DBSelectError(e.to_string()))?;

    Ok(result)
}

/// Save or update Diceware generation settings.
pub async fn upsert_diceware_settings(
    pool: &SqlitePool,
    settings: DicewareGenerationSettings,
) -> Result<(), DBError> {
    DicewareGenerationSettings::upsert_by_id(&settings, pool)
        .await
        .map_err(|e| DBError::DBSaveError(e.to_string()))?;
    Ok(())
}

/// Upsert batch di StoredPassword usando una transazione.
/// Se un record fallisce, tutta la transazione viene rollbackata.
#[instrument(skip(pool, passwords))]
pub async fn upsert_stored_passwords_batch(
    pool: &SqlitePool,
    passwords: Vec<StoredPassword>,
) -> Result<(), DBError> {
    if passwords.is_empty() {
        return Ok(());
    }

    debug!("Batch upserting {} passwords", passwords.len());

    // Inizia transazione - RAII: rollback automatico se droppata senza commit
    let mut tx = pool.begin().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to begin transaction: {}", e))
    })?;

    for stored_password in &passwords {
        // Validazione
        if stored_password.password.expose_secret().is_empty()
            || stored_password.url.expose_secret().is_empty()
        {
            return Err(DBError::new_password_save_error(
                "Password and url cannot be empty".into(),
            ));
        }

        // Upsert singolo dentro la transazione
        StoredPassword::upsert_by_id(stored_password, &mut *tx)
            .await
            .map_err(|e| DBError::new_password_save_error(format!("Upsert failed: {}", e)))?;
    }

    // Commit - solo se tutto è andato bene
    tx.commit().await.map_err(|e| {
        DBError::new_transaction_error(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(())
}

#[instrument(fields(password_id = id))]
pub async fn delete_stored_password(pool: &SqlitePool, id: i64) -> Result<(), DBError> {
    debug!(
        user_id = id,
        "Attempting to delete stored password from database"
    );
    let _ = query("DELETE FROM passwords WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_password_delete_error(format!("Failed to delete password: {}", e))
        })?;

    Ok(())
}

#[instrument(fields(user_id = user_id))]
pub async fn delete_all_user_stored_passwords(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<(), DBError> {
    let _ = query("DELETE FROM passwords WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_password_delete_error(format!("Failed to delete passwords: {}", e))
        })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Vault-scoped query functions
// ---------------------------------------------------------------------------

/// Fetch all passwords for a vault, ordered by created_at DESC.
#[instrument(skip(pool))]
pub async fn fetch_all_stored_passwords_for_vault(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!("Fetching all passwords for vault_id: {}", vault_id);
    let builder = StoredPassword::builder_select()
        .vault_id(&vault_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .order_by_created_at_desc()
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;
    builder
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch passwords: {}", e)))
}

/// Fetch passwords paginated for a vault with optional strength filter.
#[instrument(skip(pool))]
pub async fn fetch_passwords_paginated_for_vault(
    pool: &SqlitePool,
    vault_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredPassword>, u64), DBError> {
    debug!(
        "Fetching passwords paginated for vault: vault_id={}, filter={:?}, page={}, page_size={}",
        vault_id, filter, page, page_size
    );

    // Mappa filtro strength -> range di score
    let (min_score, max_score) = match filter {
        None => (None, None),
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => {
            // Range impossibile: score >= 255 AND score <= 0 -> nessun risultato
            (Some(255), Some(0))
        }
    };

    let offset = page as i64 * page_size as i64;

    let results = match (min_score, max_score) {
        (None, None) => {
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE vault_id = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(vault_id)
            .bind(page_size as i32)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch paginated passwords: {}", e))
            })?
        }
        (Some(min), Some(max)) => {
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE vault_id = ? AND score >= ? AND score <= ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(vault_id)
            .bind(min)
            .bind(max)
            .bind(page_size as i32)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch paginated passwords: {}", e))
            })?
        }
        _ => unreachable!("min_score e max_score sono sempre entrambi Some o entrambi None"),
    };

    // Count totale per la paginazione (con stesso filtro)
    let total: (i64,) = match (min_score, max_score) {
        (None, None) => sqlx::query_as("SELECT COUNT(*) FROM passwords WHERE vault_id = ?")
            .bind(vault_id)
            .fetch_one(pool)
            .await
            .map_err(|e| DBError::new_list_error(format!("Failed to count passwords: {}", e)))?,
        (Some(min), Some(max)) => sqlx::query_as(
            "SELECT COUNT(*) FROM passwords WHERE vault_id = ? AND score >= ? AND score <= ?",
        )
        .bind(vault_id)
        .bind(min)
        .bind(max)
        .fetch_one(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to count passwords: {}", e)))?,
        _ => unreachable!(),
    };

    Ok((results, total.0 as u64))
}

/// Fetch all passwords for a vault with optional strength filter and dynamic order.
#[instrument(skip(pool))]
pub async fn fetch_all_passwords_for_vault_with_filter(
    pool: &SqlitePool,
    vault_id: i64,
    filter: Option<PasswordStrength>,
    order: &str,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!(
        "Fetching all passwords for vault_id={} with filter={:?}, order={}",
        vault_id, filter, order
    );

    // Mappa filtro strength -> range di score
    let (min_score, max_score) = match filter {
        None => (None, None),
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => (Some(255), Some(0)),
    };

    let results = match (min_score, max_score) {
        (None, None) => sqlx::query_as::<_, StoredPassword>(&format!(
            r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE vault_id = ?
                ORDER BY {order}
                "#
        ))
        .bind(vault_id)
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch all passwords: {}", e)))?,
        (Some(min), Some(max)) => sqlx::query_as::<_, StoredPassword>(&format!(
            r#"
                SELECT id, user_id, vault_id, name, username, username_nonce, url, url_nonce,
                       password, password_nonce, notes, notes_nonce, score, created_at
                FROM passwords
                WHERE vault_id = ? AND score >= ? AND score <= ?
                ORDER BY {order}
                "#
        ))
        .bind(vault_id)
        .bind(min)
        .bind(max)
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch all passwords: {}", e)))?,
        _ => unreachable!("min_score e max_score sono sempre entrambi Some o entrambi None"),
    };

    Ok(results)
}

/// Fetch password statistics for a vault (counts grouped by strength).
#[instrument(skip(pool))]
pub async fn fetch_password_stats_for_vault(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<PasswordStats, DBError> {
    debug!("Fetching password stats for vault_id: {}", vault_id);

    let rows = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            CASE
                WHEN score < 50 THEN 0
                WHEN score < 70 THEN 1
                WHEN score < 85 THEN 2
                WHEN score < 96 THEN 3
                ELSE 4
            END as strength_group,
            COUNT(*) as count
        FROM passwords
        WHERE vault_id = ?
        GROUP BY strength_group
        ORDER BY strength_group
        "#,
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await
    .map_err(|e| DBError::new_list_error(format!("Failed to fetch password stats: {}", e)))?;

    let mut stats = PasswordStats::default();

    for (group, count) in rows {
        match group {
            0 => stats.weak = count as usize,
            1 => stats.medium = count as usize,
            2 => stats.strong = count as usize,
            3 => stats.epic = count as usize,
            4 => stats.god = count as usize,
            _ => {}
        }
    }

    stats.total = stats.weak + stats.medium + stats.strong + stats.epic + stats.god;

    Ok(stats)
}

/// Delete all passwords belonging to a vault.
#[instrument(skip(pool))]
pub async fn delete_vault_passwords(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<(), DBError> {
    debug!("Deleting all passwords in vault_id: {}", vault_id);
    sqlx::query("DELETE FROM passwords WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_password_delete_error(format!("Failed to delete vault passwords: {}", e))
        })?;
    Ok(())
}

/// Questa funzione viene usata solo per i test - NON DEVE ESSERE RIMOSSA
pub(crate) async fn fetch_user_temp_old_password(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<String>, DBError> {
    let row = query("SELECT temp_old_password FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            DBError::new_fetch_error(format!("Failed to fetch temp_old_password: {}", e))
        })?;
    Ok(row.and_then(|row| row.get::<Option<String>, _>("temp_old_password")))
}

pub async fn remove_temp_old_password(pool: &SqlitePool, user_id: i64) -> Result<(), DBError> {
    let _ = query("UPDATE users SET temp_old_password = NULL WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_fetch_error(format!("Failed to remove temp_old_password: {}", e))
        })?;
    Ok(())
}

/// Ripristina la vecchia password dalla colonna temp_old_password.
/// Utilizzato quando la migrazione fallisce per ripristinare lo stato precedente.
pub async fn restore_old_password(pool: &SqlitePool, user_id: i64) -> Result<(), DBError> {
    query(
        r#"
        UPDATE users
        SET password = temp_old_password,
            temp_old_password = NULL
        WHERE id = ?
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| DBError::new_fetch_error(format!("Failed to restore old password: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // Questo modulo può contenere test per gli helper functions stessi
    use super::*;
    use crate::backend::test_helpers::{create_test_user, setup_test_db};

    #[tokio::test]
    async fn test_fetch_user_passwords_generation_settings() {
        let pool = setup_test_db().await;
        let (user_id, _) = create_test_user(&pool, "user_generation_set", "abc", None).await;
        let _settings = create_user_settings(&pool, user_id, PasswordPreset::God)
            .await
            .unwrap();
        let passwords_generation_settings =
            fetch_user_passwords_generation_settings(&pool, user_id)
                .await
                .unwrap();
        println!(
            "######## user generation settings :{:?}",
            passwords_generation_settings
        );
        assert_eq!(
            passwords_generation_settings,
            PasswordPreset::God.to_config(passwords_generation_settings.id.unwrap())
        );
    }

    #[tokio::test]
    async fn test_get_user_auth() {
        let pool = setup_test_db().await;
        let mut error: Option<DBError> = None;
        let user_auth = match fetch_user_auth_from_id(&pool, 99).await {
            Ok(data) => {
                println!("{:?}", data);
                Some(data)
            }
            Err(e) => {
                println!("User auth not found for user_id: {}", e);
                error = Some(e);
                None
            }
        };
        if let Some(e) = error {
            println!("{:?}", e);
        } else {
            println!("{:?}", user_auth)
        }
    }
}
