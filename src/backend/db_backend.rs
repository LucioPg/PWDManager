#![allow(dead_code)]
use crate::backend::init_queries::QUERIES;
use crate::backend::password_types_helper::{StoredPassword, UserAuth};
use crate::backend::settings_types::PasswordPreset;
use crate::backend::utils::verify_password;
use custom_errors::{AuthError, DBError};
use dioxus::prelude::*;
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
        fields
    }
}

/// Inizializza il database SQLite con le tabelle necessarie.
///
/// # Parametri
///
/// * `pool` - Il pool SQLite restituito (inizializzato e con WAL mode)
///
/// # Valòre Restituito
///
/// Return [`SqlitePool`](sqlx::SqlitePool) se l'inizializzazione ha successo.
///
/// # Errori
///
/// - `DBError::new_general_error` - Fallisce la connessione al database
/// - `DBError::new_general_error` con messaggio specifico per fallimento creazione tabelle
///
/// # Comportamento
///
/// 1. Configura il database in modalità WAL (Write-Ahead Logging) per concorrenza
/// 2. Abilita le foreign keys per l'integrità referenziale
/// 3. Crea le tabelle mancanti se necessario (`.create_if_missing(true)`)
/// 4. Esegue tutte le query di inizializzazione definite in `QUERIES`
#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<SqlitePool, DBError> {
    let options = SqliteConnectOptions::from_str("sqlite:database.db")
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .pragma("foreign_keys", "ON")
        .journal_mode(SqliteJournalMode::Wal) //fondamentale per la concorrenza
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| DBError::new_general_error(e.to_string()))?;
    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    }

    Ok(pool)
}

/// Salva temporaneamente la vecchia password dell'utente prima di un aggiornamento.
///
/// Questa funzione viene utilizzata internamente da `prepare_user_update` per
/// preservare la password corrente prima di applicare un aggiornamento.
/// Il valore viene salvato nel campo `temp_old_password` della tabella users.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `user_id` - ID dell'utente di cui salvare la password temporanea
/// * `password` - Password corrente da salvare temporaneamente
///
/// # Valore Restituito
///
/// Return `Ok(())` se il salvataggio ha successo
///
/// # Errori
///
/// - `DBError::new_save_temp_password_error` - Errore durante il salvataggio
async fn set_temp_password(
    pool: &SqlitePool,
    user_id: i64,
    password: &SecretString,
) -> Result<(), DBError> {
    query("UPDATE users SET temp_old_password = ? WHERE id = ?")
        .bind(password.expose_secret())
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_save_temp_password_error(format!("Failed to save temp password: {}", e))
        })?;
    Ok(())
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
    };

    if let Some(psw) = password {
        if !psw.expose_secret().trim().is_empty() {
            // Backup della vecchia password hash prima di sovrascriverla
            match fetch_user_auth_from_id(pool, user_id).await {
                Ok(user_auth) => {
                    set_temp_password(pool, user_id, &user_auth.password.0).await?;
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
) -> Result<i64, DBError> {
    debug!("Attempting to save/update user credentials");

    // 1. Criptazione comune a entrambi i casi

    match id {
        // --- CASO UPDATE ---
        Some(user_id) => {
            let update = prepare_user_update(pool, user_id, username, password, avatar).await?;

            if !update.has_updates() {
                return Ok(user_id);
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
            query = query.bind(user_id);

            query
                .execute(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;

            Ok(user_id)
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
                    "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id"
                )
                .bind(&username)
                .bind(&hash_password)
                .bind(&avatar)
                .fetch_one(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;

                Ok(user_id)
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
) -> Result<(), DBError> {
    debug!("Creating default settings for user_id: {}", user_id);

    // Inizia transazione - verrà automaticamente rollbackata se droppata
    let mut tx = pool.begin().await
        .map_err(|e| DBError::new_transaction_error(format!("Failed to begin transaction: {}", e)))?;

    // 1. Inserisci user_settings e ottieni l'id con RETURNING
    let settings_id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO user_settings (user_id) VALUES (?) RETURNING id"
    )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DBError::new_settings_error(format!("Failed to insert user_settings: {}", e)))?;

    // 2. Inserisci passwords_generation_settings
    let config = preset.to_config();
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)"
    )
        .bind(settings_id)
        .bind(config.length)
        .bind(config.symbols)
        .bind(config.numbers)
        .bind(config.uppercase)
        .bind(config.lowercase)
        .execute(&mut *tx)
        .await
        .map_err(|e| DBError::new_settings_error(format!("Failed to insert gen_settings: {}", e)))?;

    // Commit transazione
    tx.commit().await
        .map_err(|e| DBError::new_transaction_error(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
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
    let mut tx = pool.begin().await
        .map_err(|e| DBError::new_transaction_error(format!("Failed to begin transaction: {}", e)))?;

    // 2. Cripta la password
    let psw = password.unwrap_or_default();
    if psw.expose_secret().trim().is_empty() {
        return Err(DBError::new_registration_error("Password cannot be empty".into()));
    }

    let hash_password = crate::backend::utils::encrypt(psw)
        .map_err(|e| DBError::new_registration_error(format!("Failed to encrypt password: {}", e)))?;

    // 3. Inserisci utente e ottieni l'id
    let user_id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id"
    )
    .bind(&username)
    .bind(&hash_password)
    .bind(&avatar)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| DBError::new_registration_error(format!("Failed to insert user: {}", e)))?;

    debug!(user_id = user_id, "User created in transaction, now creating settings");

    // 4. Inserisci user_settings e ottieni l'id
    let settings_id: i64 = sqlx::query_scalar::<_, i64>(
        "INSERT INTO user_settings (user_id) VALUES (?) RETURNING id"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| DBError::new_registration_error(format!("Failed to insert user_settings: {}", e)))?;

    // 5. Inserisci passwords_generation_settings
    let config = preset.to_config();
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)"
    )
    .bind(settings_id)
    .bind(config.length)
    .bind(config.symbols)
    .bind(config.numbers)
    .bind(config.uppercase)
    .bind(config.lowercase)
    .execute(&mut *tx)
    .await
    .map_err(|e| DBError::new_registration_error(format!("Failed to insert gen_settings: {}", e)))?;

    // 6. Commit - solo se tutto è andato bene
    tx.commit().await
        .map_err(|e| DBError::new_transaction_error(format!("Failed to commit transaction: {}", e)))?;

    debug!(user_id = user_id, "Atomic registration completed successfully");
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
        .map_err(|e| {
            DBError::new_delete_error(format!("Failed to delete user: {}", e))
        })?;

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

/// recupera la lista degli utenti (senza avatar).
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
///
/// # Valòre Restituito
///
/// Return [`Vec<(i64, String, String, Option<Vec<u8>)>`] - Lista di utenti, ognuno come tupla (ID, username, created_at, avatar)
///
/// # Limiti
///
/// * Ultimi 10 utenti ordinati per ID decrescente
///
/// # Errori
///
/// - `DBError::new_list_error` - Errore nel recupero della lista
///
/// # Note
///
/// - Gli avatar vengono esclusi per ottimizzare le performance
#[instrument(skip(pool))]
pub async fn list_users(
    pool: &SqlitePool,
) -> Result<Vec<(i64, String, String, Option<Vec<u8>>)>, DBError> {
    debug!("Fetching list of users from database");
    let rows =
        query("SELECT id, username, created_at, avatar FROM users ORDER BY id DESC LIMIT 10")
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to list users: {}", e))
            })?;
    let users = rows.into_iter().map(|row| get_user_row(row)).collect();

    Ok(users)
}

