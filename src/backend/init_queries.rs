// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Modulo contenente le query SQL di inizializzazione del database.
//!
//! Fornisce le query `CREATE TABLE IF NOT EXISTS` per creare le tabelle
//! necessarie all'avvio dell'applicazione se non esistono.
//!
//! # Tabelle
//!
//! - **users**: Tabella utenti con username, password (hash Argon2), avatar
//! - **vaults**: Tabella vault per organizzare le password, con foreign key verso users
//! - **passwords**: Tabella password criptate con AES-256-GCM, con foreign key verso users e vaults
//! - **user_settings**: Tabella settings generali utente (relazione 1:1 con users)
//! - **passwords_generation_settings**: Tabella settings per generazione password
//!
//! # Migrazioni Schema
//!
//! Il modulo fornisce anche [`run_migrations`] per gestire l'evoluzione dello
//! schema nel tempo. Viene chiamato ad ogni avvio dopo [`run_init_queries`].
//!
//! ## Logica di rilevamento
//!
//! Usa `PRAGMA table_info(tabella)` per verificare se le colonne vault esistono.
//! Se mancano, esegue l'ALTER TABLE, crea un vault "Default" per ogni utente esistente,
//! e migra le password assegnandole al vault default.

use custom_errors::DBError;
use sqlx::{Row, SqlitePool, query};
use tracing;

/// Query SQL di inizializzazione del database.
///
/// Contiene le query `CREATE TABLE IF NOT EXISTS` per creare le tabelle
/// necessarie all'avvio dell'applicazione.
///
/// # Tabelle create
///
/// 1. **users**:
///    - `id`: INTEGER PRIMARY KEY (auto-increment)
///    - `username`: TEXT NOT NULL UNIQUE (nome utente unico)
///    - `password`: TEXT NOT NULL (password hash Argon2)
///    - `temp_old_password`: TEXT (per salvare temporaneamente la password vecchia durante aggiornamenti)
///    - `created_at`: TEXT DEFAULT (datetime('now')) (timestamp creazione)
///    - `avatar`: BLOB (immagine avatar come bytes)
///
/// 2. **vaults**:
///    - `id`: INTEGER PRIMARY KEY (auto-increment)
///    - `user_id`: INTEGER NOT NULL (rif. all'utente)
///    - `name`: TEXT NOT NULL (nome del vault)
///    - `description`: TEXT (descrizione opzionale)
///    - `created_at`: TEXT DEFAULT (datetime('now')) (timestamp creazione)
///    - `FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE` (cancella vault se utente cancellato)
///    - `UNIQUE(user_id, name)` (nome vault unico per utente)
///
/// 3. **passwords**:
///    - `id`: INTEGER PRIMARY KEY (auto-increment)
///    - `user_id`: INTEGER NOT NULL (rif. all'utente)
///    - `vault_id`: INTEGER NOT NULL (rif. al vault)
///    - `name`: TEXT NOT NULL (nome del servizio)
///    - `username`: BLOB NOT NULL (nome utente criptato AES-256-GCM)
///    - `username_nonce`: BLOB NOT NULL UNIQUE (nonce per username, 12 byte)
///    - `url`: BLOB NOT NULL (luogo/nome del servizio criptato AES-256-GCM)
///    - `url_nonce`: BLOB NOT NULL UNIQUE (nonce per url, 12 byte)
///    - `password`: BLOB NOT NULL (password criptata AES-256-GCM)
///    - `password_nonce`: BLOB NOT NULL UNIQUE (nonce per password, 12 byte)
///    - `notes`: BLOB (note opzionali criptate)
///    - `notes_nonce`: BLOB UNIQUE (nonce per notes, 12 byte, opzionale)
///    - `score`: INTEGER NOT NULL CHECK (0 <= score <= 100) (punteggio password 0-100)
///    - `created_at`: TEXT DEFAULT (datetime('now')) (timestamp creazione)
///    - `FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE` (cancella password se utente cancellato)
///    - `FOREIGN KEY(vault_id) REFERENCES vaults(id) ON DELETE CASCADE` (cancella password se vault cancellato)
pub static QUERIES: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                temp_old_password TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                avatar BLOB
            );",
    "CREATE TABLE IF NOT EXISTS vaults (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, name)
    )",
    "CREATE TABLE IF NOT EXISTS passwords (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                vault_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                username BLOB NOT NULL,
                username_nonce BLOB NOT NULL UNIQUE,
                url BLOB NOT NULL,
                url_nonce BLOB NOT NULL UNIQUE,
                password BLOB NOT NULL,
                password_nonce BLOB NOT NULL UNIQUE,
                notes BLOB,
                notes_nonce BLOB UNIQUE,
                score INTEGER NOT NULL CHECK (0 <= score <= 100),
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY(vault_id) REFERENCES vaults(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS user_settings (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL UNIQUE,
                theme TEXT NOT NULL DEFAULT 'Light',
                auto_update BOOLEAN NOT NULL DEFAULT 1,
                auto_logout_settings TEXT DEFAULT 'TenMinutes',
                active_vault_id INTEGER REFERENCES vaults(id),
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS passwords_generation_settings (
                id INTEGER PRIMARY KEY,
                settings_id INTEGER NOT NULL,
                length INTEGER NOT NULL CHECK (0 <= length <= 100),
                symbols INTEGER NOT NULL,
                numbers BOOLEAN NOT NULL,
                uppercase BOOLEAN NOT NULL,
                lowercase BOOLEAN NOT NULL,
                excluded_symbols TEXT,
                FOREIGN KEY(settings_id) REFERENCES user_settings(id) ON DELETE CASCADE
                CHECK (symbols <= length),
                UNIQUE (settings_id)
    )",
    "CREATE TABLE IF NOT EXISTS diceware_generation_settings (
                id INTEGER PRIMARY KEY,
                settings_id INTEGER NOT NULL,
                word_count INTEGER NOT NULL DEFAULT 6 CHECK (word_count >= 1 AND word_count <= 20),
                add_special_char BOOLEAN NOT NULL DEFAULT 0,
                numbers INTEGER NOT NULL DEFAULT 0 CHECK (numbers >= 0),
                language TEXT NOT NULL DEFAULT 'EN',
                FOREIGN KEY(settings_id) REFERENCES user_settings(id) ON DELETE CASCADE,
                CHECK (language IN ('EN', 'IT', 'FR')),
                UNIQUE (settings_id)
    )",
];

