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
                auto_update BOOLEAN NOT NULL DEFAULT 0,
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
