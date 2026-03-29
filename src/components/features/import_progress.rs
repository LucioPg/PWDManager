// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Componente per mostrare il progresso dell'import.

use crate::backend::import::import_passwords_pipeline_with_progress;
use crate::backend::migration_types::{MigrationStage, ProgressMessage};
use crate::backend::import_data::ImportData;
use crate::components::{show_toast_error, show_toast_success, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI (versione import).
fn format_import_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing import...".to_string(),
        MigrationStage::Reading => "Reading file...".to_string(),
        MigrationStage::Deserializing => "Parsing file...".to_string(),
        MigrationStage::Deduplicating => "Removing duplicates...".to_string(),
        MigrationStage::Encrypting => "Encrypting passwords...".to_string(),
        MigrationStage::Importing => "Importing to database...".to_string(),
        MigrationStage::Completed => "Import completed!".to_string(),
        MigrationStage::Failed => "Import failed".to_string(),
        _ => "Processing...".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ImportProgressChn(
    /// Callback quando l'import è completato con successo
    on_completed: Signal<bool>,

    /// Callback quando l'import fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    #[allow(clippy::redundant_closure)]
    let mut status_message = use_signal(|| String::new());
    let mut import_started = use_signal(|| false);

    let context = use_context::<Signal<ImportData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia import automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio
        if import_started() {
            return;
        }
        import_started.set(true);

        let context_for_progress = context;
        let pool_for_progress = pool.clone();
        let mut on_completed_progress = on_completed;
        let mut on_failed_progress = on_failed;
        let toast = toast;

        let (tx, mut rx) = mpsc::channel::<ProgressMessage>(100);

        // Task per ricevere progress updates
        spawn(async move {
            tracing::info!("Import progress receiver task started");
            while let Some(msg) = rx.recv().await {
                tracing::info!(
                    "Import progress received: stage={:?}, progress={}%",
                    msg.stage,
                    msg.percentage()
                );
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(format_import_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed_progress.set(true);
                }
            }
            tracing::info!("Import progress receiver task ended");
        });

        // Task per eseguire l'import
        spawn(async move {
            tracing::info!("Import task started");
            let user_id = context_for_progress.read().user_id;
            let input_path = context_for_progress.read().input_path.clone();
            let format = context_for_progress.read().format;

            tracing::info!(
                "Import data: user_id={}, path={:?}, format={:?}",
                user_id,
                input_path,
                format
            );

            let progress_tx: Option<Arc<mpsc::Sender<ProgressMessage>>> = Some(Arc::new(tx));

            tracing::info!("Calling import_passwords_pipeline_with_progress...");
            let result = import_passwords_pipeline_with_progress(
                &pool_for_progress,
                user_id,
                &input_path,
                format,
                progress_tx,
            )
            .await;

            tracing::info!(
                "Import pipeline completed with result: {:?}",
                result.is_ok()
            );

            match result {
                Ok(import_res) => {
                    show_toast_success(
                        format!(
                            "Import completed: {} passwords imported, {} skipped (duplicates), {} total in files",
                            import_res.imported_count,
                            import_res.skipped_duplicates,
                            import_res.total_in_file
                        ),
                        toast,
                    );
                }
                Err(e) => {
                    show_toast_error(format!("Import failed: {}", e), toast);
                    stage.set(MigrationStage::Failed);
                    on_failed_progress.set(true);
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
                class: "progress progress-primary w-full",
                value: "{progress}",
                max: "100",
            }

            // Percentuale
            p { class: "text-center text-sm opacity-70", "{progress}%" }
        }
    }
}
