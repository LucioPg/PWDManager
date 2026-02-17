//! Modulo contenente le query SQL di inizializzazione del database.
//!
//! Fornisce le query `CREATE TABLE IF NOT EXISTS` per creare le tabelle
//! necessarie all'avvio dell'applicazione se non esistono.
//!
//! # Tabelle
//!
//! - **users**: Tabella utenti con username, password (hash Argon2), avatar
//! - **passwords**: Tabella password criptate con AES-256-GCM, con foreign key verso users

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
/// 2. **passwords**:
///    - `id`: INTEGER PRIMARY KEY (auto-increment)
///    - `user_id`: INTEGER NOT NULL (rif. all'utente)
///    - `location`: TEXT NOT NULL (luogo/nome del servizio, es. "Google", "Netflix")
///    - `password`: BLOB NOT NULL (password criptata AES-256-GCM)
///    - `notes`: TEXT (note opzionali)
///    - `strength`: TEXT NOT NULL CHECK (forza password: weak/medium/strong)
///    - `created_at`: TEXT DEFAULT (datetime('now')) (timestamp creazione)
///    - `nonce`: BLOB NOT NULL UNIQUE (nonce AES-256-GCM a 12 byte, deve essere unico)
///    - `FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE` (cancella password se utente cancellato)
pub static QUERIES: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                temp_old_password TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                avatar BLOB
            );",
    "CREATE TABLE IF NOT EXISTS passwords (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                location TEXT NOT NULL,
                password BLOB NOT NULL,
                notes TEXT,
                score INTEGER NOT NULL CHECK (0 <= score <= 100),
                created_at TEXT DEFAULT (datetime('now')),
                nonce BLOB NOT NULL UNIQUE,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
    )",
];