/// Recupera la lista degli utenti senza avatar.
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
///
/// # Valore Restituito
///
/// Return [`Vec<(i64, String, String)>`] - Lista di utenti come tute (ID, username, created_at)
///
/// # Limiti
///
/// * Ultimi 10 utenti ordinati per ID decrescente
///
/// # Note
///
/// - Questa versione non recupera l'avatar per ottimizzare le performance
#[instrument(skip(pool))]
pub async fn list_users_no_avatar(
    pool: &SqlitePool,
) -> Result<Vec<(i64, String, String)>, DBError> {
    debug!("Fetching list of users from database");
    let rows = query("SELECT id, username, created_at FROM users ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to list users: {}", e)))?;
    let users = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<i64, _>("id"),
                row.get::<String, _>("username"),
                row.get::<String, _>("created_at"),
            )
        })
        .collect();

    Ok(users)
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
    let user_auth =
        sqlx::query_as::<_, UserAuth>("SELECT id, password, created_at FROM users WHERE id = ?")
            .bind(user_id) // SQLite preferisce i64 per gli ID
            .fetch_optional(pool) // Rimosso & perché pool è già un riferimento o clonabile
            .await
            .map_err(|e| DBError::new_select_error(e.to_string()))?; // Cattura l'errore reale del DB
    // Ora gestisci il caso in cui la query ha avuto successo ma non ha trovato righe
    user_auth.ok_or_else(|| DBError::new_select_error("User not found".into()))
}

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
pub async fn fetch_user_data(
    pool: &SqlitePool,
    username: &str,
) -> Result<(i64, String, String, Option<Vec<u8>>), DBError> {
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
        .map_err(|e| AuthError::DB(e))?;
    verify_password(password, hash.as_str()).map_err(|e| AuthError::Decryption(e))?;

    Ok(())
}

/// Salva o aggiorna una password nel database usando sqlx-template.
///
/// Utilizza il metodo generato `upsert_by_id` che gestisce sia INSERT che UPDATE:
/// - `id = None` → INSERT di una nuova password
/// - `id = Some(id)` → INSERT OR REPLACE (aggiorna la password esistente)
///
/// # Parametri
///
/// * `pool` - Pool SQLite per la connessione al database
/// * `stored_password` - Struct `StoredPassword` con i dati della password da salvare
///
/// # Valore Restituito
///
/// Return `Ok(())` se il salvataggio/aggiornamento ha successo
///
/// # Errori
///
/// - `DBError::new_password_save_error` - Password o location vuote
/// - `DBError::new_password_save_error` - Errore durante l'upsert
pub async fn save_or_update_stored_password(
    pool: &SqlitePool,
    stored_password: StoredPassword,
) -> Result<(), DBError> {
    debug!("Attempting to save/update user password");

    // Validazione comune
    if stored_password.password.expose_secret().is_empty()
        || stored_password.location.trim().is_empty()
    {
        return Err(DBError::new_password_save_error(
            "Password and location cannot be empty".into(),
        ));
    }

    // sqlx-template genera upsert_by_id() che gestisce entrambi i casi:
    // - Se id è None → INSERT
    // - Se id è Some(id) → INSERT OR REPLACE (aggiorna)
    StoredPassword::upsert_by_id(&stored_password, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Upsert failed: {}", e)))?;

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

#[cfg(test)]
mod tests {
    // Questo modulo può contenere test per gli helper functions stessi
    use super::*;
    use crate::backend::test_helpers::setup_test_db;

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
