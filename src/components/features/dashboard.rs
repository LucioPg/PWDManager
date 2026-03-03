use crate::backend::db_backend::delete_stored_password;
use crate::backend::password_utils::get_stored_raw_passwords;
use crate::components::features::DashboardMenu;
use crate::components::globals::{StatCard, StatVariant};
use crate::components::{
    StoredPasswordDeletionDialog, StoredPasswordUpsertDialog, StoredRawPasswordsTable,
    show_toast_error, use_toast,
};
use custom_errors::DBError;
use dioxus::prelude::*;
use pwd_types::{PasswordScore, PasswordStats, PasswordStrength, StoredRawPassword};
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
    let pool_clone = pool.clone();
    let mut error = use_signal(|| <Option<DBError>>::None);
    let user_id_option = auth_state.user.cloned().map(|u| u.id);
    let toast = use_toast();

    let stored_raw_passwords_data = use_resource(move || {
        let pool_clone = pool.clone();
        async move {
            let user_id = user_id_option.unwrap_or_else(|| {
                error.set(Some(DBError::new_select_error("User not logged in".into())));
                return -1;
            });
            if user_id == -1 {
                return None;
            }
            let result = get_stored_raw_passwords(&pool_clone, user_id).await;
            match result {
                Ok(passwords) => Some(passwords),
                Err(e) => {
                    error.set(Some(e));
                    None
                }
            }
        }
    });

    // stored raw passwords

    #[allow(unused_mut)]
    let current_filter = use_signal(|| <Option<PasswordStrength>>::None);

    let stats = use_memo(move || {
        let mut stats_ = PasswordStats::default();
        if let Some(Some(list)) = &*stored_raw_passwords_data.read() {
            for p in list {
                let strength = PasswordScore::get_strength(p.score.map(|s| s.value() as i64));
                match strength {
                    PasswordStrength::WEAK => stats_.weak += 1,
                    PasswordStrength::MEDIUM => stats_.medium += 1,
                    PasswordStrength::STRONG => stats_.strong += 1,
                    PasswordStrength::EPIC => stats_.epic += 1,
                    PasswordStrength::GOD => stats_.god += 1,
                    PasswordStrength::NotEvaluated => stats_.not_evaluated += 1,
                }
                stats_.total += 1;
            }
        }
        stats_
    });

    let filtered_stored_raw_passwords = use_memo(move || {
        let data = stored_raw_passwords_data.read();
        let active_filter = current_filter();
        // Invece di fare l'if let qui, mappiamo il contenuto del segnale
        // Questo restituirà Some(Vec) se i dati sono pronti, None altrimenti
        data.as_ref()
            .and_then(|inner_option| inner_option.as_ref())
            .map(|list| match active_filter {
                None => list.clone(),
                Some(target_strength) => list
                    .iter()
                    .filter(|p| {
                        let current_strength =
                            PasswordScore::get_strength(p.score.map(|s| s.value() as i64));
                        target_strength == current_strength
                    })
                    .cloned()
                    .collect(),
            })
    });

    // upsert modal - refresh tabella dopo salvataggio
    let on_confirm_upsert = {
        let stored_raw_passwords_data = stored_raw_passwords_data.clone();
        move |_| {
            let mut resource = stored_raw_passwords_data.clone();
            spawn(async move {
                resource.restart();
            });
        }
    };

    // deletion modal
    let on_confirm_delete = {
        // Cattura tutto quello che serve (già clonato)
        let pool = pool_clone.clone();
        let stored_raw_passwords_data = stored_raw_passwords_data.clone();
        let mut deletion_password_dialog_state = deletion_password_dialog_state.clone();
        let mut error = error.clone();

        move |_| {
            // Clona PER OGNI invocazione (rende la closure FnMut)
            let pool = pool.clone();
            let mut resource = stored_raw_passwords_data.clone();
            let mut delete_state = deletion_password_dialog_state.clone();
            let mut error_signal = error.clone();

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
                        resource.restart();
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
        let mut resource = stored_raw_passwords_data.clone();
        if need_restart() {
            resource.restart();
            need_restart.set(false);
        }
    });

    rsx! {
        div { class: "content-container animate-fade-in",
            div { class: "mb-8 flex justify-between items-start",
                div {
                    h1 { class: "text-h2", "Welcome, {username}!" }
                    p { class: "text-body mt-2", "Manage your passwords and secure your digital life" }
                }
                DashboardMenu { on_need_restart: on_need_restart.clone() }
            }
            div { class: "stats-grid",
                StatCard {
                    value: stats().total.to_string(),
                    label: "Total Passwords".to_string(),
                    variant: StatVariant::Primary,
                    on_click: move |_| current_filter.clone().set(None),
                }
                StatCard {
                    value: stats().god.to_string(),
                    label: "God Passwords".to_string(),
                    variant: StatVariant::Success,
                    on_click: move |_| current_filter.clone().set(Some(PasswordStrength::GOD)),
                }
                StatCard {
                    value: stats().epic.to_string(),
                    label: "Epic Passwords".to_string(),
                    variant: StatVariant::Success,
                    on_click: move |_| current_filter.clone().set(Some(PasswordStrength::EPIC)),
                }
                StatCard {
                    value: stats().strong.to_string(),
                    label: "Strong Passwords".to_string(),
                    variant: StatVariant::Success,
                    on_click: move |_| current_filter.clone().set(Some(PasswordStrength::STRONG)),
                }
                StatCard {
                    value: stats().medium.to_string(),
                    label: "Medium Passwords".to_string(),
                    variant: StatVariant::Warning,
                    on_click: move |_| current_filter.clone().set(Some(PasswordStrength::MEDIUM)),
                }
                StatCard {
                    value: stats().weak.to_string(),
                    label: "Weak Passwords".to_string(),
                    variant: StatVariant::Warning,
                    on_click: move |_| current_filter.clone().set(Some(PasswordStrength::WEAK)),
                }
            }
            div { class: "card-no-border items-end",
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
                let table_data = filtered_stored_raw_passwords();
                let count = table_data.as_ref().map(|p| p.len()).unwrap_or(0);
                rsx! {
                    div { class: "card card-lg",
                        StoredRawPasswordsTable { key: "{count}", data: table_data }
                    }
                }
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
