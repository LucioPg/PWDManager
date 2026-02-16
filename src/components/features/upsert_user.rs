use crate::auth::{AuthState, User};
use crate::backend::db_backend::{delete_user, save_or_update_user};
use crate::backend::ui_utils::pick_and_process_avatar;
use crate::backend::utils::get_user_avatar_with_default;
use crate::components::{
    ActionButton, AvatarSelector, AvatarSize, ButtonSize, ButtonType, ButtonVariant, FormField,
    FormSecret, InputType, PasswordHandler, UserDeletionDialog, schedule_toast_success,
    show_toast_error, show_toast_success, use_toast,
};
use dioxus::prelude::*;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;
use tracing::instrument;

// #[derive(Props, Clone, PartialEq, Debug, Default)]
// pub struct UserFormProps {
//     pub user_to_edit: Option<User>,
// }

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
    let mut auth_state_logout_clone = auth_state.clone();
    #[allow(unused_mut)]
    let mut auth_state_submit_clone = auth_state.clone();

    // --- Stato ---
    #[allow(unused_mut)]
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    #[allow(unused_mut)]
    let mut new_avatar = use_signal(|| None::<Vec<u8>>);
    let is_user_deleted = use_signal::<bool>(|| false);
    #[allow(unused_mut)]
    let mut is_picking = use_signal(|| false); // Traccia se il dialog è aperto
    let mut show_delete_modal = use_signal(|| false);

    // Inizializzazione dati utente (Semplificata con unwrap_or_default)
    #[allow(unused_mut)]
    let mut username = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default()
    });
    #[allow(unused_mut)]
    let mut evaluated_password = use_signal(|| Option::<FormSecret>::None);
    let mut avatar = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.avatar.clone())
            .unwrap_or_else(|| get_user_avatar_with_default(None))
    });

    // --- Derivazione Proprietà (Configurazione UI) ---
    let is_updating = user_to_edit.is_some();
    let user_id = user_to_edit.as_ref().map(|u| u.id.clone());

    let (header, paragraph, class_container, submit_btn_text, password_required) = if is_updating {
        (
            "Account Settings",
            "Update Your Profile",
            "auth-form-tabbed",
            "Update",
            false,
        )
    } else {
        (
            "Create Account",
            "Sign up to get started",
            "auth-form-lg",
            "Register",
            true,
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
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error saving user: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Gestione errori centralizzata
    use_effect(move || {
        let user = auth_state_delete_clone.get_user();
        let mut user_deleted = is_user_deleted.clone();
        let toast = toast.clone();
        if user_deleted() {
            if let Some(u) = user {
                show_toast_success(format!("User {} deleted successfully!", u.username), toast);
            }
            user_deleted.set(false);
        }
    });

    // --- Handlers ---
    let pick_image = move |_| {
        // Controllo doppio: previene click se già caricando o picking
        if is_loading() || is_picking() {
            return;
        }
        #[allow(unused_mut)]
        let mut new_avatar_clone = new_avatar.clone();
        #[allow(unused_mut)]
        let mut is_loading_clone = is_loading.clone();
        #[allow(unused_mut)]
        let mut is_picking_clone = is_picking.clone(); // Clona anche is_picking
        #[allow(unused_mut)]
        let mut error_clone = error.clone();
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
        let mut is_user_deleted = is_user_deleted.clone();
        let pool_for_delete = pool.clone();
        let user = auth_state.get_user();
        let mut error = error.clone();
        let mut auth_state_logout_clone = auth_state_logout_clone.clone();
        let mut show_modal = show_delete_modal.clone();

        match user {
            Some(user) => {
                spawn(async move {
                    match delete_user(&pool_for_delete, user.id).await {
                        Ok(()) => {
                            is_user_deleted.set(true);
                            auth_state_logout_clone.logout();
                            show_modal.set(false);
                        }
                        Err(e) => {
                            error.set(Some(e.to_string()));
                            show_modal.set(false);
                        }
                    }
                });
            }
            None => println!("No user to delete"),
        }
    };

    // Chiude il modal senza cancellare
    let cancel_delete = move |_| {
        show_delete_modal.set(false);
    };
    let on_submit = move |_| {
        let pwd = match evaluated_password.read().clone() {
            Some(p) => p,
            None => {
                error.set(Some("Please complete password validation".to_string()));
                return;
            }
        };

        // Validate password is not empty for registration (CREATE mode)
        if !is_updating && pwd.expose_secret().trim().is_empty() {
            error.set(Some("Password is required for registration".to_string()));
            return;
        }

        let u = username.read().clone();
        let a = new_avatar.read().clone();
        let pool = pool_clone_on_submit.clone();
        let mut auth_state = auth_state_submit_clone.clone();

        // For update mode, allow empty password
        if is_updating && pwd.expose_secret().trim().is_empty() {
            // Password not being changed, continue with update
            spawn(async move {
                match save_or_update_user(&pool, user_id, u, None, a).await {
                    Ok(_) => {
                        auth_state.logout();
                        let message = "User Updated successfully!";
                        schedule_toast_success(message.to_string(), toast);
                        nav.push("/login");
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
            return;
        }

        spawn(async move {
            match save_or_update_user(&pool, user_id, u, Some(pwd.0), a).await {
                Ok(_) => {
                    auth_state.logout();
                    let message = if is_updating {
                        "User Updated successfully!"
                    } else {
                        "User Registered successfully!"
                    };
                    schedule_toast_success(message.to_string(), toast);
                    nav.push("/login");
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
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
                    is_picking: is_picking,  // ← Passa il signal per disabilitare il bottone
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
                        password_required: password_required,
                    }
                    // ActionButtons {
                    //     primary_text: "{submit_btn_text}",
                    //     secondary_text: "Login",
                    //     primary_on_click: move |_| {},
                    //     secondary_on_click: move |_| { nav.push("/login"); },
                    //     variant: ActionButtonsVariant::Auth,
                    // }
                    ActionButton {
                        text: "{submit_btn_text}",
                        variant: ButtonVariant::Primary,
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
                            additional_class: "text-error-600 hover:bg-error-50 hover:text-error-700"
                        }
                    }
                    else {
                        ActionButton {
                            text: "Login",
                            variant: ButtonVariant::Secondary,
                            button_type: ButtonType::Button,
                            size: ButtonSize::Normal,
                            on_click: move |_| { nav.push("/login"); },
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
    }
}
