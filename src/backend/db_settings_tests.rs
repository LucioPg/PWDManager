//! Test per la creazione dei settings utente.
//!
//! Questo modulo contiene test per:
//! - `PasswordPreset::to_config()` - verifica valori per ogni preset
//! - `create_user_settings()` - creazione settings nel database
//! - `register_user_with_settings()` - registrazione atomica con singola transazione DB
//!
//! # Atomicità
//!
//! La funzione `register_user_with_settings()` usa una **singola transazione DB**
//! con pattern RAII. Se qualsiasi operazione fallisce, il DB fa automaticamente
//! rollback senza bisogno di compensazione manuale.
//!
//! I test verificano l'atomicità usando vincoli "naturali" del DB:
//! - FOREIGN KEY: utente inesistente
//! - UNIQUE: username duplicato, settings duplicati

#![allow(dead_code)]

use crate::backend::db_backend::{create_user_settings, register_user_with_settings};
use crate::backend::settings_types::PasswordPreset;
use crate::backend::test_helpers::{create_test_user, setup_test_db};
use secrecy::SecretString;
use sqlx::sqlite::SqliteRow;
use sqlx::{query, Row};
use std::str::FromStr;

#[cfg(test)]
mod tests {

    use super::*;

    // ============ Categoria 1: Test PasswordPreset::to_config() ============

    #[test]
    fn test_preset_medium_config_values() {
        let config = PasswordPreset::Medium.to_config();

        assert_eq!(config.length, 8, "Medium preset: length should be 8");
        assert_eq!(config.symbols, 2, "Medium preset: symbols should be 2");
        assert!(config.numbers, "Medium preset: numbers should be true");
        assert!(config.uppercase, "Medium preset: uppercase should be true");
        assert!(config.lowercase, "Medium preset: lowercase should be true");
    }

    #[test]
    fn test_preset_strong_config_values() {
        let config = PasswordPreset::Strong.to_config();

        assert_eq!(config.length, 12, "Strong preset: length should be 12");
        assert_eq!(config.symbols, 2, "Strong preset: symbols should be 2");
        assert!(config.numbers, "Strong preset: numbers should be true");
        assert!(config.uppercase, "Strong preset: uppercase should be true");
        assert!(config.lowercase, "Strong preset: lowercase should be true");
    }

    #[test]
    fn test_preset_epic_config_values() {
        let config = PasswordPreset::Epic.to_config();

        assert_eq!(config.length, 16, "Epic preset: length should be 16");
        assert_eq!(config.symbols, 2, "Epic preset: symbols should be 2");
        assert!(config.numbers, "Epic preset: numbers should be true");
        assert!(config.uppercase, "Epic preset: uppercase should be true");
        assert!(config.lowercase, "Epic preset: lowercase should be true");
    }

    #[test]
    fn test_preset_god_config_values() {
        let config = PasswordPreset::God.to_config();

        assert_eq!(config.length, 26, "God preset: length should be 26");
        assert_eq!(config.symbols, 2, "God preset: symbols should be 2");
        assert!(config.numbers, "God preset: numbers should be true");
        assert!(config.uppercase, "God preset: uppercase should be true");
        assert!(config.lowercase, "God preset: lowercase should be true");
    }

    #[test]
    fn test_all_presets_have_valid_symbols_ratio() {
        // Verifica che symbols <= length per tutti i preset
        for preset in [
            PasswordPreset::Medium,
            PasswordPreset::Strong,
            PasswordPreset::Epic,
            PasswordPreset::God,
        ] {
            let config = preset.to_config();
            assert!(
                config.symbols <= config.length,
                "{:?}: symbols ({}) should be <= length ({})",
                preset,
                config.symbols,
                config.length
            );
        }
    }

    // ============ Categoria 2: Test create_user_settings() - Successo ============

    #[tokio::test]
    async fn test_create_user_settings_medium_preset() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_medium_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        let result = create_user_settings(&pool, user_id, PasswordPreset::Medium).await;

        assert!(result.is_ok(), "create_user_settings should succeed");

        // Verifica che user_settings sia stato creato
        let user_settings_row: Option<SqliteRow> = query(
            "SELECT id, user_id FROM user_settings WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query user_settings");

        assert!(user_settings_row.is_some(), "user_settings should exist");
        let row = user_settings_row.unwrap();
        assert_eq!(row.get::<i64, _>("user_id"), user_id);

