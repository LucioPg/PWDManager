// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Headless setup for NSIS installer integration.
//! Creates encrypted DB with random diceware passphrase, stores key in prod keyring.
//!
//! Uses `perform_setup()` from `db_backend` to share the same DB initialization
//! logic as the normal app startup, avoiding code duplication.

use crate::backend::db_backend::perform_setup;
use crate::backend::db_key;
use custom_errors::DBError;
use secrecy::SecretString;

/// Runs headless database setup (for NSIS installer).
/// Always uses production keyring (`SERVICE_NAME`), never dev keyring.
/// Returns the generated recovery passphrase.
pub async fn run_setup() -> Result<SecretString, DBError> {
    let passphrase = db_key::generate_recovery_passphrase()
        .map_err(|e| DBError::new_general_error(format!("Passphrase generation: {}", e)))?;

    // perform_setup derives key, stores in keyring, creates DB, runs init queries.
    // Pass SERVICE_NAME explicitly — this is always a production setup.
    let (recovery_phrase, _pool) = perform_setup(&passphrase, db_key::SERVICE_NAME).await?;
    // pool is dropped here — DB file is created and closed cleanly.

    Ok(recovery_phrase)
}