/// Checks if a column exists in a table.
async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool, DBError> {
    let sql = format!("PRAGMA table_info('{}')", table);
    let rows = query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| DBError::new_general_error(format!("Failed to check schema: {}", e)))?;

    Ok(rows.iter().any(|row| {
        row.try_get::<&str, _>("name")
            .map(|name| name == column)
            .unwrap_or(false)
    }))
}

/// Runs database migrations for schema evolution.
///
/// Called after `run_init_queries` on every startup.
/// Checks if vault-related columns exist and adds them if missing,
/// creating a default vault for each existing user and migrating
/// existing passwords to that vault.
///
/// This function is idempotent: running it multiple times has no effect
/// if the schema is already up to date.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DBError> {
    // Check if passwords table has vault_id column
    let vault_id_exists = column_exists(pool, "passwords", "vault_id").await?;

    if !vault_id_exists {
        tracing::info!("Migration: adding vault_id column to passwords table");

        // Step 1: Add vault_id column with default 0
        // We use DEFAULT 0 instead of NOT NULL because SQLite ALTER TABLE
        // doesn't support NOT NULL without a default on existing tables.
        // The value 0 is a placeholder; we'll update all rows to valid vault IDs.
        query("ALTER TABLE passwords ADD COLUMN vault_id INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| {
                DBError::new_general_error(format!("Failed to add vault_id column: {}", e))
            })?;

        // Step 2: Ensure vaults table exists (redundant with QUERIES, but safe)
        query(
            "CREATE TABLE IF NOT EXISTS vaults (
                    id INTEGER PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    description TEXT,
                    created_at TEXT DEFAULT (datetime('now')),
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                    UNIQUE(user_id, name)
                )",
        )
        .execute(pool)
        .await
        .map_err(|e| DBError::new_general_error(format!("Failed to create vaults table: {}", e)))?;

        // Step 3: Create a default vault for each user
        let user_ids: Vec<i64> = query("SELECT id FROM users")
            .fetch_all(pool)
            .await
            .map_err(|e| DBError::new_general_error(format!("Failed to fetch users: {}", e)))?
            .iter()
            .map(|row| row.get::<i64, _>("id"))
            .collect();

        for user_id in &user_ids {
            query("INSERT OR IGNORE INTO vaults (user_id, name) VALUES (?, 'Default')")
                .bind(*user_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    DBError::new_general_error(format!(
                        "Failed to create default vault for user {}: {}",
                        user_id, e
                    ))
                })?;
        }

        // Step 4: Migrate all passwords to their user's default vault
        query(
            "UPDATE passwords SET vault_id = (
                    SELECT v.id FROM vaults v
                    WHERE v.user_id = passwords.user_id
                    AND v.name = 'Default'
                )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_general_error(format!("Failed to migrate passwords to vaults: {}", e))
        })?;

        tracing::info!(
            "Migration: migrated {} passwords across {} users to default vaults",
            user_ids.len(),
            user_ids.len()
        );
    }

    // Check if user_settings table has active_vault_id column
    let active_vault_exists = column_exists(pool, "user_settings", "active_vault_id").await?;

    if !active_vault_exists {
        tracing::info!("Migration: adding active_vault_id column to user_settings table");

        query("ALTER TABLE user_settings ADD COLUMN active_vault_id INTEGER REFERENCES vaults(id)")
            .execute(pool)
            .await
            .map_err(|e| {
                DBError::new_general_error(format!("Failed to add active_vault_id column: {}", e))
            })?;

        // Set active_vault_id to each user's default vault
        query(
            "UPDATE user_settings SET active_vault_id = (
                    SELECT v.id FROM vaults v
                    WHERE v.user_id = user_settings.user_id
                    AND v.name = 'Default'
                )",
        )
        .execute(pool)
        .await
        .map_err(|e| DBError::new_general_error(format!("Failed to set active_vault_id: {}", e)))?;

        tracing::info!("Migration: set active_vault_id for all users");
    }

    Ok(())
}
