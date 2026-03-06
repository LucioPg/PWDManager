use crate::backend::db_backend::delete_all_user_stored_passwords;
use crate::backend::export_types::ExportFormat;
use crate::components::features::export_data::ExportData;
use crate::components::globals::{ExportProgressDialog, ExportWarningDialog};
use crate::components::AllStoredPasswordDeletionDialog;
use dioxus::prelude::*;
use pwd_dioxus::{show_toast_error, use_toast};
use sqlx::SqlitePool;

/// Dashboard menu component with import/export actions and delete all option.
/// Provides a dropdown menu with nested submenus for file operations.
#[component]
pub fn DashboardMenu(on_need_restart: Signal<bool>) -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let pool = use_context::<SqlitePool>();
    let user = auth_state.get_user();
    let toast = use_toast();
    let mut error = use_signal(|| Option::<String>::None);
    let mut warning_open = use_signal(|| false);

    // Export state
    let mut export_warning_open = use_signal(|| false);
    let mut export_progress_open = use_signal(|| false);
    let mut export_completed = use_signal(|| false);
    let mut export_failed = use_signal(|| false);
    let mut export_data = use_signal(ExportData::default);
    let mut export_format = use_signal(|| ExportFormat::Json);

    // Clone user for each closure that needs it
    let user_for_delete = user.clone();
    let user_json = user.clone();
    let user_csv = user.clone();
    let user_xml = user.clone();

    let on_delete_all = move |_| {
        let user_clone = user_for_delete.clone();
        if let Some(user) = user_clone {
            let pool_clone = pool.clone();
            spawn(async move {
                match delete_all_user_stored_passwords(&pool_clone, user.id).await {
                    Ok(()) => {
                        tracing::info!("All user passwords deleted successfully");
                        println!("All user passwords deleted successfully");
                        on_need_restart.set(true);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                    }
                }
            });
        };
    };

    let on_warning_open = move |_| {
        let mut warning_open = warning_open.clone();
        warning_open.set(true);
    };

    use_effect(move || {
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error saving user: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Export handler per JSON
    let toast_json = toast.clone();
    let mut export_data_json = export_data.clone();
    let mut export_format_json = export_format.clone();
    let mut export_warning_open_json = export_warning_open.clone();
    let on_export_json = move |_| {
        let user_clone = user_json.clone();
        let toast = toast_json.clone();
        let format = ExportFormat::Json;

        spawn(async move {
            if let Some(user) = user_clone {
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(&format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_json.set(ExportData::new(user.id, path, format));
                        export_format_json.set(format);
                        export_warning_open_json.set(true);
                    }
                    Ok(None) => {
                        tracing::info!("Export cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening save dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    // Export handler per CSV
    let toast_csv = toast.clone();
    let mut export_data_csv = export_data.clone();
    let mut export_format_csv = export_format.clone();
    let mut export_warning_open_csv = export_warning_open.clone();
    let on_export_csv = move |_| {
        let user_clone = user_csv.clone();
        let toast = toast_csv.clone();
        let format = ExportFormat::Csv;

        spawn(async move {
            if let Some(user) = user_clone {
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(&format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_csv.set(ExportData::new(user.id, path, format));
                        export_format_csv.set(format);
                        export_warning_open_csv.set(true);
                    }
                    Ok(None) => {
                        tracing::info!("Export cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening save dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    // Export handler per XML
    let toast_xml = toast.clone();
    let mut export_data_xml = export_data.clone();
    let mut export_format_xml = export_format.clone();
    let mut export_warning_open_xml = export_warning_open.clone();
    let on_export_xml = move |_| {
        let user_clone = user_xml.clone();
        let toast = toast_xml.clone();
        let format = ExportFormat::Xml;

        spawn(async move {
            if let Some(user) = user_clone {
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(&format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_xml.set(ExportData::new(user.id, path, format));
                        export_format_xml.set(format);
                        export_warning_open_xml.set(true);
                    }
                    Ok(None) => {
                        tracing::info!("Export cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening save dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    // Handler per confermare l'export
    let on_export_confirm = move |_| {
        export_warning_open.set(false);
        export_progress_open.set(true);
    };

    // Handler per chiudere il progress dopo completamento
    use_effect(move || {
        if export_completed() || export_failed() {
            export_progress_open.set(false);
        }
    });

    rsx! {
        div { class: "dropdown dropdown-end",
            // Trigger button with three dots
            div {
                tabindex: "0",
                role: "button",
                class: "btn btn-ghost btn-sm btn-circle flex items-center justify-center",
                "aria-label": "Menu azioni",
                // Three dots icon (centered)
                span { class: "text-lg font-bold leading-none", "..." }
            }

            // Dropdown content
            ul {
                tabindex: "0",
                class: "dropdown-content menu bg-base-100 rounded-box z-[100] w-52 p-2 shadow-lg border border-base-300",

                // Import submenu
                li {
                    details {
                        summary { class: "cursor-pointer", "Import" }
                        ul {
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import JSON clicked");
                                    },
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import CSV clicked");
                                    },
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import XML clicked");
                                    },
                                    "XML"
                                }
                            }
                        }
                    }
                }

                // Export submenu
                li {
                    details {
                        summary { class: "cursor-pointer", "Export" }
                        ul {
                            li {
                                button {
                                    r#type: "button",
                                    onclick: on_export_json,
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: on_export_csv,
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: on_export_xml,
                                    "XML"
                                }
                            }
                        }
                    }
                }

                // Separator
                li { class: "my-1",
                    hr { class: "border-base-300" }
                }

                // Delete all (dangerous action)
                li {
                    button {
                        r#type: "button",
                        class: "text-error hover:bg-error hover:text-error-content",
                        onclick: on_warning_open,
                        "Delete All"
                    }
                }
            }
        }
        AllStoredPasswordDeletionDialog {
            open: warning_open,
            on_confirm: on_delete_all,
            on_cancel: move |_| {
                let mut warning_open = warning_open.clone();
                warning_open.set(false);
            },
        }

        // Export warning dialog
        ExportWarningDialog {
            open: export_warning_open,
            output_path: export_data.read().output_path.display().to_string(),
            format: format!("{:?}", export_format()),
            on_confirm: on_export_confirm,
            on_cancel: move |_| {
                export_warning_open.set(false);
            },
        }

        // Export progress dialog
        ExportProgressDialog {
            open: export_progress_open,
            on_completed: export_completed,
            on_failed: export_failed,
        }
    }
}
