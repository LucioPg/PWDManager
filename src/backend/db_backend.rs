#![allow(dead_code)]
use custom_errors::{AuthError, DBError};
use sqlx::sqlite::{ SqliteConnectOptions, SqlitePool};
use sqlx::{query, Row};
use std::str::FromStr;
use dioxus::prelude::*;
#[cfg(feature = "desktop")]
use tracing::{debug, instrument};
use crate::backend::utils::verify_password;

#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<SqlitePool, DBError> {
    let options = SqliteConnectOptions::from_str("sqlite:database.db")
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await.map_err(|e| DBError::new_general_error(e.to_string()))?;

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

pub async fn save_user(pool: &SqlitePool, username: String, password: String, avatar: Option<Vec<u8>>) -> dioxus::Result<()> {
    debug!("Attempting to save user credentials to database");
    let hash_password = crate::backend::utils::encrypt(&password)
        .map_err(|e| DBError::new_save_error(format!("Failed to encrypt password: {}", e)))?;
    let _ = query("INSERT INTO users (username, password, avatar) VALUES (?, ?, ?)")
        .bind(username)
        .bind(hash_password)
        .bind(avatar)
        .execute(pool)
        .await
        .map_err(|e| DBError::new_save_error(format!("Failed to save user credentials: {}", e)))?;

    Ok(())
}


#[instrument(fields(user_id = id))]
pub async fn delete_user(pool: &SqlitePool, id: i32) -> Result<(), DBError> {
    debug!(
        user_id = id,
        "Attempting to delete user from database"
    );
    let _ = query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| DBError::new_delete_error(format!("Failed to save user credentials: {}", e)))?;

    Ok(())

}

#[instrument(skip(pool))]
pub async fn list_users(pool: &SqlitePool) -> Result<Vec<(i32, String, String, Option<Vec<u8>>)>, DBError> {
    debug!("Fetching list of users from database");
    let rows = query("SELECT id, username, created_at FROM users ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to save user credentials: {}", e)))?;
    let users = rows
        .into_iter()
        .map(|row| (
            row.get::<i32, _>("id"),
            row.get::<String, _>("username"),
            row.get::<String, _>("created_at"),
            row.get::<Option<Vec<u8>>, _>("avatar")
            ))
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
        Err(e) => Err(DBError::new_fetch_error(format!("Failed to fetch user credentials: {}", e)))
    }
}

#[instrument(skip(pool))]
pub async fn fetch_user_data(pool: &SqlitePool, username: &str) -> Result<(i32, String, String, Option<Vec<u8>>), DBError> {
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
            row.get::<Option<Vec<u8>>, _>("avatar")
            )),
        Ok(None) => Err(DBError::new_select_error("User not found".into())),
        Err(e) => Err(DBError::new_fetch_error(format!("Failed to fetch user data: {}", e)))
    }
}

#[instrument(skip(pool))]
pub async fn check_user(pool: &SqlitePool, username: &str, password: &str) -> Result<(), AuthError> {
    debug!("Checking user credentials in database");
    let hash = fetch_user_password(pool, username).await.map_err(|e| AuthError::DB(e))?;
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