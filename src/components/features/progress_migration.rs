use crate::backend::db_backend::restore_old_password;
use crate::backend::migration_types::{MigrationStage, ProgressMessage};
use crate::backend::password_utils::stored_passwords_migration_pipeline_with_progress;
use crate::components::{MigrationData, show_toast_error, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI.
fn format_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing...".to_string(),
        MigrationStage::Decrypting => "Decrypting passwords...".to_string(),
        MigrationStage::Encrypting => "Encrypting...".to_string(),
        MigrationStage::Serializing => "Serializing data...".to_string(),
        MigrationStage::Deserializing => "Parsing file...".to_string(),
        MigrationStage::Reading => "Reading file...".to_string(),
        MigrationStage::Writing => "Writing file...".to_string(),
        MigrationStage::Deduplicating => "Removing duplicates...".to_string(),
        MigrationStage::Importing => "Importing to database...".to_string(),
        MigrationStage::Finalizing => "Finalizing...".to_string(),
        MigrationStage::Completed => "Completed!".to_string(),
        MigrationStage::Failed => "Operation failed".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ProgressMigrationChn(
    /// Callback quando la migrazione è completata con successo
    on_completed: Signal<bool>,

    /// Callback quando la migrazione fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    let mut status_message = use_signal(|| String::new());
    let mut migration_started = use_signal(|| false); // Flag per evitare doppi avvii
    let context = use_context::<Signal<MigrationData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia migrazione automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio della migrazione
        if migration_started() {
            return;
        }
        migration_started.set(true);

        let context = context.clone();
        let pool = pool.clone();
        let mut on_completed = on_completed.clone();
        let mut on_failed = on_failed.clone();
        let toast = toast.clone();

        let (tx, mut rx) = mpsc::channel::<ProgressMessage>(100);

        // Task per ricevere progress updates
        spawn(async move {
            while let Some(msg) = rx.recv().await {
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(format_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed.set(true);
                }
            }
        });

        // Task per eseguire la migrazione
        spawn(async move {
            let user_id = context.read().user_id;
            let old_password = context.read().old_password.clone();

            match (user_id, old_password) {
                (Some(uid), Some(pwd)) => {
                    let result = stored_passwords_migration_pipeline_with_progress(
                        &pool,
                        uid,
                        pwd,
                        Some(Arc::new(tx)),
                    )
                    .await;

                    if let Err(e) = result {
                        // Mostra toast errore
                        show_toast_error(format!("Migration failed: {}", e), toast);

                        // Rollback password
                        let _ = restore_old_password(&pool, uid).await;

                        // Imposta stato fallito
                        stage.set(MigrationStage::Failed);
                        on_failed.set(true);
                    }
                }
                _ => {
                    // Se almeno user_id c'è, prova a fare rollback della password
                    if let Some(uid) = user_id {
                        let _ = restore_old_password(&pool, uid).await;
                    }
                    show_toast_error("Migration failed: missing user data".to_string(), toast);
                    stage.set(MigrationStage::Failed);
                    on_failed.set(true);
                }
            }
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4 w-full futuristic",
            // Messaggio stato
            p { class: "text-center font-medium text-base-content", "{status_message}" }

            // Progress bar DaisyUI
            progress {
                class: "progress progress-primary w-full futuristic",
                value: "{progress}",
                max: "100",
            }

            // Percentuale
            p { class: "text-center text-sm opacity-70", "{progress}%" }
        }
    }
}
