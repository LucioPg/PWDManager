#![allow(dead_code)]
use custom_errors::DBError;
use sqlx::sqlite::{ SqliteConnectOptions, SqlitePool};
use sqlx::{query, Row};
use std::str::FromStr;
use dioxus::prelude::*;
#[cfg(feature = "desktop")]
use tracing::{debug, instrument};

#[cfg(feature = "desktop")]
pub async fn init_db() -> Result<SqlitePool, DBError> {
    let options = SqliteConnectOptions::from_str("sqlite:database.db")
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await.map_err(|e| DBError::new_general_error(e.to_string()))?;

    query(
        "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                logged BOOLEAN NOT NULL DEFAULT FALSE
            );",
    )
    .execute(&pool)
    .await
    .map_err(|e| DBError::new_general_error(format!("Failed to create table: {}", e)))?;
    Ok(pool)
}

// #[post("/api/save_user_id")]
// #[server]
// pub async fn save_user_id(image: String) -> dioxus::Result<()> {
// use std::io::Write;
//
// Open the "user_ids.txt" file in append mode, creating it if it doesn't exist yet
// let mut file = std::fs::OpenOptions::new()
//     .write(true)
//     .append(true)
//     .create(true)
//     .open("user_ids.txt")
//     .unwrap();
// file.write_fmt(format_args!("{}\n", image)).unwrap();
// Ok(())

// db approach


pub async fn save_user(pool: &SqlitePool, username: String, password: String) -> dioxus::Result<()> {
    debug!("Attempting to save user credentials to database");

    let _ = query("INSERT INTO users (username, password) VALUES (?, ?)")
        .bind(username)
        .bind(password)
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

#[instrument]
pub async fn list_users(pool: &SqlitePool) -> Result<Vec<(i32, String, bool, String)>, DBError> {
    debug!("Fetching list of users from database");
    let rows = query("SELECT id, username, logged, created_at FROM users ORDER BY id DESC LIMIT 10")
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to save user credentials: {}", e)))?;

    let users = rows
        .into_iter()
        .map(|row| (
            row.get::<i32, _>("id"),
            row.get::<String, _>("username"),
            row.get::<bool, _>("logged"),
            row.get::<String, _>("created_at")
            ))
        .collect();

    Ok(users)


}

