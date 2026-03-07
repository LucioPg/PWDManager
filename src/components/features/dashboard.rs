use crate::backend::db_backend::{delete_stored_password, fetch_password_stats};
use crate::backend::password_utils::get_stored_raw_passwords_paginated;
use crate::components::features::DashboardMenu;
use crate::components::globals::StatsAside;
use crate::components::globals::pagination::{PaginationControls, PaginationState};
use crate::components::globals::toggle::{Toggle, ToggleColor, ToggleSize};
use crate::components::{
    StoredPasswordDeletionDialog, StoredPasswordUpsertDialog, StoredRawPasswordsTable,
    show_toast_error, use_toast,
};
use custom_errors::DBError;
use dioxus::prelude::*;
use pwd_types::{PasswordStats, PasswordStrength, StoredRawPassword};
use sqlx::SqlitePool;
use std::ops::Deref;

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
    #[allow(unused_mut)]
    let mut deletion_password_dialog_state =
        use_context_provider(|| DeleteStoredPasswordDialogState {
            is_open: Signal::new(false),
            password_id: Signal::new(Option::<i64>::None),
        });
    // DATA
    let pool = use_context::<SqlitePool>();
    let pool_for_page = pool.clone();
    let pool_for_stats = pool.clone();
    let pool_for_delete = pool.clone();
    let mut error = use_signal(|| <Option<DBError>>::None);
    let user_id_option = auth_state.user.cloned().map(|u| u.id);
    let toast = use_toast();

    // SIGNALS
    let mut unlock_locations = use_signal(|| false);
    let mut unlock_passwords = use_signal(|| false);
    // Estrae user_id
    let user_id = user_id_option.unwrap_or(-1);

    // Stato paginazione
    let mut pagination = use_context_provider(|| PaginationState::new());

    // Resource per pagina corrente
    let mut password_page_data = use_resource(move || {
        let pool = pool.clone();
        let page = pagination.current_page();
        let filter = pagination.active_filter();
        let page_size = pagination.page_size();
        async move {
            if user_id == -1 {
                return None;
            }
            // Controlla cache
            if let Some(cached) = pagination.get_current_page_from_cache() {
                return Some(cached);
            }
            pagination.is_loading.set(true);
            let result =
                get_stored_raw_passwords_paginated(&pool, user_id, filter, page, page_size).await;
            pagination.is_loading.set(false);
            match result {
                Ok((passwords, total)) => {
                    pagination.total_count.set(total);
                    pagination.cache_page(filter, page, passwords.clone());
                    Some(passwords)
                }
                Err(e) => {
                    error.set(Some(e));
                    None
                }
            }
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
        let mut pagination = pagination.clone();
        let mut stats_data = stats_data.clone();
        let mut password_page_data = password_page_data.clone();
        move |_| {
            pagination.invalidate();
            stats_data.restart();
            password_page_data.restart();
        }
    };

    // deletion modal
    let on_confirm_delete = {
        let pool = pool_for_delete.clone();
        let mut pagination = pagination.clone();
        let mut stats_data = stats_data.clone();
        let mut password_page_data = password_page_data.clone();
        let mut deletion_password_dialog_state = deletion_password_dialog_state.clone();
        let mut error = error.clone();

        move |_| {
            let pool = pool.clone();
            let mut delete_state = deletion_password_dialog_state.clone();
            let mut error_signal = error.clone();
            let mut pagination = pagination.clone();
            let mut stats_data = stats_data.clone();
            let mut password_page_data = password_page_data.clone();

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
                        pagination.invalidate();
                        stats_data.restart();
                        password_page_data.restart();
                        delete_state.is_open.set(false);
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
        let mut pagination = pagination.clone();
        let mut stats_data = stats_data.clone();
        let mut password_page_data = password_page_data.clone();
        if need_restart() {
            pagination.invalidate();
            stats_data.restart();
            password_page_data.restart();
            need_restart.set(false);
        }
    });

    rsx! {
        // Stats Aside - posizionato fixed con z-index alto
        StatsAside {
            stats: stats(),
            on_stat_click: move |strength| {
                pagination.set_filter(strength);
                password_page_data.restart();
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

            div { class: "flex flex-row gap-3 mb-4 justify-end align-center",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text strong", "View Locations" }
                    }
                    // Toggle con dimensione e colore personalizzati
                    Toggle {
                        checked: unlock_locations(),
                        onchange: move |_| unlock_locations.toggle(),
                        size: ToggleSize::Small,
                        color: ToggleColor::Success,
                        disabled: false,
                    }
                }

                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text strong", "View Passwords" }
                    }
                    // Toggle con dimensione e colore personalizzati
                    Toggle {
                        checked: unlock_passwords(),
                        onchange: move |_| unlock_passwords.toggle(),
                        size: ToggleSize::Small,
                        color: ToggleColor::Success,
                        disabled: false,
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
                let table_data: Option<Vec<StoredRawPassword>> = password_page_data
                    .read()
                    .clone()
                    .flatten();
                let count = table_data.as_ref().map(|p| p.len()).unwrap_or(0);
                rsx! {
                    div { class: "card card-lg",
                        StoredRawPasswordsTable { key: "{count}", data: table_data }
                    }
                }
            }

            // Controlli paginazione
            PaginationControls {
                pagination: pagination.clone(),
                on_page_change: move |new_page| {
                    pagination.go_to_page(new_page);
                    password_page_data.restart();
                },
            }
        }
        // on_cancel gestito internamente al componente
        StoredPasswordUpsertDialog { on_confirm: on_confirm_upsert, on_cancel: move |_| {} }
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
