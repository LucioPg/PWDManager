// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Dialog shown when a breaking change update is available.
//!
//! Warns the user that the database may be rebuilt and suggests
//! exporting passwords before proceeding with the update.

use super::base_modal::ModalVariant;
use crate::auth::AuthState;
use crate::backend::export::export_all_user_passwords_pipeline;
use crate::backend::export_types::ExportFormat;
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
use crate::backend::updater_types::UpdateManifest;
use crate::components::globals::WarningIcon;
use crate::components::{show_toast_error, show_toast_success, use_toast};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use rfd::FileDialog;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Context data for the breaking-change export pipeline.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct BreakingChangeExportData {
    pub user_id: i64,
    pub output_path: std::path::PathBuf,
    pub format: ExportFormat,
}

/// Dedicated progress pipeline for breaking-change export (multi-vault).
#[allow(non_snake_case)]
#[component]
pub fn BreakingChangeExportProgress(
    on_completed: Signal<bool>,
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    #[allow(clippy::redundant_closure)]
    let mut status_message = use_signal(|| String::new());
    let mut export_started = use_signal(|| false);

    let export_context = use_context::<Signal<BreakingChangeExportData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    use_effect(move || {
        if export_started() {
            return;
        }
        export_started.set(true);

        let ctx = export_context;
        let pool = pool.clone();
        let mut on_completed = on_completed;
        let mut on_failed = on_failed;
        let toast = toast;

        let (tx, mut rx) = mpsc::channel::<ProgressMessage>(100);

        // Receiver task for progress updates
        spawn(async move {
            while let Some(msg) = rx.recv().await {
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(match &msg.stage {
                    MigrationStage::Idle => "Preparing export...".to_string(),
                    MigrationStage::Decrypting => "Decrypting passwords...".to_string(),
                    MigrationStage::Serializing => "Serializing data...".to_string(),
                    MigrationStage::Writing => "Writing file...".to_string(),
                    MigrationStage::Completed => "Export completed!".to_string(),
                    MigrationStage::Failed => "Export failed".to_string(),
                    _ => "Processing...".to_string(),
                });

                if msg.stage == MigrationStage::Completed {
                    on_completed.set(true);
                }
            }
        });

        // Export task
        spawn(async move {
            let user_id = ctx.read().user_id;
            let output_path = ctx.read().output_path.clone();
            let format = ctx.read().format;

            let progress_tx: Option<Arc<ProgressSender>> = Some(Arc::new(tx));

            let result = export_all_user_passwords_pipeline(
                &pool,
                user_id,
                &output_path,
                format,
                progress_tx,
            )
            .await;

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
                    on_failed.set(true);
                }
            }
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            p { class: "text-center font-medium text-base-content", "{status_message}" }
            progress {
                class: "progress progress-warning w-full",
                value: "{progress}",
                max: "100",
            }
            p { class: "text-center text-sm opacity-70", "{progress}%" }
        }
    }
}

#[component]
pub fn BreakingChangeDialog(
    open: Signal<bool>,
    manifest: UpdateManifest,
    on_update_now: EventHandler<()>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let mut open = open;
    let mut export_completed = use_signal(|| false);
    let mut export_failed = use_signal(|| false);
    let mut is_exporting = use_signal(|| false);
    let mut show_export_progress = use_signal(|| false);

    let auth_state = use_context::<AuthState>();
    let user_id = auth_state.get_user_id();
    let toast = use_toast();

    let manifest_version = manifest.version.clone();
    let manifest_notes = manifest.notes.clone();

    // Provide context for export pipeline
    use_context_provider(|| Signal::new(BreakingChangeExportData::default()));
    let export_context = use_context::<Signal<BreakingChangeExportData>>();

    let start_export = move |_| {
        let user_id = user_id;
        let mut show_export_progress = show_export_progress;
        let mut is_exporting = is_exporting;
        let mut export_completed = export_completed;
        let mut export_failed = export_failed;
        let toast = toast;
        let mut export_context = export_context;

        // Open file dialog to choose save location
        let file_handle = FileDialog::new()
            .set_file_name("pwdmanager_backup.json")
            .add_filter("JSON", &["json"])
            .set_title("Export passwords before update");

        match file_handle.save_file() {
            Some(path) => {
                // Update export context
                *export_context.write() = BreakingChangeExportData {
                    user_id,
                    output_path: path,
                    format: ExportFormat::Json,
                };
                show_export_progress.set(true);
                is_exporting.set(true);
                export_completed.set(false);
                export_failed.set(false);
            }
            None => {
                show_toast_error("Export cancelled".to_string(), toast);
            }
        }
    };

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                open.set(false);
                on_dismiss.call(());
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    open.set(false);
                    on_dismiss.call(());
                },
                "\u{2715}"
            }

            // Warning icon
            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-8 h-8".to_string()) }
            }

            // Title
            h3 { class: "font-bold text-lg mb-2 text-center", "Important update available!" }

            // Version info
            div { class: "text-center mb-4",
                p { class: "text-sm opacity-70",
                    "Version {manifest_version}"
                }
                if !manifest_notes.is_empty() {
                    p { class: "text-xs opacity-50 mt-1",
                        "{manifest_notes}"
                    }
                }
            }

            // Breaking change warning
            div { class: "bg-warning/10 border border-warning/30 rounded-lg p-4 mb-4",
                p { class: "font-bold text-warning mb-2",
                    "This version contains important changes:"
                }
                ul { class: "list-disc list-inside text-sm space-y-1 opacity-80",
                    li { "The database may be rebuilt" }
                    li { "Multi-user support will be removed" }
                    li { "Passwords will no longer be accessible without export" }
                }
            }

            // Recommendation
            p { class: "text-sm mb-4 text-center",
                "We recommend exporting your passwords before proceeding with the update."
            }

            // Export progress (hidden until export starts)
            if show_export_progress() {
                BreakingChangeExportProgress {
                    on_completed: export_completed,
                    on_failed: export_failed,
                }
            }

            // Action buttons
            div { class: "modal-action",
                if export_completed() {
                    ActionButton {
                        text: "Update now".to_string(),
                        variant: ButtonVariant::Success,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            on_update_now.call(());
                        },
                    }
                }

                if !is_exporting() && !export_completed() {
                    ActionButton {
                        text: "Export passwords".to_string(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: start_export,
                    }
                }

                {
                    let label = if export_completed() { "Close" } else { "Later" }.to_string();
                    rsx! {
                        ActionButton {
                            text: label,
                            variant: ButtonVariant::Secondary,
                            button_type: ButtonType::Button,
                            size: ButtonSize::Normal,
                            on_click: move |_| {
                                open.set(false);
                                on_dismiss.call(());
                            },
                        }
                    }
                }
            }
        }
    }
}
