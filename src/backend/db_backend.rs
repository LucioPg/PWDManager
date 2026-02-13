#![allow(dead_code)]
use crate::backend::init_queries::QUERIES;
use crate::backend::user_auth_helper::{StoredPassword, UserAuth};
use crate::backend::utils::verify_password;
use custom_errors::{AuthError, DBError};
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::query::Query;
use sqlx::sqlite::{
    SqliteArguments, SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteRow,
};
use sqlx::{Row, Sqlite, query};
use std::str::FromStr;
#[cfg(feature = "desktop")]
use tracing::{debug, instrument};

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

/// Prepara l'aggiornamento utente recuperando la vecchia password se necessario
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
            // Prima recupera la vecchia password hash usando user_id e salvala in temp_old_password
            if let Ok(user_auth) = fetch_password_created_at_from_id(pool, user_id).await {
                set_temp_password(pool, user_id, &user_auth.password.0).await?;
            }

            let hash_password = crate::backend::utils::encrypt(psw.clone())
                .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
            update.password = Some(SecretString::new(hash_password.into()));
        }
    }

    Ok(update)
}

pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i64>, // Se Some, fa l'UPDATE. Se None, fa l'INSERT.
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<(), DBError> {
    debug!("Attempting to save/update user credentials");

    // 1. Criptazione comune a entrambi i casi

    match id {
        // --- CASO UPDATE ---
        Some(user_id) => {
            let update = prepare_user_update(pool, user_id, username, password, avatar).await?;

            if !update.has_updates() {
                return Ok(());
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
        }
        // --- CASO INSERT ---
        None => {
            let psw = password.unwrap_or_default();
            if !psw.expose_secret().trim().is_empty() {
                let hash_password = crate::backend::utils::encrypt(psw)
                    .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
                sqlx::query("INSERT INTO users (username, password, avatar) VALUES (?, ?, ?)")
                    .bind(username)
                    .bind(hash_password)
                    .bind(avatar)
                    .execute(pool)
                    .await
                    .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;
            } else {
                return Err(DBError::new_save_error("Password cannot be empty".into()));
            }
        }
    }

    Ok(())
}

#[instrument(fields(user_id = id))]
pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<(), DBError> {
    debug!(user_id = id, "Attempting to delete user from database");
    let _ = query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_delete_error(format!("Failed to save user credentials: {}", e))
        })?;

    Ok(())
}

fn get_user_row(row: SqliteRow) -> (i64, String, String, Option<Vec<u8>>) {
    (
        row.get::<i64, _>("id"),
        row.get::<String, _>("username"),
        row.get::<String, _>("created_at"),
        row.get::<Option<Vec<u8>>, _>("avatar"),
    )
}

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
                DBError::new_list_error(format!("Failed to save user credentials: {}", e))
            })?;
    let users = rows.into_iter().map(|row| get_user_row(row)).collect();

    Ok(users)
}

#[instrument(skip(pool))]
pub async fn list_users_no_avatar(
    pool: &SqlitePool,
) -> Result<Vec<(i64, String, String)>, DBError> {
    debug!("Fetching list of users from database");
    let rows = query("SELECT id, username, created_at FROM users ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to save user credentials: {}", e)))?;
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

#[instrument(skip(pool))]
pub async fn fetch_password_created_at_from_id(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<UserAuth, DBError> {
    debug!("Fetching user credentials in database");

    let user_auth =
        sqlx::query_as::<_, UserAuth>("SELECT password, created_at FROM users WHERE id = ?")
            .bind(user_id) // SQLite preferisce i64 per gli ID
            .fetch_optional(pool) // Rimosso & perché pool è già un riferimento o clonabile
            .await
            .map_err(|e| DBError::new_select_error(e.to_string()))?; // Cattura l'errore reale del DB

    // Ora gestisci il caso in cui la query ha avuto successo ma non ha trovato righe
    user_auth.ok_or_else(|| DBError::new_select_error("User not found".into()))
}

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

#[instrument(skip(pool))]
pub async fn get_all_passwords_for_user(
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
