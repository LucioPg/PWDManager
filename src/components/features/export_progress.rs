//! Componente per mostrare il progresso dell'export.

use crate::backend::export::export_passwords_pipeline_with_progress;
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use crate::components::ExportData;
use crate::components::{show_toast_error, show_toast_success, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI (versione export).
fn format_export_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing export...".to_string(),
        MigrationStage::Decrypting => "Decrypting passwords...".to_string(),
        MigrationStage::Serializing => "Serializing data...".to_string(),
        MigrationStage::Writing => "Writing file...".to_string(),
        MigrationStage::Completed => "Export completed!".to_string(),
        MigrationStage::Failed => "Export failed".to_string(),
        _ => "Processing...".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ExportProgressChn(
    /// Signal che diventa true quando l'export è completato
    on_completed: Signal<bool>,

    /// Signal che diventa true se l'export fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    let mut status_message = use_signal(|| String::new());
    let mut export_started = use_signal(|| false);

    let context = use_context::<Signal<ExportData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia export automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio
        if export_started() {
            return;
        }
        export_started.set(true);

        let context_for_progress = context.clone();
        let pool_for_progress = pool.clone();
        let mut on_completed_progress = on_completed.clone();
        let mut on_failed_progress = on_failed.clone();
        let toast = toast.clone();

        let (tx, mut rx) = mpsc::channel::<ProgressMessage>(100);

        // Task per ricevere progress updates
        spawn(async move {
            tracing::info!("Export progress receiver task started");
            while let Some(msg) = rx.recv().await {
                tracing::info!(
                    "Export progress received: stage={:?}, progress={}%",
                    msg.stage,
                    msg.percentage()
                );
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(format_export_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed_progress.set(true);
                }
            }
            tracing::info!("Export progress receiver task ended");
        });

        // Task per eseguire l'export
        spawn(async move {
            tracing::info!("Export task started");
            let user_id = context_for_progress.read().user_id;
            let output_path = context_for_progress.read().output_path.clone();
            let format = context_for_progress.read().format;

            tracing::info!(
                "Export data: user_id={}, path={:?}, format={:?}",
                user_id,
                output_path,
                format
            );

            let progress_tx: Option<Arc<ProgressSender>> = Some(Arc::new(tx));

            tracing::info!("Calling export_passwords_pipeline_with_progress...");
            let result = export_passwords_pipeline_with_progress(
                &pool_for_progress,
                user_id,
                &output_path,
                format,
                progress_tx,
            )
            .await;

            tracing::info!(
                "Export pipeline completed with result: {:?}",
                result.is_ok()
            );

            match result {
                Ok(()) => {
                    show_toast_success(
                        format!("Export completed: {}", output_path.display()),
                        toast,
                    );
                }
                Err(e) => {
                    show_toast_error(format!("Export failed: {}", e), toast);
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
                class: "progress progress-primary w-full futuristic",
                value: "{progress}",
                max: "100",
            }

            // Percentuale
            p { class: "text-center text-sm opacity-70", "{progress}%" }
        }
    }
}