        // Verifica che passwords_generation_settings sia stato creato con i valori corretti
        let settings_id: i64 = row.get("id");
        let gen_settings_row: SqliteRow = query(
            "SELECT length, symbols, numbers, uppercase, lowercase, excluded_symbols
             FROM passwords_generation_settings WHERE settings_id = ?"
        )
        .bind(settings_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query passwords_generation_settings");

        assert_eq!(gen_settings_row.get::<i64, _>("length"), 8);
        assert_eq!(gen_settings_row.get::<i64, _>("symbols"), 2);
        assert!(gen_settings_row.get::<bool, _>("numbers"));
        assert!(gen_settings_row.get::<bool, _>("uppercase"));
        assert!(gen_settings_row.get::<bool, _>("lowercase"));
        assert!(gen_settings_row.get::<Option<String>, _>("excluded_symbols").is_none());
    }

    #[tokio::test]
    async fn test_create_user_settings_strong_preset() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_strong_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        let result = create_user_settings(&pool, user_id, PasswordPreset::Strong).await;

        assert!(result.is_ok(), "create_user_settings should succeed");

        // Verifica valori Strong preset
        let gen_settings_row: SqliteRow = query(
            "SELECT pgs.length, pgs.symbols, pgs.numbers, pgs.uppercase, pgs.lowercase
             FROM passwords_generation_settings pgs
             JOIN user_settings us ON pgs.settings_id = us.id
             WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query settings");

        assert_eq!(gen_settings_row.get::<i64, _>("length"), 12);
        assert_eq!(gen_settings_row.get::<i64, _>("symbols"), 2);
    }

    #[tokio::test]
    async fn test_create_user_settings_epic_preset() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_epic_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        let result = create_user_settings(&pool, user_id, PasswordPreset::Epic).await;

        assert!(result.is_ok(), "create_user_settings should succeed");

        // Verifica valori Epic preset
        let gen_settings_row: SqliteRow = query(
            "SELECT pgs.length, pgs.symbols, pgs.numbers, pgs.uppercase, pgs.lowercase
             FROM passwords_generation_settings pgs
             JOIN user_settings us ON pgs.settings_id = us.id
             WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query settings");

        assert_eq!(gen_settings_row.get::<i64, _>("length"), 16);
        assert_eq!(gen_settings_row.get::<i64, _>("symbols"), 2);
    }

    #[tokio::test]
    async fn test_create_user_settings_god_preset() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_god_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        let result = create_user_settings(&pool, user_id, PasswordPreset::God).await;

        assert!(result.is_ok(), "create_user_settings should succeed");

        // Verifica valori God preset
        let gen_settings_row: SqliteRow = query(
            "SELECT pgs.length, pgs.symbols, pgs.numbers, pgs.uppercase, pgs.lowercase
             FROM passwords_generation_settings pgs
             JOIN user_settings us ON pgs.settings_id = us.id
             WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query settings");

        assert_eq!(gen_settings_row.get::<i64, _>("length"), 26);
        assert_eq!(gen_settings_row.get::<i64, _>("symbols"), 2);
    }

    // ============ Categoria 3: Test create_user_settings() - Errori ============

