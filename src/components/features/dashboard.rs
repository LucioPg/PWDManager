use crate::backend::db_backend::{delete_stored_password, fetch_password_stats};
use crate::backend::password_utils::get_all_stored_raw_passwords_with_filter;
use crate::components::features::DashboardMenu;
use crate::components::globals::StatsAside;
use crate::components::globals::pagination::{PaginationControls, PaginationState};
use crate::components::globals::spinner::{Spinner, SpinnerSize};
use crate::components::globals::types::TableOrder;
use crate::components::{
    StoredPasswordDeletionDialog, StoredPasswordShowDialog, StoredPasswordUpsertDialog,
    StoredRawPasswordsTable, show_toast_error, use_toast,
};
use custom_errors::DBError;
use dioxus::prelude::*;
use pwd_dioxus::Combobox;
use pwd_types::StoredRawPassword;
use sqlx::SqlitePool;
use std::ops::Deref;

fn table_order_options() -> Vec<(&'static str, Option<TableOrder>)> {
    vec![
        ("A-Z", Some(TableOrder::AZ)),
        ("Z-A", Some(TableOrder::ZA)),
        ("Oldest", Some(TableOrder::Oldest)),
        ("Newest", Some(TableOrder::Newest)),
    ]
}

#[component]
pub fn Dashboard() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let on_need_restart = use_signal(|| false);
    let username = auth_state.get_username();
    let mut stored_password_dialog_state =
        use_context_provider(|| StoredPasswordUpsertDialogState {
            is_open: Signal::new(false),
            current_stored_raw_password: Signal::new(None::<StoredRawPassword>),
        });
    let mut stored_password_show_dialog_state =
        use_context_provider(|| StoredPasswordShowDialogState {
            is_open: Signal::new(false),
            current_stored_raw_password: Signal::new(None::<StoredRawPassword>),
        });
    #[allow(unused_mut)]
    let mut deletion_password_dialog_state =
        use_context_provider(|| DeleteStoredPasswordDialogState {
            is_open: Signal::new(false),
            password_id: Signal::new(Option::<i64>::None),
        });
    // DATA
    let pool = use_context::<SqlitePool>();
    let pool_for_stats = pool.clone();
    let pool_for_delete = pool.clone();
    let mut error = use_signal(|| <Option<DBError>>::None);
    let user_id_option = auth_state.user.cloned().map(|u| u.id);
    let toast = use_toast();
    let options = table_order_options();

    // Ordinamento: default Newest (coincide con ORDER BY created_at DESC del DB)
    let mut current_table_order = use_signal(|| Some(TableOrder::Newest));

    // Dati completi per ordinamento frontend
    let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());

    // Search query per filtro client-side
    let mut search_query = use_signal(|| String::new());

    // Estrae user_id
    let user_id = user_id_option.unwrap_or(-1);

    // Stato paginazione
    let mut pagination = use_context_provider(|| PaginationState::new());

    // Resource per fetch completa (ordinamento delegato al DB)
    // Reagisce a: current_table_order, pagination.active_filter()
    let mut sorted_passwords_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id.clone();
        let filter = pagination.active_filter();
        let order_clause = current_table_order()
            .unwrap_or(TableOrder::Newest)
            .order_by_clause();

        async move {
            if user_id == -1 {
                return Vec::new();
            }

            get_all_stored_raw_passwords_with_filter(&pool, user_id, filter, order_clause)
                .await
                .unwrap_or_else(|e| {
                    error.set(Some(e));
                    Vec::new()
                })
        }
    });

    // Aggiorna all_passwords quando la resource completa
    use_effect(move || {
        if let Some(data) = sorted_passwords_resource.read().as_ref() {
            all_passwords.set(data.clone());
        }
    });

    // Filtro per nome (case-insensitive). Computazione pura.
    let filtered_passwords = use_memo(move || {
        let query = search_query();
        let query_lower = query.to_lowercase();
        let all = all_passwords();

        if query_lower.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|p| p.name.to_lowercase().contains(&query_lower))
                .collect()
        }
    });

    // Sync total_count con i risultati filtrati
    use_effect(move || {
        let count = filtered_passwords().len();
        pagination.total_count.set(count as u64);
    });

    // Paginazione locale: slice dei dati filtrati
    let page_data = use_memo(move || {
        let page = pagination.current_page();
        let page_size = pagination.page_size();
        let filtered = filtered_passwords();

        let start = page * page_size;
        let end = (start + page_size).min(filtered.len());
        if start < filtered.len() {
            Some(filtered[start..end].to_vec())
        } else {
            Some(Vec::new())
        }
    });

    // Stats sempre fresche (query separata)
    let stats_data = use_resource(move || {
        let pool = pool_for_stats.clone();
        async move {
            if user_id == -1 {
                return None;
            }
            match fetch_password_stats(&pool, user_id).await {
                Ok(stats) => Some(stats),
                Err(e) => {
                    error.set(Some(e));
                    None
                }
            }
        }
    });

    // stored raw passwords

    // Stats dalle query DB (non più calcolate lato client)
    let stats = use_memo(move || stats_data.read().clone().flatten().unwrap_or_default());

    // upsert modal - refresh tabella dopo salvataggio
    let on_confirm_upsert = {
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        move |_| {
            stats_data.restart();
            sorted_passwords_resource.restart();
        }
    };

    // deletion modal
    let on_confirm_delete = {
        let pool = pool_for_delete.clone();
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        let mut deletion_password_dialog_state = deletion_password_dialog_state.clone();
        let mut error = error.clone();

        move |_| {
            let pool = pool.clone();
            let mut delete_state = deletion_password_dialog_state.clone();
            let mut error_signal = error.clone();
            let mut stats_data = stats_data.clone();
            let mut sorted_passwords_resource = sorted_passwords_resource.clone();

            let Some(password_id) = (delete_state.password_id)() else {
                error_signal.set(Some(DBError::new_general_error(
                    "A Stored Password id is required".to_string(),
                )));
                return;
            };

            spawn(async move {
                let result = delete_stored_password(&pool, password_id).await;
                match result {
                    Ok(_) => {
                        stats_data.restart();
                        delete_state.is_open.set(false);
                        sorted_passwords_resource.restart();
                    }
                    Err(e) => {
                        error_signal.set(Some(e));
                    }
                }
            });

            deletion_password_dialog_state.password_id.set(None);
        }
    };
    let cancel_delete = move |_| {};

    use_effect(move || {
        if let Some(e) = error.read().deref() {
            show_toast_error(format!("Error fetching user data: {}", e), toast.clone());
        }
    });

    use_effect(move || {
        let mut need_restart = on_need_restart.clone();
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        if need_restart() {
            stats_data.restart();
            sorted_passwords_resource.restart();
            need_restart.set(false);
        }
    });

    rsx! {
        // Stats Aside - posizionato fixed con z-index alto
        StatsAside {
            stats: stats(),
            on_stat_click: move |strength| {
                pagination.set_filter(strength);
                pagination.go_to_page(0);
                sorted_passwords_resource.restart();
            },
            active_filter: pagination.active_filter(),
        }

        // Main content con margin-left per fare spazio all'aside collassato (52px)
        div { class: "content-container animate-fade-in ml-16",
            div { class: "mb-8 flex justify-between items-start align-center",
                div {
                    h1 { class: "text-h2", "Welcome, {username}!" }
                    p { class: "text-body mt-2", "Manage your passwords and secure your digital life" }
                }
                DashboardMenu { on_need_restart: on_need_restart.clone() }
            }
            div { class: "pwd-controls-bar",
                div { class: "pwd-controls-left",
                    // Search input
                    div { class: "pwd-search-wrapper",
                        svg {
                            class: "pwd-search-icon",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path { d: "M21 21l-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z" }
                        }
                        input {
                            class: "input input-bordered input-sm pwd-search-input",
                            r#type: "text",
                            placeholder: "Cerca per nome...",
                            value: "{search_query}",
                            oninput: move |e| {
                                let value = e.value();
                                search_query.set(value);
                                pagination.go_to_page(0);
                            },
                        }
                        if !search_query().is_empty() {
                            button {
                                class: "pwd-search-clear",
                                onclick: move |_| {
                                    search_query.set(String::new());
                                    pagination.go_to_page(0);
                                },
                                svg {
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    path { d: "M18 6L6 18M6 6l12 12" }
                                }
                            }
                        }
                    }
                    // Sort Combobox
                    Combobox::<TableOrder> {
                        options: options.clone(),
                        placeholder: "Order by".to_string(),
                        on_change: move |v| {
                            current_table_order.set(v);
                            pagination.go_to_page(0);
                            sorted_passwords_resource.restart();
                        },
                    }
                }
                button {
                    class: "btn btn-success",
                    r#type: "button",
                    onclick: move |_| {
                        stored_password_dialog_state.current_stored_raw_password.set(None);
                        stored_password_dialog_state.is_open.set(true);
                    },
                    "New Password"
                }
            }

            {
                let table_data = page_data();
                if sorted_passwords_resource.read().is_none() {
                    rsx! {
                        div { class: "card card-lg",
                            div { class: "flex justify-center py-8",
                                Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }
                            }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "card card-lg",
                            StoredRawPasswordsTable { data: table_data }
                        }
                    }
                }
            }

            // Controlli paginazione
            PaginationControls {
                pagination: pagination.clone(),
                on_page_change: move |new_page| {
                    pagination.go_to_page(new_page);
                },
            }
        }
        // on_cancel gestito internamente al componente
        StoredPasswordUpsertDialog { on_confirm: on_confirm_upsert, on_cancel: move |_| {} }
        StoredPasswordShowDialog { on_confirm: move |_| {}, on_cancel: move |_| {} }
        StoredPasswordDeletionDialog {
            open: deletion_password_dialog_state.is_open.clone(),
            on_confirm: on_confirm_delete,
            on_cancel: cancel_delete,
        }
    }
}

#[derive(Clone)]
pub struct StoredPasswordUpsertDialogState {
    pub is_open: Signal<bool>,
    pub current_stored_raw_password: Signal<Option<StoredRawPassword>>,
}

#[derive(Clone)]
pub struct DeleteStoredPasswordDialogState {
    pub is_open: Signal<bool>,
    pub password_id: Signal<Option<i64>>,
}

#[derive(Clone)]
pub struct StoredPasswordShowDialogState {
    pub is_open: Signal<bool>,
    pub current_stored_raw_password: Signal<Option<StoredRawPassword>>,
}
