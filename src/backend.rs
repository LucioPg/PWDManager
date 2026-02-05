
use custom_errors::DBError;
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
use tracing::{info, error, debug, instrument};

#[cfg(feature = "desktop")]
thread_local! {
    pub static DB: rusqlite::Connection = {
        info!("Initializing database connection");
        // Open the database from the persisted "database.db" file
        let conn = rusqlite::Connection::open("database.db").unwrap_or_else(|e| {
            error!(
                error = %e,
                db_file = "database.db",
                "Failed to open database connection"
            );
            panic!("Database connection failed: {}", e);
        });

        // Create the "user_ids" table if it doesn't already exist
        if let Err(e) = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        ) {
            error!(
                error = %e,
                "Failed to create database table"
            );
            panic!("Database table creation failed: {}", e);
        } else {
            info!("Database table initialized successfully");
        }

        // Return the connection
        conn
    };
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


pub async fn save_user(username: String, password: String) -> dioxus::Result<()> {
    debug!("Attempting to save user credentials to database");

    DB.with(|c| {
        let result = c.execute("INSERT INTO users (username, password) VALUES (?1, ?2)", &[&username, &password]);
        match result {
            Ok(rows_affected) => {
                debug!(
                    rows_affected = rows_affected,
                    username = %username,
                    "Successfully saved user's credentials to database"
                );
                Ok(rows_affected)
            }
            Err(e) => {
                error!(
                    error = %e,
                    username = %username,
                    password = %password,
                    "Failed to insert user's credentials into database"
                );
                Err(e)
            }
        }
    })?;

    Ok(())
}


#[instrument(fields(user_id = id))]
pub async fn delete_user(id: i32) -> Result<i32, DBError> {
    debug!(
        user_id = id,
        "Attempting to delete user from database"
    );

    let result = DB.with(|c| {
        match c.prepare("DELETE FROM users WHERE id = ?1") {
            Ok(mut stmt) => {
                match stmt.execute(&[&id]) {
                    Ok(rows_affected) => {
                        if rows_affected == 0 {
                            error!(
                                user_id = id,
                                "No user found with specified ID"
                            );
                        }
                        Ok(rows_affected)
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            user_id = id,
                            "Failed to execute DELETE statement"
                        );
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    user_id = id,
                    "Failed to prepare DELETE statement"
                );
                Err(e)
            }
        }
    });

    match result {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                debug!(
                    user_id = id,
                    rows_affected = rows_affected,
                    "Successfully deleted user from database"
                );
            }
            Ok(id)
        }
        Err(e) => {
            Err(DBError::new_general_error(e.to_string()))
        }
    }
}

#[instrument]
pub async fn list_users() -> Result<Vec<(i32, String)>, DBError> {
    debug!("Fetching list of users from database");

    let users = DB.with(|c| {
        let mut stmt = match c.prepare("SELECT id, url FROM users ORDER BY id DESC LIMIT 10") {
            Ok(stmt) => stmt,
            Err(e) => {
                error!(
                    error = %e,
                    "Failed to prepare SELECT statement"
                );
                return Err(DBError::new_select_error(e.to_string()));
            }
        };

        let rows = match stmt.query_map([], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        }) {
            Ok(rows) => rows,
            Err(e) => {
                error!(
                    error = %e,
                    "Failed to query users from database"
                );
                return Err(DBError::new_list_error(e.to_string()));
            }
        };

        match rows.collect::<Result<Vec<_>, _>>() {
            Ok(users) => {
                debug!(
                    count = users.len(),
                    "Successfully retrieved registered users from database"
                );
                Ok(users)
            }
            Err(e) => {
                error!(
                    error = %e,
                    "Failed to collect users from query results"
                );
                Err(DBError::new_list_error(e.to_string()))
            }
        }
    })?;

    Ok(users)
}