    #[tokio::test]
    async fn test_create_user_settings_nonexistent_user() {
        let pool = setup_test_db().await;
        let fake_user_id = 99999i64; // ID che non esiste

        let result = create_user_settings(&pool, fake_user_id, PasswordPreset::Medium).await;

        // Deve fallire per foreign key constraint
        assert!(
            result.is_err(),
            "create_user_settings should fail with nonexistent user_id"
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to insert user_settings") || error_msg.contains("FOREIGN KEY"),
            "Error should mention foreign key or insert failure, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_create_user_settings_duplicate_user() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_dup_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        // Prima creazione - deve riuscire
        let result1 = create_user_settings(&pool, user_id, PasswordPreset::Medium).await;
        assert!(result1.is_ok(), "First create_user_settings should succeed");

        // Seconda creazione - deve fallire per UNIQUE constraint su user_id
        let result2 = create_user_settings(&pool, user_id, PasswordPreset::Strong).await;
        assert!(
            result2.is_err(),
            "Second create_user_settings should fail due to UNIQUE constraint"
        );

        let error_msg = result2.unwrap_err().to_string();
        assert!(
            error_msg.contains("Failed to insert user_settings") || error_msg.contains("UNIQUE"),
            "Error should mention unique constraint or insert failure, got: {}",
            error_msg
        );
    }

    // ============ Categoria 4: Test Integrità Referenziale ============

    #[tokio::test]
    async fn test_cascade_delete_settings_on_user_delete() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_cascade_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        // Crea i settings
        let result = create_user_settings(&pool, user_id, PasswordPreset::Medium).await;
        assert!(result.is_ok(), "create_user_settings should succeed");

        // Ottieni settings_id prima di cancellare
        let settings_id: Option<i64> = query("SELECT id FROM user_settings WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to query settings_id")
            .map(|row| row.get("id"));

        assert!(settings_id.is_some(), "Settings should exist before delete");

        // Cancella l'utente
        let _ = query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("Failed to delete user");

        // Verifica che i settings siano stati cancellati in cascade
        let user_settings: Option<SqliteRow> = query("SELECT id FROM user_settings WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to query user_settings after delete");

        assert!(
            user_settings.is_none(),
            "user_settings should be deleted after user deletion (CASCADE)"
        );

        // Verifica che anche passwords_generation_settings sia stato cancellato
        let gen_settings: Option<SqliteRow> = query(
            "SELECT id FROM passwords_generation_settings WHERE settings_id = ?"
        )
        .bind(settings_id.unwrap())
        .fetch_optional(&pool)
        .await
        .expect("Failed to query gen_settings after delete");

        assert!(
            gen_settings.is_none(),
            "passwords_generation_settings should be deleted after user deletion (CASCADE)"
        );
    }

    // ============ Categoria 5: Test Relazione 1:1 ============

    #[tokio::test]
    async fn test_one_to_one_relationship() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_user_relation_{}", timestamp);

        let (user_id, _) = create_test_user(&pool, &username, "password123", None).await;

        // Crea i settings
        let _ = create_user_settings(&pool, user_id, PasswordPreset::Medium).await;

        // Verifica che esista esattamente UN record per questo utente
        let count: i64 = query("SELECT COUNT(*) as count FROM user_settings WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to count user_settings")
            .get("count");

        assert_eq!(count, 1, "Should have exactly one user_settings record per user");

        // Verifica che esista esattamente UN record in passwords_generation_settings
        let gen_count: i64 = query(
            "SELECT COUNT(*) as count FROM passwords_generation_settings pgs
             JOIN user_settings us ON pgs.settings_id = us.id
             WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count gen_settings")
        .get("count");

        assert_eq!(gen_count, 1, "Should have exactly one passwords_generation_settings record per user");
    }

    // ============ Categoria 6: Test Transaction Error (Pool Closed) ============

    /// Test che verifica la gestione degli errori di transazione.
    ///
    /// Simula un errore di connessione chiudendo il pool prima di chiamare
    /// create_user_settings(). Questo testa il path di errore di `begin()`
    /// verificando che la funzione gestisca gli errori di connessione senza panic.
    #[tokio::test]
    async fn test_create_user_settings_handles_pool_errors_gracefully() {
        // Crea un pool temporaneo separato (non il singleton)
        let db_path = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| ".".to_string());
        let test_dir = std::path::PathBuf::from(db_path).join("test_dbs");
        if !test_dir.exists() {
            std::fs::create_dir_all(&test_dir).expect("Failed to create test_dbs");
        }

