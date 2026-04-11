// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::Route;
use crate::backend::db_backend::{
    delete_stored_password, fetch_password_stats_for_vault, move_passwords_to_vault,
};
use crate::backend::password_utils::{
    clone_passwords_to_vault, get_all_stored_raw_passwords_for_vault_with_filter,
};
use crate::components::globals::StatsAside;
use crate::components::globals::pagination::{PaginationControls, PaginationState};
use crate::components::globals::spinner::{Spinner, SpinnerSize};
use crate::components::globals::types::TableOrder;
use crate::components::globals::{ActiveVaultState, VaultListState};
use crate::components::{
    BulkActionBar, StoredPasswordDeletionDialog, StoredPasswordShowDialog,
    StoredPasswordUpsertDialog, StoredRawPasswordsTable, VaultAction, VaultActionDialog,
    show_toast_error, use_toast,
};
use custom_errors::DBError;
use dioxus::prelude::*;
use pwd_dioxus::{Combobox, ComboboxSize};
use pwd_types::StoredRawPassword;
use sqlx::SqlitePool;
use std::collections::HashSet;
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
    let username = auth_state.get_username();
    let mut stored_password_dialog_state =
        use_context_provider(|| StoredPasswordUpsertDialogState {
            is_open: Signal::new(false),
            current_stored_raw_password: Signal::new(None::<StoredRawPassword>),
        });
    let _stored_password_show_dialog_state =
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
    let pool_for_move = pool.clone();
    let mut error = use_signal(|| <Option<DBError>>::None);
    let user_id_option = auth_state.user.cloned().map(|u| u.id);
    let toast = use_toast();
    let options = table_order_options();

    // Ordinamento: default Newest (coincide con ORDER BY created_at DESC del DB)
    let mut current_table_order = use_signal(|| Some(TableOrder::Newest));

    // Dati completi per ordinamento frontend
    #[allow(clippy::redundant_closure)]
    let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());

    // Search query per filtro client-side
    #[allow(clippy::redundant_closure)]
    let mut search_query = use_signal(|| String::new());

    // Estrae user_id
    let user_id = user_id_option.unwrap_or(-1);

    // Vault state
    let mut active_vault_state = use_context::<ActiveVaultState>();
    let active_vault_id = active_vault_state.0;
    println!("initial active vault id {:#?}", active_vault_id(),);
    // Vault list resource (shared via VaultListState from AuthWrapper)
    #[allow(unused_mut)]
    let mut vaults_resource = use_context::<VaultListState>().0;

    // Vault combobox options
    let vault_options = use_memo(move || {
        let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        let opts: Vec<(&'static str, Option<i64>)> = vaults
            .iter()
            .map(|v| {
                // Leaked because the Combobox expects &'static str
                let name = Box::leak(v.name.clone().into_boxed_str()) as &'static str;
                (name, Some(v.id.unwrap_or(0)))
            })
            .collect();
        opts
    });

    // Stato paginazione
    #[allow(clippy::redundant_closure)]
    let mut pagination = use_context_provider(|| PaginationState::new());

    // Multi-select state for password table
    let mut selected_ids: Signal<HashSet<i64>> = use_signal(HashSet::new);

    // Dialog open states for bulk actions
    let mut vault_action_dialog_open = use_signal(|| false);
    let mut current_vault_action = use_signal(|| VaultAction::Move);

    // Reactive signal for vault combobox disabled state
    let mut vaults_empty = use_signal(|| true);
    use_effect(move || {
        let v = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        vaults_empty.set(v.is_empty());
    });

    // Resource per fetch completa (ordinamento delegato al DB)
    // Reagisce a: active_vault_id, current_table_order, pagination.active_filter()
    let mut sorted_passwords_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id;
        let vault_id = active_vault_id().unwrap_or(-1);
        let filter = pagination.active_filter();
        let order_clause = current_table_order()
            .unwrap_or(TableOrder::Newest)
            .order_by_clause();

        async move {
            if user_id == -1 || vault_id == -1 {
                return Vec::new();
            }

            get_all_stored_raw_passwords_for_vault_with_filter(
                &pool,
                user_id,
                vault_id,
                filter,
                order_clause,
            )
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

    // Stats sempre fresche (query separata, vault-scoped)
    let mut stats_data = use_resource(move || {
        let pool = pool_for_stats.clone();
        let vault_id = active_vault_id().unwrap_or(-1);
        async move {
            if vault_id == -1 {
                return None;
            }
            match fetch_password_stats_for_vault(&pool, vault_id).await {
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
    let stats = use_memo(move || stats_data.read().flatten().unwrap_or_default());

    // upsert modal - refresh tabella dopo salvataggio
    let on_confirm_upsert = {
        let mut stats_data = stats_data;
        let mut sorted_passwords_resource = sorted_passwords_resource;
        move |_| {
            stats_data.restart();
            sorted_passwords_resource.restart();
        }
    };

    // deletion modal
    let on_confirm_delete = {
        let pool = pool_for_delete.clone();
        let sorted_passwords_resource = sorted_passwords_resource;
        let mut deletion_password_dialog_state = deletion_password_dialog_state.clone();
        let error = error;

        move |_| {
            let pool = pool.clone();
            let mut delete_state = deletion_password_dialog_state.clone();
            let mut error_signal = error;
            let mut stats_data = stats_data;
            let mut sorted_passwords_resource = sorted_passwords_resource;

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
            show_toast_error(format!("Error fetching user data: {}", e), toast);
        }
    });

    // Restart resources when active vault changes
    use_effect(move || {
        println!("Active vault changed: {:?}", *active_vault_id.read());
        let _ = *active_vault_id.read();
        pagination.go_to_page(0);
        selected_ids.set(HashSet::new());
        sorted_passwords_resource.restart();
        stats_data.restart();
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
            div { class: "mb-8",
                h1 { class: "text-h2", "Welcome, {username}!" }
                p { class: "text-body mt-2", "Manage your passwords and secure your digital life" }
            }
            div { class: "pwd-controls-bar",
                div { class: "pwd-controls-left",
                    // Search input
                    div { class: "pwd-search-wrapper max-w-[150px]",
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
                            aria_label: "Search passwords by name",
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
                                aria_label: "Clear search",
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
                        size: ComboboxSize::Small,
                        on_change: move |v| {
                            current_table_order.set(v);
                            pagination.go_to_page(0);
                            sorted_passwords_resource.restart();
                        },
                    }
                    // Vault selector Combobox
                    {
                        let vault_key = active_vault_id().unwrap_or(-1);
                        let selected = active_vault_id();
                        let is_empty = vaults_empty();
                        rsx! {
                            Combobox::<i64> {
                                key: "{vault_key}",
                                options: vault_options(),
                                placeholder: if is_empty { "Create a vault first".to_string() } else { "Select Vault".to_string() },
                                size: ComboboxSize::Medium,
                                selected_value: selected,
                                disabled: vaults_empty,
                                on_change: move |v| {
                                    println!("Selected vault: {:?}", v);
                                    active_vault_state.0.set(v);
                                },
                            }
                        }
                    }
                }
                button {
                    class: if active_vault_id().is_none() { "btn btn-success btn-disabled" } else { "btn btn-success" },
                    r#type: "button",
                    disabled: active_vault_id().is_none(),
                    onclick: move |_| {
                        stored_password_dialog_state.current_stored_raw_password.set(None);
                        stored_password_dialog_state.is_open.set(true);
                    },
                    "New Password"
                }
            }

            // Bulk action bar - shown when passwords are selected
            {
                let count = selected_ids.read().len();
                if count > 0 {
                    rsx! {
                        BulkActionBar {
                            count,
                            on_move: move |_| {
                                current_vault_action.set(VaultAction::Move);
                                vault_action_dialog_open.set(true);
                            },
                            on_clone: move |_| {
                                current_vault_action.set(VaultAction::Clone);
                                vault_action_dialog_open.set(true);
                            },
                            on_clear: move |_| {
                                selected_ids.set(HashSet::new());
                            },
                        }
                    }
                } else {
                    rsx! {}
                }
            }

            {
                let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
                if vaults.is_empty() {
                    rsx! {
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
                            h3 { class: "text-h3", "Create your first Vault" }
                            p { class: "text-body mt-2 pwd-empty-state-subtitle",
                                "A vault is where your passwords live. Create one to get started."
                            }
                            Link { to: Route::MyVaults, class: "btn btn-primary mt-4", "+ New Vault" }
                        }
                    }
                } else {
                    let table_data = page_data();
                    if sorted_passwords_resource.read().is_none() {
                        rsx! {
                            div { class: "card card-lg",
                                div { class: "flex justify-center py-8",
                                    Spinner { size: SpinnerSize::Medium, color_class: "text-info" }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "card card-lg",
                                StoredRawPasswordsTable {
                                    data: table_data,
                                    selected_ids,
                                    on_select: move |(id, checked)| {
                                        let mut ids = selected_ids.write();
                                        if checked {
                                            ids.insert(id);
                                        } else {
                                            ids.remove(&id);
                                        }
                                    },
                                    on_select_all: move |select_all| {
                                        let mut ids = selected_ids.write();
                                        if select_all {
                                            if let Some(data) = page_data() {
                                                for p in data {
                                                    if let Some(id) = p.id {
                                                        ids.insert(id);
                                                    }
                                                }
                                            }
                                        } else {
                                            ids.clear();
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }

            // Controlli paginazione
            PaginationControls {
                pagination,
                on_page_change: move |new_page| {
                    pagination.go_to_page(new_page);
                    selected_ids.set(HashSet::new());
                },
            }
        }
        // on_cancel gestito internamente al componente
        StoredPasswordUpsertDialog { on_confirm: on_confirm_upsert, on_cancel: move |_| {} }
        StoredPasswordShowDialog { on_confirm: move |_| {}, on_cancel: move |_| {} }
        StoredPasswordDeletionDialog {
            open: deletion_password_dialog_state.is_open,
            on_confirm: on_confirm_delete,
            on_cancel: cancel_delete,
        }
        VaultActionDialog {
            open: vault_action_dialog_open,
            action: current_vault_action(),
            selected_passwords: all_passwords()
                .into_iter()
                .filter(|p| p.id.is_some_and(|id| selected_ids.read().contains(&id)))
                .collect(),
            current_vault_id: active_vault_id().unwrap_or(0),
            on_confirm: move |target_vault_id| {
                let pool = pool_for_move.clone();
                let user_id = user_id;
                let ids: Vec<i64> = selected_ids.read().iter().cloned().collect();
                let mut dialog_open = vault_action_dialog_open;
                let mut sorted_resource = sorted_passwords_resource;
                let mut stats_res = stats_data;
                let mut vaults_res = vaults_resource;
                let action = current_vault_action();
                spawn(async move {
                    let result = if action == VaultAction::Move {
                        move_passwords_to_vault(&pool, ids, target_vault_id).await
                    } else {
                        clone_passwords_to_vault(&pool, user_id, ids, target_vault_id).await
                    };
                    match result {
                        Ok(()) => {
                            selected_ids.set(HashSet::new());
                            dialog_open.set(false);
                            sorted_resource.restart();
                            stats_res.restart();
                            vaults_res.restart();
                        }
                        Err(e) => {
                            let action_word = if action == VaultAction::Move {
                                "move"
                            } else {
                                "clone"
                            };
                            show_toast_error(
                                format!("Failed to {} passwords: {}", action_word, e),
                                toast,
                            );
                        }
                    }
                });
            },
            on_cancel: move |_| {
                vault_action_dialog_open.set(false);
            },
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
