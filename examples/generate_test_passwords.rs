//! Example per generare 10000 StoredPassword nel database dell'applicazione.
//!
//! Questo example popola il database con dati di test usando la stessa pipeline
//! dell'applicazione (crittografia AES-256-GCM, Argon2 per derivazione chiavi).
//!
//! # Prerequisiti
//!
//! - L'utente "t" (id=1) deve esistere nel database
//! - Password in chiaro: "t"
//! - Password hash: "$argon2id$v=19$m=19456,t=2,p=1$Q6yMuyZkFjPytAY0Eq+i/g$3ZYZjpIpWuWONYAaX2yBsfcvB6IohAvQTBizZ+tyv44"
//!
//! # Esecuzione
//!
//! ```bash
//! cargo run --example generate_test_passwords
//! ```
//!
//! # Attenzione
//!
//! Questo script modifica il database dell'applicazione (database.db).
//! Eseguire solo in ambiente di sviluppo!

use custom_errors::DBError;
use pwd_types::{PasswordPreset, PasswordScore, StoredRawPassword};
use secrecy::{ExposeSecret, SecretString};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

// Importa le funzioni dalla crate principale
use pwd_manager::backend::db_backend::{
    create_user_settings, fetch_user_auth_from_id, fetch_user_passwords_generation_settings,
};
use pwd_manager::backend::evaluate_password_strength;
use pwd_manager::backend::password_utils::{
    create_stored_data_pipeline_bulk, generate_suggested_password,
};

/// Configurazione del generatore di password
const TOTAL_PASSWORDS: usize = 10_000;
const BATCH_SIZE: usize = 100; // Salva in batch per evitare memory issues

/// Preset disponibili per la generazione delle password
const PRESETS: [PasswordPreset; 4] = [
    PasswordPreset::Medium,
    PasswordPreset::Strong,
    PasswordPreset::Epic,
    PasswordPreset::God,
];

/// Inizializza la connessione al database dell'applicazione.
///
/// Usa lo stesso path dell'app principale: `sqlite:database.db`
async fn init_app_db() -> Result<SqlitePool, DBError> {
    let options = SqliteConnectOptions::from_str("sqlite:database.db")
        .map_err(|e| DBError::new_general_error(e.to_string()))?
        .pragma("foreign_keys", "ON")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| DBError::new_general_error(e.to_string()))?;

    Ok(pool)
}

/// Verifica che l'utente di test esista nel database.
async fn verify_test_user(pool: &SqlitePool, user_id: i64) -> Result<(), DBError> {
    println!("Verifico utente con id={}...", user_id);

    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;

    println!("Utente trovato:");
    println!("  ID: {}", user_auth.id);
    println!(
        "  Password hash (primi 50 char): {}...",
        &user_auth.password.0.expose_secret()[..50]
    );

    Ok(())
}

/// Genera un singolo StoredRawPassword con preset casuale.
fn generate_single_password(user_id: i64, index: usize, settings_id: i64) -> StoredRawPassword {
    // Seleziona preset in base all'indice (distribuzione uniforme)
    let preset = &PRESETS[index % PRESETS.len()];
    let config = preset.to_config(settings_id);

    // Genera password con il preset selezionato
    let password = generate_suggested_password(Some(config));

    // Calcola lo score della password generata
    let evaluation = evaluate_password_strength(&password, None);
    let score = evaluation.score.unwrap_or(PasswordScore::new(0));

    // Crea location incrementale
    let location = format!("location_{}", index + 1);

    // Crea StoredRawPassword
    StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: format!("Service {}", index + 1),
        username: SecretString::new(format!("user{}@example.com", index + 1).into()),
        location: SecretString::new(location.into()),
        password,
        notes: Some(SecretString::new(
            format!("Nota di test per password #{}", index + 1).into(),
        )),
        score: Some(score),
        created_at: None,
    }
}

/// Genera e salva le password in batch.
async fn generate_and_save_passwords(
    pool: &SqlitePool,
    user_id: i64,
    settings_id: i64,
) -> Result<(), DBError> {
    println!("\n=== Generazione password ===");
    println!("Totale password da generare: {}", TOTAL_PASSWORDS);
    println!("Batch size: {}", BATCH_SIZE);

    let total_start = Instant::now();
    let mut generated_count = 0;

    // Processa in batch per evitare problemi di memoria
    for batch_start in (0..TOTAL_PASSWORDS).step_by(BATCH_SIZE) {
        let batch_end = std::cmp::min(batch_start + BATCH_SIZE, TOTAL_PASSWORDS);
        let batch_num = batch_start / BATCH_SIZE + 1;
        let total_batches = (TOTAL_PASSWORDS + BATCH_SIZE - 1) / BATCH_SIZE;

        println!(
            "\nBatch {}/{} (password {}-{})...",
            batch_num,
            total_batches,
            batch_start + 1,
            batch_end
        );

        let batch_start_time = Instant::now();

        // Genera password per questo batch
        let stored_raw_passwords: Vec<StoredRawPassword> = (batch_start..batch_end)
            .map(|i| generate_single_password(user_id, i, settings_id))
            .collect();

        println!(
            "  Generate {} password in {:?}",
            stored_raw_passwords.len(),
            batch_start_time.elapsed()
        );

        // Salva nel database usando la pipeline completa
        let save_start = Instant::now();
        create_stored_data_pipeline_bulk(pool, user_id, stored_raw_passwords).await?;

        generated_count += batch_end - batch_start;

        println!(
            "  Salvate {} password in {:?}",
            batch_end - batch_start,
            save_start.elapsed()
        );
        println!(
            "  Progresso totale: {}/{} ({:.1}%)",
            generated_count,
            TOTAL_PASSWORDS,
            (generated_count as f64 / TOTAL_PASSWORDS as f64) * 100.0
        );
    }

    println!("\n=== Completato ===");
    println!("Tempo totale: {:?}", total_start.elapsed());
    println!("Password generate: {}", generated_count);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("  Generatore di password di test");
    println!("  PWDManager - Database populator");
    println!("========================================\n");

    // Inizializza la blacklist per la valutazione delle password
    // Questo è necessario per evaluate_password_strength
    println!("Inizializzazione blacklist...");
    pwd_manager::backend::init_blacklist_from_path("assets/blacklist.txt")?;
    println!("Blacklist caricata.\n");

    // Connetti al database dell'applicazione
    println!("Connessione al database dell'applicazione (database.db)...");
    let pool = init_app_db().await?;
    println!("Connesso.\n");

    // Verifica che l'utente di test esista
    const TEST_USER_ID: i64 = 1;
    verify_test_user(&pool, TEST_USER_ID).await?;

    // Recupera o crea i settings dell'utente
    println!("\nConfigurazione settings utente...");
    let settings_id = match fetch_user_passwords_generation_settings(&pool, TEST_USER_ID).await {
        Ok(config) => {
            println!(
                "Settings esistenti trovati (ID: {})",
                config.id.unwrap_or(0)
            );
            config.id.unwrap_or(1)
        }
        Err(_) => {
            println!("Creazione nuovi settings...");
            create_user_settings(&pool, TEST_USER_ID, PasswordPreset::God).await?
        }
    };
    println!("Settings ID: {}", settings_id);

    // Genera e salva le password
    generate_and_save_passwords(&pool, TEST_USER_ID, settings_id).await?;

    // Chiudi la connessione
    println!("\nChiusura connessione database...");
    pool.close().await;
    println!("Operazione completata con successo!");

    Ok(())
}