        let db_file = test_dir.join("test_pool_closed.db");
        let db_path_str = format!("sqlite:{}", db_file.to_str().unwrap());

        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&db_path_str)
            .expect("Invalid DB path")
            .foreign_keys(true)
            .create_if_missing(true);

        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("Failed to create pool");

        // Inizializza le tabelle
        for init_query in crate::backend::init_queries::QUERIES {
            query(init_query)
                .execute(&pool)
                .await
                .expect("Failed to create table");
        }

        // Crea un utente
        let (user_id, _) = create_test_user(&pool, "test_pool_closed", "password123", None).await;

        // Chiudi il pool - questo simula un errore di connessione
        pool.close().await;

        // Tenta di creare i settings con il pool chiuso
        let result = create_user_settings(&pool, user_id, PasswordPreset::Medium).await;

        // La funzione DEVE restituire errore (non panic)
        assert!(
            result.is_err(),
            "create_user_settings should return error when pool is closed, not panic"
        );

        // Verifica che l'errore sia quello atteso
        let error_msg = result.unwrap_err().to_string();
        // sqlx può restituire diversi tipi di errore per pool chiuso
        assert!(
            error_msg.contains("Failed to begin transaction")
                || error_msg.contains("closed")
                || error_msg.contains("pool")
                || error_msg.contains("connection"),
            "Error should mention transaction or connection issue, got: {}",
            error_msg
        );

        // Cleanup: rimuovi il file del database di test
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(db_file.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_file.with_extension("db-shm"));
    }

    // ============ Categoria 7: Test Atomicità Registrazione ============

    /// Test che verifica l'atomicità della registrazione con username duplicato.
    ///
    /// Se proviamo a registrare un utente con username già esistente,
    /// la transazione fallisce e NESSUN nuovo record viene creato.
    #[tokio::test]
    async fn test_atomic_registration_duplicate_username() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_dup_user_{}", timestamp);

        // Prima registrazione - deve riuscire
        let result1 = register_user_with_settings(
            &pool,
            username.clone(),
            Some(SecretString::new("password123".into())),
            None,
            PasswordPreset::Medium,
        )
        .await;

        assert!(result1.is_ok(), "First registration should succeed");
        let user_id_1 = result1.unwrap();

        // Seconda registrazione con stesso username - deve fallire
        let result2 = register_user_with_settings(
            &pool,
            username.clone(),
            Some(SecretString::new("password456".into())),
            None,
            PasswordPreset::God,
        )
        .await;

        assert!(
            result2.is_err(),
            "Second registration with duplicate username should fail"
        );

        // Verifica che esista SOLO il primo utente
        let users: Vec<SqliteRow> = query("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_all(&pool)
            .await
            .expect("Failed to query users");

        assert_eq!(users.len(), 1, "Should have exactly one user with this username");
        assert_eq!(users[0].get::<i64, _>("id"), user_id_1, "Should be the first user");
    }

    /// Test che verifica l'atomicità della registrazione con password vuota.
    ///
    /// Se la password è vuota, la registrazione deve fallire e NESSUN
    /// record deve essere creato (né utente né settings).
    #[tokio::test]
    async fn test_atomic_registration_empty_password() {
        let pool = setup_test_db().await;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_empty_pwd_{}", timestamp);

        // Tentativo con password vuota
        let result = register_user_with_settings(
            &pool,
            username.clone(),
            Some(SecretString::new("".into())), // Password vuota
            None,
            PasswordPreset::Medium,
        )
        .await;

        assert!(
            result.is_err(),
            "Registration with empty password should fail"
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Password cannot be empty") || error_msg.contains("registration error"),
            "Error should mention empty password, got: {}",
            error_msg
        );

        // Verifica che NESSUN utente sia stato creato
        let user_exists: Option<SqliteRow> = query("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&pool)
            .await
            .expect("Failed to query user");

        assert!(
            user_exists.is_none(),
            "User '{}' should NOT exist after failed registration (atomic rollback)",
            username
        );
    }

    /// Test che verifica la registrazione atomica con successo.
    #[tokio::test]
    async fn test_atomic_registration_success() {
        let pool = setup_test_db().await;

        // Genera username univoco
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = format!("{:?}", std::thread::current().id());
        let username = format!("test_atomic_success_{}_{}", thread_id, timestamp);

        // Registra l'utente con settings
        let result = register_user_with_settings(
            &pool,
            username.clone(),
            Some(SecretString::new("password123".into())),
            None,
            PasswordPreset::Medium,
        )
        .await;

        // La registrazione DEVE riuscire
        assert!(
            result.is_ok(),
            "register_user_with_settings should succeed: {:?}",
            result
        );

        let user_id = result.unwrap();

        // Verifica che l'utente esista
        let user_exists: Option<SqliteRow> = query("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&pool)
            .await
            .expect("Failed to query user");

        assert!(
            user_exists.is_some(),
            "User should exist after successful registration"
        );

        // Verifica che i settings esistano
        let settings_exist: Option<SqliteRow> = query(
            "SELECT us.id FROM user_settings us WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query settings");

        assert!(
            settings_exist.is_some(),
            "User settings should exist after successful registration"
        );

        // Verifica che passwords_generation_settings esista con i valori corretti
        let gen_settings: SqliteRow = query(
            "SELECT pgs.length, pgs.symbols FROM passwords_generation_settings pgs
             JOIN user_settings us ON pgs.settings_id = us.id
             WHERE us.user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query gen_settings");

        assert_eq!(gen_settings.get::<i64, _>("length"), 8); // Medium preset
        assert_eq!(gen_settings.get::<i64, _>("symbols"), 2);
    }
}
