// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::backend::db_backend::delete_vault_passwords;
use crate::backend::export_types::ExportFormat;
use crate::backend::import::validate_import_path;
use crate::backend::vault_utils::{fetch_password_count_by_vault, fetch_vaults_by_user};
use crate::components::globals::ActiveVaultState;
use crate::components::globals::spinner::{Spinner, SpinnerSize};
use crate::components::globals::{
    ExportProgressDialog, ExportWarningDialog, ImportProgressDialog, ImportWarningDialog,
};
use crate::components::{
    AllStoredPasswordDeletionDialog, ExportData, ImportData, VaultCreateDialog, VaultDeleteDialog,
    VaultEditDialog, show_toast_error, use_toast,
};
use dioxus::prelude::*;
use pwd_types::Vault;
use rfd::FileDialog;
use sqlx::SqlitePool;

/// MyVaults page: vault card grid with CRUD, import/export/delete scoped to selected vault.
#[component]
pub fn MyVaults() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let pool = use_context::<SqlitePool>();
    let pool_for_effect = pool.clone();
    let pool_for_delete = pool.clone();
    let user = auth_state.get_user();
    let toast = use_toast();
    let user_id = user.as_ref().map(|u| u.id).unwrap_or(-1);

    // Vault list resource
    let mut vaults_resource = use_resource(move || {
        let pool = pool.clone();
        async move {
            if user_id == -1 {
                return Vec::new();
            }
            fetch_vaults_by_user(&pool, user_id)
                .await
                .unwrap_or_default()
        }
    });

    // Password count per vault
    let mut password_counts = use_signal(std::collections::HashMap::<i64, u64>::new);

    // Fetch password counts whenever vault list changes
    use_effect(move || {
        let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        let pool = pool_for_effect.clone();
        spawn(async move {
            let mut counts = std::collections::HashMap::new();
            for vault in &vaults {
                if let Some(id) = vault.id {
                    let count = fetch_password_count_by_vault(&pool, id).await.unwrap_or(0);
                    counts.insert(id, count);
                }
            }
            password_counts.set(counts);
        });
    });

    // Dialog states
    let mut create_dialog_open = use_signal(|| false);
    let mut edit_dialog_open = use_signal(|| false);
    let mut delete_dialog_open = use_signal(|| false);
    let mut delete_warning_open = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    // Vault being edited or deleted
    let mut edit_vault = use_signal(|| Option::<Vault>::None);
    let mut delete_vault = use_signal(|| Option::<Vault>::None);
    let mut delete_password_count = use_signal(|| 0u64);

    // Export state
    let mut export_warning_open = use_signal(|| false);
    let mut export_progress_open = use_signal(|| false);
    let export_completed = use_signal(|| false);
    let export_failed = use_signal(|| false);
    let export_data = use_context_provider(|| Signal::new(ExportData::default()));
    let export_format = use_signal(|| ExportFormat::Json);

    // Import state
    let mut import_warning_open = use_signal(|| false);
    let mut import_progress_open = use_signal(|| false);
    let import_completed = use_signal(|| false);
    let import_failed = use_signal(|| false);
    let import_data = use_context_provider(|| Signal::new(ImportData::default()));
    let import_format = use_signal(|| ExportFormat::Json);

    // Active vault state for import/export/delete scope
    let active_vault_state = use_context::<ActiveVaultState>();

    // Error toast effect
    use_effect(move || {
        let mut this_error = error;
        let toast = toast;
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error: {}", msg), toast);
            this_error.set(None);
        }
    });

    // --- Vault CRUD handlers ---

    let on_vault_created = move |_| {
        vaults_resource.restart();
    };

    let on_vault_updated = move |_| {
        vaults_resource.restart();
        edit_dialog_open.set(false);
    };

    let on_vault_deleted = move |_| {
        vaults_resource.restart();
        delete_dialog_open.set(false);
    };

    let mut on_edit_click = move |vault: Vault| {
        edit_vault.set(Some(vault));
        edit_dialog_open.set(true);
    };

    let mut on_delete_click = move |vault: Vault| {
        let vid = vault.id.unwrap_or(0);
        let count = password_counts.read().get(&vid).copied().unwrap_or(0);
        delete_vault.set(Some(vault));
        delete_password_count.set(count);
        delete_dialog_open.set(true);
    };

    // --- Delete All Passwords handler (scoped to active vault) ---

    let on_delete_all = move |_| {
        let vault_id = active_vault_state.0().unwrap_or(-1);
        if vault_id == -1 {
            error.set(Some("No vault selected".to_string()));
            return;
        }
        let pool = pool_for_delete.clone();
        let mut error_signal = error;
        let mut password_counts = password_counts;
        spawn(async move {
            match delete_vault_passwords(&pool, vault_id).await {
                Ok(()) => {
                    tracing::info!("All vault passwords deleted successfully");
                    let mut counts = password_counts.read().clone();
                    counts.insert(vault_id, 0);
                    password_counts.set(counts);
                }
                Err(e) => {
                    error_signal.set(Some(e.to_string()));
                }
            }
        });
    };

    let on_warning_open = move |_| {
        delete_warning_open.set(true);
    };

    // --- Export handlers (scoped to active vault) ---
    // Following the same explicit-closure pattern as the former DashboardMenu.

    let user_json = user.clone();
    let toast_json = toast;
    let export_data_for_json = export_data;
    let export_format_for_json = export_format;
    let export_warning_open_for_json = export_warning_open;
    let on_export_json = move |_| {
        let user_clone = user_json.clone();
        let toast = toast_json;
        let mut export_data_json = export_data_for_json;
        let mut export_format_json = export_format_for_json;
        let mut export_warning_open_json = export_warning_open_for_json;
        let format = ExportFormat::Json;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_json.set(ExportData::new(user.id, vault_id, path, format));
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

    let user_csv = user.clone();
    let toast_csv = toast;
    let export_data_for_csv = export_data;
    let export_format_for_csv = export_format;
    let export_warning_open_for_csv = export_warning_open;
    let on_export_csv = move |_| {
        let user_clone = user_csv.clone();
        let toast = toast_csv;
        let mut export_data_csv = export_data_for_csv;
        let mut export_format_csv = export_format_for_csv;
        let mut export_warning_open_csv = export_warning_open_for_csv;
        let format = ExportFormat::Csv;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_csv.set(ExportData::new(user.id, vault_id, path, format));
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

    let user_xml = user.clone();
    let toast_xml = toast;
    let export_data_for_xml = export_data;
    let export_format_for_xml = export_format;
    let export_warning_open_for_xml = export_warning_open;
    let on_export_xml = move |_| {
        let user_clone = user_xml.clone();
        let toast = toast_xml;
        let mut export_data_xml = export_data_for_xml;
        let mut export_format_xml = export_format_for_xml;
        let mut export_warning_open_xml = export_warning_open_for_xml;
        let format = ExportFormat::Xml;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let ext = format.extension().to_string();
                let file_result = tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Export File", &[ext.as_str()])
                        .set_file_name(format!("pwdmanager_export.{}", ext))
                        .save_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => {
                        export_data_xml.set(ExportData::new(user.id, vault_id, path, format));
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

    // Export confirm / progress
    let on_export_confirm = move |_| {
        export_warning_open.set(false);
        export_progress_open.set(true);
    };

    use_effect(move || {
        if export_completed() || export_failed() {
            export_progress_open.set(false);
        }
    });

    // --- Import handlers (scoped to active vault) ---

    let user_import_json = user.clone();
    let toast_import_json = toast;
    let import_data_for_json = import_data;
    let import_format_for_json = import_format;
    let import_warning_open_for_json = import_warning_open;
    let on_import_json = move |_| {
        let user_clone = user_import_json.clone();
        let toast = toast_import_json;
        let mut import_data_json = import_data_for_json;
        let mut import_format_json = import_format_for_json;
        let mut import_warning_open_json = import_warning_open_for_json;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let file_result = tokio::task::spawn_blocking(move || {
                    FileDialog::new()
                        .add_filter("Import File", &["json"])
                        .set_title("Import JSON passwords")
                        .pick_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => match validate_import_path(&path) {
                        Ok(detected_format) => {
                            import_data_json.set(ImportData::new(
                                user.id,
                                vault_id,
                                path,
                                detected_format,
                            ));
                            import_format_json.set(detected_format);
                            import_warning_open_json.set(true);
                        }
                        Err(e) => {
                            show_toast_error(format!("Invalid file: {}", e), toast);
                        }
                    },
                    Ok(None) => {
                        tracing::info!("Import cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening file dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    let user_import_csv = user.clone();
    let toast_import_csv = toast;
    let import_data_for_csv = import_data;
    let import_format_for_csv = import_format;
    let import_warning_open_for_csv = import_warning_open;
    let on_import_csv = move |_| {
        let user_clone = user_import_csv.clone();
        let toast = toast_import_csv;
        let mut import_data_csv = import_data_for_csv;
        let mut import_format_csv = import_format_for_csv;
        let mut import_warning_open_csv = import_warning_open_for_csv;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let file_result = tokio::task::spawn_blocking(move || {
                    FileDialog::new()
                        .add_filter("Import File", &["csv"])
                        .set_title("Import CSV passwords")
                        .pick_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => match validate_import_path(&path) {
                        Ok(detected_format) => {
                            import_data_csv.set(ImportData::new(
                                user.id,
                                vault_id,
                                path,
                                detected_format,
                            ));
                            import_format_csv.set(detected_format);
                            import_warning_open_csv.set(true);
                        }
                        Err(e) => {
                            show_toast_error(format!("Invalid file: {}", e), toast);
                        }
                    },
                    Ok(None) => {
                        tracing::info!("Import cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening file dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    let user_import_xml = user.clone();
    let toast_import_xml = toast;
    let import_data_for_xml = import_data;
    let import_format_for_xml = import_format;
    let import_warning_open_for_xml = import_warning_open;
    let on_import_xml = move |_| {
        let user_clone = user_import_xml.clone();
        let toast = toast_import_xml;
        let mut import_data_xml = import_data_for_xml;
        let mut import_format_xml = import_format_for_xml;
        let mut import_warning_open_xml = import_warning_open_for_xml;

        spawn(async move {
            if let Some(user) = user_clone {
                let vault_id = active_vault_state.0().unwrap_or(0);
                let file_result = tokio::task::spawn_blocking(move || {
                    FileDialog::new()
                        .add_filter("Import File", &["xml"])
                        .set_title("Import XML passwords")
                        .pick_file()
                })
                .await;

                match file_result {
                    Ok(Some(path)) => match validate_import_path(&path) {
                        Ok(detected_format) => {
                            import_data_xml.set(ImportData::new(
                                user.id,
                                vault_id,
                                path,
                                detected_format,
                            ));
                            import_format_xml.set(detected_format);
                            import_warning_open_xml.set(true);
                        }
                        Err(e) => {
                            show_toast_error(format!("Invalid file: {}", e), toast);
                        }
                    },
                    Ok(None) => {
                        tracing::info!("Import cancelled by user");
                    }
                    Err(e) => {
                        show_toast_error(format!("Error opening file dialog: {}", e), toast);
                    }
                }
            }
        });
    };

    // Import confirm / progress
    let on_import_confirm = move |_| {
        import_warning_open.set(false);
        import_progress_open.set(true);
    };

    use_effect(move || {
        if import_completed() || import_failed() {
            import_progress_open.set(false);
            vaults_resource.restart();
        }
    });

    // --- Render ---

    let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
    let counts = password_counts.read();

    rsx! {
        div { class: "content-container animate-fade-in",
            // Header
            div { class: "flex items-center justify-between mb-8",
                div {
                    h1 { class: "text-h2", "My Vaults" }
                    p { class: "text-body mt-2", "Manage your vaults and their contents" }
                }
                button {
                    class: "btn btn-success",
                    r#type: "button",
                    onclick: move |_| create_dialog_open.set(true),
                    "+ New Vault"
                }
            }

            // Import/Export/Delete All bar
            div { class: "flex items-center gap-2 mb-6",
                // Import dropdown
                div { class: "dropdown",
                    div { tabindex: "0", role: "button", class: "btn btn-sm", "Import" }
                    ul {
                        tabindex: "0",
                        class: "dropdown-content menu bg-base-100 rounded-box z-[100] w-40 p-2 shadow-lg border border-base-300",
                        li {
                            button { r#type: "button", onclick: on_import_json, "JSON" }
                        }
                        li {
                            button { r#type: "button", onclick: on_import_csv, "CSV" }
                        }
                        li {
                            button { r#type: "button", onclick: on_import_xml, "XML" }
                        }
                    }
                }

                // Export dropdown
                div { class: "dropdown",
                    div { tabindex: "0", role: "button", class: "btn btn-sm", "Export" }
                    ul {
                        tabindex: "0",
                        class: "dropdown-content menu bg-base-100 rounded-box z-[100] w-40 p-2 shadow-lg border border-base-300",
                        li {
                            button { r#type: "button", onclick: on_export_json, "JSON" }
                        }
                        li {
                            button { r#type: "button", onclick: on_export_csv, "CSV" }
                        }
                        li {
                            button { r#type: "button", onclick: on_export_xml, "XML" }
                        }
                    }
                }

                // Delete All (scoped to active vault)
                button {
                    r#type: "button",
                    class: "btn btn-sm btn-ghost text-error hover:bg-error hover:text-error-content ml-auto",
                    onclick: on_warning_open,
                    "Delete All Passwords"
                }
            }

            // Vault card grid
            if vaults_resource.read().is_none() {
                div { class: "flex justify-center py-12",
                    Spinner { size: SpinnerSize::Medium, color_class: "text-info" }
                }
            } else if vaults.is_empty() {
                div { class: "pwd-empty-state",
                    div { class: "pwd-empty-state-icon",
                        svg {
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            rect {
                                x: "3",
                                y: "11",
                                width: "18",
                                height: "11",
                                rx: "2",
                                ry: "2",
                            }
                            path { d: "M7 11V7a5 5 0 0 1 10 0v4" }
                        }
                    }
                    h3 { class: "text-h3", "No vaults yet" }
                    p { class: "text-body mt-2 pwd-empty-state-subtitle",
                        "Create your first vault to start storing passwords."
                    }
                    button {
                        class: "btn btn-primary mt-4",
                        onclick: move |_| create_dialog_open.set(true),
                        "+ New Vault"
                    }
                }
            } else {
                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                    for vault in vaults.iter() {
                        {
                            let vault_clone = vault.clone();
                            let count = vault.id.and_then(|id| counts.get(&id).copied()).unwrap_or(0);
                            rsx! {
                                super::vault_card::VaultCard {
                                    key: "{vault_clone.id.unwrap_or(0)}",
                                    vault: vault_clone,
                                    password_count: count,
                                    on_edit: move |v: Vault| {
                                        on_edit_click(v);
                                    },
                                    on_delete: move |v: Vault| {
                                        on_delete_click(v);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }

        // --- Dialogs ---

        // Create vault dialog
        VaultCreateDialog {
            open: create_dialog_open,
            on_created: on_vault_created,
            on_cancel: move |_| create_dialog_open.set(false),
        }

        // Edit vault dialog
        if let Some(vault) = edit_vault() {
            VaultEditDialog {
                key: "{vault.id.unwrap_or(0)}-edit",
                open: edit_dialog_open,
                vault,
                on_updated: on_vault_updated,
                on_cancel: move |_| edit_dialog_open.set(false),
            }
        }

        // Delete vault dialog
        if let Some(vault) = delete_vault() {
            VaultDeleteDialog {
                key: "{vault.id.unwrap_or(0)}-delete",
                open: delete_dialog_open,
                vault,
                password_count: delete_password_count(),
                on_deleted: on_vault_deleted,
                on_cancel: move |_| delete_dialog_open.set(false),
            }
        }

        // Delete all passwords dialog (scoped to active vault)
        AllStoredPasswordDeletionDialog {
            open: delete_warning_open,
            on_confirm: on_delete_all,
            on_cancel: move |_| delete_warning_open.set(false),
        }

        // Export warning dialog
        ExportWarningDialog {
            open: export_warning_open,
            output_path: export_data.read().output_path.display().to_string(),
            format: format!("{:?}", export_format()),
            on_confirm: on_export_confirm,
            on_cancel: move |_| export_warning_open.set(false),
        }

        // Export progress dialog
        ExportProgressDialog {
            open: export_progress_open,
            on_completed: export_completed,
            on_failed: export_failed,
        }

        // Import warning dialog
        ImportWarningDialog {
            open: import_warning_open,
            input_path: import_data.read().input_path.display().to_string(),
            format: format!("{:?}", import_format()),
            on_confirm: on_import_confirm,
            on_cancel: move |_| import_warning_open.set(false),
        }

        // Import progress dialog
        ImportProgressDialog {
            open: import_progress_open,
            on_completed: import_completed,
            on_failed: import_failed,
        }
    }
}
