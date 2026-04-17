// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::{AuthState, User};
use crate::backend::avatar_utils::get_user_avatar_with_default;
use crate::backend::db_backend::{
    delete_user, fetch_user_data, register_user_with_settings, save_or_update_user,
};
use crate::backend::ui_utils::pick_and_process_avatar;
use crate::backend::vault_utils::create_vault;
use crate::components::{
    ActionButton, AvatarSelector, AvatarSize, ButtonSize, ButtonType, ButtonVariant,
    MigrationProgressDialog, MigrationWarningDialog, PasswordHandler, UserDeletionDialog,
    schedule_toast_success, show_toast_error, show_toast_success, use_toast,
};
use dioxus::prelude::*;
use pwd_dioxus::InputType;

#[cfg(feature = "desktop")]
use pwd_dioxus::Toggle;

#[cfg(feature = "desktop")]
use crate::backend::db_backend::{get_auto_login_user, set_auto_login_enabled};
#[cfg(feature = "desktop")]
use crate::backend::hello_auth;
use pwd_dioxus::form::FormField;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;

use pwd_types::{PasswordChangeResult, PasswordPreset};
use tracing::instrument;

#[component]
#[instrument]
pub fn UpsertUser(user_to_edit: Option<User>) -> Element {
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let pool_clone_on_submit = pool.clone();
    let toast = use_toast();
    let auth_state = use_context::<AuthState>();
    #[allow(unused_mut)]
    let mut auth_state_delete_clone = auth_state.clone();
    #[allow(unused_mut)]
    let mut auth_state_logout_clone = auth_state.clone(); // Per use_effect migrazione
    #[allow(unused_mut)]
    let mut auth_state_delete_logout_clone = auth_state.clone(); // Per confirm_delete_user
    #[allow(unused_mut)]
    let mut auth_state_normal_submit_clone = auth_state.clone();
    let auth_state_autologin_clone = auth_state.clone(); // Per login diretto dopo registrazione

    // --- Stato ---
    #[allow(unused_mut)]
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    #[allow(unused_mut)]
    let mut new_avatar = use_signal(|| None::<Vec<u8>>);
    #[allow(unused_mut)]
    let mut is_user_deleted = use_signal::<bool>(|| false);
    #[allow(unused_mut)]
    let mut is_picking = use_signal(|| false); // Traccia se il dialog è aperto
    let mut show_delete_modal = use_signal(|| false);
    let mut show_warning_modal = use_signal(|| false);
    let show_migration_progress_modal = use_signal(|| false);
    let mut migration_completed = use_signal(|| false);
    #[allow(unused_mut)]
    let mut migration_failed = use_signal(|| false);
    let submit_completed = use_signal(|| false); // Per il caso senza migrazione
    let save_completed_for_migration = use_signal(|| false); // Per sincronizzare modale migrazione
    let mut auto_login_enabled = use_signal(|| false);
    #[allow(unused_mut)]
    let mut auto_login_data = use_signal(|| Option::<(String, i64)>::None);

    // Inizializzazione dati utente (Semplificata con unwrap_or_default)
    #[allow(unused_mut)]
    let mut username = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default()
    });
    #[allow(unused_mut)]
    let mut evaluated_password = use_signal(|| Option::<PasswordChangeResult>::None);
    let mut avatar = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.avatar.clone())
            .unwrap_or_else(|| get_user_avatar_with_default(None))
    });

    // --- Derivazione Proprietà (Configurazione UI) ---
    let is_updating = user_to_edit.is_some();
    let user_id = user_to_edit.as_ref().map(|u| u.id);
    let migration_data_context =
        use_context_provider(|| Signal::new(MigrationData::new(user_id, None)));
    let (
        header,
        paragraph,
        class_container,
        submit_btn_text,
        password_required,
        btn_container_class,
    ) = if is_updating {
        (
            "Account Settings",
            "Update Your Profile",
            "auth-form-tabbed futuristic w-full",
            "Update",
            false,
            "flex flex-col justify-end gap-2",
        )
    } else {
        (
            "Create Account",
            "Sign up to get started",
            "auth-form-lg futuristic w-full",
            "Register",
            true,
            "flex flex-col gap-2 w-full",
        )
    };
    // --- Effetti ---
    // Aggiorna l'anteprima avatar quando ne viene scelto uno nuovo
    use_effect(move || {
        if let Some(img) = new_avatar.read().clone() {
            avatar.set(get_user_avatar_with_default(Some(img)));
        }
    });

    use_effect(move || {
        let mut this_error = error;
        let toast = toast;
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error saving user: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Gestione errori centralizzata
    use_effect(move || {
        let user = auth_state_delete_clone.get_user();
        let mut user_deleted = is_user_deleted;
        let toast = toast;
        if user_deleted() {
            if let Some(u) = user {
                show_toast_success(format!("User {} deleted successfully!", u.username), toast);
            }
            user_deleted.set(false);
        }
    });

    // Gestione completamento migrazione password
    use_effect(move || {
        let mut show_progress = show_migration_progress_modal;
        let mut auth_state = auth_state_logout_clone.clone();
        let mut migration_data = migration_data_context;
        if migration_completed() {
            migration_data.write().old_password = None;
            show_progress.set(false);
            auth_state.logout();
            nav.push("/login");
        }
    });

    // Apertura modale migrazione dopo salvataggio completato
    use_effect(move || {
        let mut show_progress = show_migration_progress_modal;
        let mut save_signal = save_completed_for_migration;
        if save_signal() {
            // Reset del signal per permettere future migrazioni
            save_signal.set(false);
            migration_completed.set(false);
            // Apri il modale - ora il context è popolato con old_password
            show_progress.set(true);
        }
    });

    // Gestione completamento submit normale (senza migrazione)
    use_effect(move || {
        let mut auth_state = auth_state_normal_submit_clone.clone();
        if submit_completed() {
            auth_state.logout();
            nav.push("/login");
        }
    });

    // Gestione login diretto dopo registrazione con auto-login
    #[cfg(feature = "desktop")]
    let pool_for_autologin = pool.clone();
    #[cfg(feature = "desktop")]
    let nav_for_autologin = nav.clone();
    #[cfg(feature = "desktop")]
    let toast_for_autologin = toast;

    #[cfg(feature = "desktop")]
    use_effect(move || {
        let mut auth_state = auth_state_autologin_clone.clone();
        if let Some((username, user_id)) = auto_login_data() {
            auto_login_data.set(None);
            let pool = pool_for_autologin.clone();
            let toast = toast_for_autologin.clone();
            spawn(async move {
                match fetch_user_data(&pool, &username).await {
                    Ok((_id, uname, created_at, avatar)) => {
                        auth_state.login(user_id, uname, created_at, avatar);
                        nav_for_autologin.push("/dashboard");
                    }
                    Err(e) => {
                        show_toast_error(format!("Errore: {}", e), toast);
                        auth_state.logout();
                        nav_for_autologin.push("/login");
                    }
                }
            });
        }
    });

    // --- Handlers ---
    let on_auto_login_toggle = move |_| {
        #[cfg(feature = "desktop")]
        if !auto_login_enabled() {
            let mut auto_login_enabled = auto_login_enabled;
            let toast = toast;
            spawn(async move {
                let result = tokio::task::spawn_blocking(|| {
                    hello_auth::request_verification("Verifica identità per attivare auto-login")
                })
                .await
                .unwrap_or(hello_auth::HelloResult::Failed("Task spawn fallito".into()));

                match result {
                    hello_auth::HelloResult::Success => {
                        auto_login_enabled.set(true);
                    }
                    hello_auth::HelloResult::Cancelled => {
                        show_toast_error("Auto-login annullato".to_string(), toast);
                    }
                    hello_auth::HelloResult::NotEnrolled => {
                        show_toast_error("Windows Hello non è configurato. Configuralo nelle Impostazioni di Windows.".to_string(), toast);
                    }
                    hello_auth::HelloResult::Failed(msg) => {
                        show_toast_error(format!("Autenticazione fallita: {}", msg), toast);
                    }
                    hello_auth::HelloResult::NotAvailable => {
                        show_toast_error(
                            "Windows Hello non è disponibile su questo dispositivo".to_string(),
                            toast,
                        );
                    }
                }
            });
        } else {
            #[cfg(feature = "desktop")]
            auto_login_enabled.set(false);
        }
    };

    let pick_image = move |_| {
        // Controllo doppio: previene click se già caricando o picking
        if is_loading() || is_picking() {
            return;
        }
        #[allow(unused_mut)]
        let mut new_avatar_clone = new_avatar;
        #[allow(unused_mut)]
        let mut is_loading_clone = is_loading;
        #[allow(unused_mut)]
        let mut is_picking_clone = is_picking; // Clona anche is_picking
        #[allow(unused_mut)]
        let mut error_clone = error;
        spawn(pick_and_process_avatar(
            new_avatar_clone,
            is_loading_clone,
            is_picking_clone, // ← Passa il nuovo signal
            error_clone,
        ));
    };

    // Apre il modal di conferma
    let on_delete_click = move |_| {
        show_delete_modal.set(true);
    };

    // Esegue la cancellazione vera e propria (chiamata dal modal)
    let confirm_delete_user = move || {
        let mut is_user_deleted = is_user_deleted;
        let pool_for_delete = pool.clone();
        let user = auth_state.get_user();
        let mut error = error;
        let mut auth_state_logout = auth_state_delete_logout_clone.clone();
        let mut show_modal = show_delete_modal;

        if let Some(user) = user {
            spawn(async move {
                match delete_user(&pool_for_delete, user.id).await {
                    Ok(()) => {
                        is_user_deleted.set(true);
                        auth_state_logout.logout();
                        show_modal.set(false);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        show_modal.set(false);
                    }
                }
            });
        }
    };
    // Chiude il modal senza cancellare
    let cancel_delete = move |_| {
        show_delete_modal.set(false);
    };

    // Closure che esegue il submit effettivo (riutilizzabile)
    // NOTA: Non fa logout né naviga - devono essere gestiti dal chiamante
    // Accetta un segnale opzionale da impostare quando il salvataggio ha successo
    let execute_submit = move |completion_signal: Option<Signal<bool>>| {
        let pwd_result = evaluated_password.read().clone();
        let u = username.read().clone();
        let a = new_avatar.read().clone();
        let pool = pool_clone_on_submit.clone();
        let mut migration_data = migration_data_context;
        // In modalità update: se password vuota o None → mantieni password attuale
        let password_to_save = pwd_result.and_then(|result| {
            if result.password.expose_secret().trim().is_empty() {
                None // Password vuota → non cambiare
            } else {
                Some(result.password) // Password presente → aggiorna
            }
        });
        spawn(async move {
            // Branch separato per registrazione vs update
            #[cfg(feature = "desktop")]
            let mut auto_login_data = auto_login_data;
            let success = if user_id.is_none() {
                // REGISTRAZIONE: usa la funzione atomica
                #[cfg(feature = "desktop")]
                let auto_login_pwd = password_to_save.clone();

                match register_user_with_settings(
                    &pool,
                    u.clone(),
                    password_to_save,
                    a,
                    PasswordPreset::God,
                )
                .await
                {
                    Ok(saved_user_id) => {
                        // Auto-create Default vault for new user
                        if create_vault(&pool, saved_user_id, "Default".to_string(), None)
                            .await
                            .is_ok()
                        {
                            schedule_toast_success(
                                "User Registered successfully!".to_string(),
                                toast,
                            );
                        }

                        // Store auto-login if enabled
                        #[cfg(feature = "desktop")]
                        if auto_login_enabled() {
                            if let Some(ref pwd) = auto_login_pwd {
                                if let Err(e) =
                                    hello_auth::store_master_password(&u, pwd.expose_secret())
                                {
                                    show_toast_error(
                                        format!("Impossibile salvare auto-login: {}", e),
                                        toast,
                                    );
                                }
                                if let Err(e) = set_auto_login_enabled(&pool, &u, true).await {
                                    show_toast_error(
                                        format!("Impossibile attivare auto-login: {}", e),
                                        toast,
                                    );
                                }
                            }
                            // Direct login — skip login page
                            auto_login_data.set(Some((u.clone(), saved_user_id)));
                        }

                        true
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        false
                    }
                }
            } else {
                // UPDATE: usa la funzione esistente
                let password_to_save_for_keyring = password_to_save.clone();
                match save_or_update_user(&pool, user_id, u.clone(), password_to_save, a).await {
                    Ok(result) => {
                        info!("User updated successfully: {:?}", result);
                        schedule_toast_success("User Updated successfully!".to_string(), toast);
                        (migration_data.write()).old_password = result.temp_old_password.clone();

                        // Update keyring if user has auto-login enabled and changed password
                        #[cfg(feature = "desktop")]
                        if password_to_save_for_keyring.is_some() {
                            let username_clone = u.clone();
                            match get_auto_login_user(&pool).await {
                                Ok(Some(auto_user)) if auto_user == username_clone => {
                                    if let Some(ref pwd) = password_to_save_for_keyring {
                                        if let Err(e) = hello_auth::store_master_password(
                                            &username_clone,
                                            pwd.expose_secret(),
                                        ) {
                                            tracing::warn!(
                                                "Impossibile aggiornare master password nel keyring: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        true
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                        false
                    }
                }
            };

            // Se il salvataggio ha successo e c'è un segnale di completamento, impostalo
            // (skip se auto-login direct login è in corso)
            #[cfg(feature = "desktop")]
            let auto_login_active = auto_login_data().is_some();
            #[cfg(not(feature = "desktop"))]
            let auto_login_active = false;

            if success && let Some(mut signal) = completion_signal {
                if !auto_login_active {
                    signal.set(true);
                }
            }
        });
    };

    // Handler per conferma del warning - procede con il submit (con migrazione password)
    let confirm_change_password = {
        let execute_submit = execute_submit.clone();
        move |_: ()| {
            show_warning_modal.set(false);
            // Passa il signal - il modale si aprirà quando il salvataggio completa
            // e il context sarà popolato con old_password
            execute_submit(Some(save_completed_for_migration));
        }
    };

    // Handler per annullamento del warning
    let cancel_migration = move |_: ()| {
        show_warning_modal.set(false);
    };

    let on_submit = move |_| {
        // In modalità update, evaluated_password può essere None (password non cambiata)
        let pwd_result = evaluated_password.read().clone();

        // Per la registrazione, la password deve essere validata
        if !is_updating && pwd_result.is_none() {
            error.set(Some("Please complete password validation".to_string()));
            return;
        }

        // Per la registrazione, la password non può essere vuota
        if let Some(ref result) = pwd_result
            && !is_updating
            && result.password.expose_secret().trim().is_empty()
        {
            error.set(Some("Password is required for registration".to_string()));
            return;
        }

        // Determina se c'è una password nuova (non vuota) da salvare
        let has_new_password = pwd_result
            .as_ref()
            .map(|r| !r.password.expose_secret().trim().is_empty())
            .unwrap_or(false);

        // In modalità update con password compilata: mostra warning prima di procedere
        if is_updating && has_new_password {
            show_warning_modal.set(true);
            return; // Non procedere con il submit, aspetta conferma utente
        }

        // Altrimenti procedi normalmente (senza migrazione password)
        execute_submit(Some(submit_completed));
    };

    rsx! {
        div { class: "page-centered",
            div { class: "{class_container} animate-scale-in",
                h1 { class: "text-h3 text-center", "{header}" }
                p { class: "text-body mb-4 text-center", "{paragraph}" }

                AvatarSelector {
                    avatar_src: avatar.read().clone(),
                    on_pick: pick_image,
                    button_text: "Select Avatar",
                    size: AvatarSize::XXLarge,
                    shadow: true,
                    show_border: true,
                    loading: is_loading,
                    is_picking, // ← Passa il signal per disabilitare il bottone
                }

                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    FormField {
                        label: "Username",
                        input_type: InputType::Text,
                        placeholder: "Choose a username",
                        value: username,
                        required: true,
                        alphanumeric_only: true,
                    }
                    PasswordHandler {
                        on_password_change: move |pwd| {
                            evaluated_password.set(Some(pwd));
                        },
                        password_required,
                    }
                    AutoLoginToggle {
                        is_updating,
                        auto_login_enabled,
                        on_toggle: on_auto_login_toggle,
                    }
                    div { class: "{btn_container_class}",
                        ActionButton {
                            text: "{submit_btn_text}",
                            variant: ButtonVariant::Success,
                            button_type: ButtonType::Submit,
                            size: ButtonSize::Normal,
                            on_click: move |_| {},
                            disabled: is_loading,
                        }
                        if is_updating {
                            ActionButton {
                                text: "Delete Account",
                                variant: ButtonVariant::Ghost,
                                button_type: ButtonType::Button,
                                size: ButtonSize::Normal,
                                on_click: on_delete_click,
                                additional_class: "text-error hover:bg-error/10",
                            }
                        } else {
                            ActionButton {
                                text: "Login",
                                variant: ButtonVariant::Secondary,
                                button_type: ButtonType::Button,
                                size: ButtonSize::Normal,
                                on_click: move |_| {
                                    nav.push("/login");
                                },
                            }
                        }
                    }
                }
            }
        }

        // UserDeletionDialog
        UserDeletionDialog {
            open: show_delete_modal,
            on_confirm: move |_| confirm_delete_user(),
            on_cancel: cancel_delete,
            username: username.read().clone(),
        }
        MigrationWarningDialog {
            open: show_warning_modal,
            on_confirm: confirm_change_password,
            on_cancel: cancel_migration,
        }

        MigrationProgressDialog {
            open: show_migration_progress_modal,
            on_completed: migration_completed,
            on_failed: migration_failed,
            on_cancel: move |_| {},
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct MigrationData {
    pub user_id: Option<i64>,
    pub old_password: Option<String>,
}

impl MigrationData {
    pub fn new(user_id: Option<i64>, old_password: Option<String>) -> Self {
        Self {
            user_id,
            old_password,
        }
    }
}

#[cfg(feature = "desktop")]
#[component]
fn AutoLoginToggle(
    is_updating: bool,
    auto_login_enabled: Signal<bool>,
    on_toggle: EventHandler<()>,
) -> Element {
    let hello_available = use_signal(|| hello_auth::is_hello_available());
    if !is_updating {
        rsx! {
            div { class: "flex items-center justify-between gap-4 mt-2",
                label { class: "text-sm font-medium", "Auto-login con Windows Hello" }
                Toggle {
                    checked: auto_login_enabled(),
                    onchange: move |_| on_toggle(()),
                }
            }
            if !hello_available() {
                p { class: "text-xs text-warning mt-1",
                    "Windows Hello non è disponibile su questo dispositivo"
                }
            }
        }
    } else {
        Ok(VNode::placeholder())
    }
}

#[cfg(not(feature = "desktop"))]
#[component]
fn AutoLoginToggle(
    is_updating: bool,
    auto_login_enabled: Signal<bool>,
    on_toggle: EventHandler<()>,
) -> Element {
    Ok(VNode::placeholder())
}
