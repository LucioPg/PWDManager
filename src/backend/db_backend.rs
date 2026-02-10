#![allow(dead_code)]
use crate::backend::utils::verify_password;
use custom_errors::{AuthError, DBError};
use dioxus::prelude::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{Row, query};
use std::str::FromStr;
#[cfg(feature = "desktop")]
use tracing::{debug, instrument};

#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<SqlitePool, DBError> {
    let options = SqliteConnectOptions::from_str("sqlite:database.db")
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| DBError::new_general_error(e.to_string()))?;

    query(
        "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                avatar BLOB
            );",
    )
    .execute(&pool)
    .await
    .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    Ok(pool)
}

pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i32>, // Se Some, fa l'UPDATE. Se None, fa l'INSERT.
    username: String,
    password: Option<String>,
    avatar: Option<Vec<u8>>,
) -> Result<(), DBError> {
    debug!("Attempting to save/update user credentials");

    // 1. Criptazione comune a entrambi i casi

    match id {
        // --- CASO UPDATE ---
        Some(user_id) => {
            match password {
                Some(psw) if !psw.is_empty() => {
                    let hash_password = crate::backend::utils::encrypt(&psw)
                        .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
                    sqlx::query("UPDATE users SET username = ?, password = ?, avatar = ? WHERE id = ?")
                        .bind(username)
                        .bind(hash_password)
                        .bind(avatar)
                        .bind(user_id)
                        .execute(pool)
                        .await
                        .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;
                }
                _ => {
                    sqlx::query("UPDATE users SET username = ?, avatar = ? WHERE id = ?")
                        .bind(username)
                        .bind(avatar)
                        .bind(user_id)
                        .execute(pool)
                        .await
                        .map_err(|e| DBError::new_save_error(format!("Update failed: {}", e)))?;
                }
            }
        }
        // --- CASO INSERT ---
        None => {
            let psw = password.unwrap_or_default();
            let hash_password = crate::backend::utils::encrypt(&psw)
                .map_err(|e| DBError::new_save_error(format!("Failed to encrypt: {}", e)))?;
            sqlx::query("INSERT INTO users (username, password, avatar) VALUES (?, ?, ?)")
                .bind(username)
                .bind(hash_password)
                .bind(avatar)
                .execute(pool)
                .await
                .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;
        }
    }

    Ok(())
}

#[instrument(fields(user_id = id))]
pub async fn delete_user(pool: &SqlitePool, id: i32) -> Result<(), DBError> {
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

#[instrument(skip(pool))]
pub async fn list_users(
    pool: &SqlitePool,
) -> Result<Vec<(i32, String, String, Option<Vec<u8>>)>, DBError> {
    debug!("Fetching list of users from database");
    let rows = query("SELECT id, username, created_at FROM users ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to save user credentials: {}", e)))?;
    let users = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<i32, _>("id"),
                row.get::<String, _>("username"),
                row.get::<String, _>("created_at"),
                row.get::<Option<Vec<u8>>, _>("avatar"),
            )
        })
        .collect();

    Ok(users)
}
#[instrument(skip(pool))]
async fn fetch_user_password(pool: &SqlitePool, username: &str) -> Result<String, DBError> {
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
pub async fn fetch_user_data(
    pool: &SqlitePool,
    username: &str,
) -> Result<(i32, String, String, Option<Vec<u8>>), DBError> {
    debug!("Fetching user credentials in database");
    let row = query("SELECT id, username, created_at, avatar FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await;
    match row {
        Ok(Some(row)) => Ok((
            row.get::<i32, _>("id"),
            row.get::<String, _>("username"),
            row.get::<String, _>("created_at"),
            row.get::<Option<Vec<u8>>, _>("avatar"),
        )),
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
    password: &str,
) -> Result<(), AuthError> {
    debug!("Checking user credentials in database");
    let hash = fetch_user_password(pool, username)
        .await
        .map_err(|e| AuthError::DB(e))?;
    verify_password(password, hash.as_str()).map_err(|e| AuthError::Decryption(e))?;

    Ok(())
}

// #[instrument]
// pub async fn login_user(pool: &SqlitePool, user_id: i32) -> Result<bool, AuthError> {
//     let _ = query("UPDATE users SET logged = TRUE WHERE id = ?")
//         .bind(user_id)
//         .execute(pool).await.map_err(|_| AuthGeneralError::LoginError).map_err(|e| AuthError::AuthenticationError);
//     Ok(true)
// }
//
// #[instrument]
// pub async fn logout_user(pool: &SqlitePool, user_id: i32) -> Result<bool, AuthError> {
//     let _ = query("UPDATE users SET logged = FALSE WHERE id = ?")
//         .bind(user_id)
//         .execute(pool).await.map_err(|_| AuthGeneralError::LogoutError).map_err(|e| AuthError::AuthenticationError);
//     Ok(true)
// }
