use crate::auth::{AuthState, User};
use crate::backend::db_backend::save_or_update_user;
use crate::backend::ui_utils::pick_and_process_avatar;
use crate::backend::utils::get_user_avatar_with_default;
use crate::components::{
    ActionButtons, ActionButtonsVariant, AvatarSelector, AvatarSize, FormField, InputType,
    ToastType, ToastsState, add_toast,
};
use dioxus::prelude::*;
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
    let mut toast_state = use_context::<Signal<ToastsState>>();
    let auth_state = use_context::<AuthState>();
    // --- Stato ---
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let mut new_avatar = use_signal(|| None::<Vec<u8>>);

    // Inizializzazione dati utente (Semplificata con unwrap_or_default)
    let mut username = use_signal(|| {
        user_to_edit.as_ref().map(|u| u.username.clone()).unwrap_or_default()
    });
    let mut password = use_signal(|| String::new());
    let mut repassword = use_signal(|| String::new());
    let mut avatar = use_signal(|| {
        user_to_edit.as_ref().map(|u| u.avatar.clone()).unwrap_or_else(|| get_user_avatar_with_default(None))
    });

    // --- Derivazione Proprietà (Configurazione UI) ---
    let is_updating = user_to_edit.is_some();
    let user_id = user_to_edit.as_ref().map(|u| u.id.clone());

    let (header, paragraph, class_container, submit_btn_text, password_required) = if is_updating {
        ("Account Settings", "Update Your Profile", "auth-form-tabbed", "Update", false)
    } else {
        ("Create Account", "Sign up to get started", "auth-form-lg", "Register", true)
    };
    // --- Effetti ---
    // Aggiorna l'anteprima avatar quando ne viene scelto uno nuovo
    use_memo(move || {
        if let Some(img) = new_avatar.read().clone() {
            avatar.set(get_user_avatar_with_default(Some(img)));
        }
    });

    // Gestione errori centralizzata
    use_effect(move || {
        let mut this_error = error.clone();
        if let Some(msg) = this_error() {
            add_toast(
                format!("Error saving user: {}", msg),
                4,
                ToastType::Error,
                toast_state,
            );

            this_error.set(None);
        }
    });

    // --- Handlers ---
    let pick_image = move |_| {
        let mut new_avatar_clone = new_avatar.clone();
        let mut is_loading_clone = is_loading.clone();
        let mut error_clone = error.clone();
        spawn(pick_and_process_avatar(new_avatar_clone, is_loading_clone, error_clone));
    };

    let on_submit = move |_| {
        let p = password.read().clone();
        let rp = repassword.read().clone();
        let u = username.read().clone();
        let a = new_avatar.read().clone();
        let pool = pool.clone();
        let mut auth_state = auth_state.clone();
        // Validazione Client-Side
        if p != rp {
            error.set(Some("Passwords do not match!".to_string()));
            return;
        }

        if !is_updating && p.is_empty() {
            error.set(Some("Password is required for registration".to_string()));
            return;
        }

        spawn(async move {


            match save_or_update_user(&pool, user_id, u, Some(p), a).await {

                Ok(_) => {
                    auth_state.logout();
                    nav.push("/login?new_user=true"); },
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
                }

                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    FormField {
                        label: "Username",
                        input_type: InputType::Text,
                        placeholder: "Choose a username",
                        value: username,
                        required: true,
                    }
                    FormField {
                        label: "Password",
                        input_type: InputType::Password,
                        placeholder: "Create a password",
                        value: password,
                        required: password_required,
                    }
                    FormField {
                        label: "Confirm Password",
                        input_type: InputType::Password,
                        placeholder: "Confirm your password",
                        value: repassword,
                        required: password_required,
                    }
                    ActionButtons {
                        primary_text: "{submit_btn_text}",
                        secondary_text: "Login",
                        primary_on_click: move |_| {},
                        secondary_on_click: move |_| { nav.push("/login"); },
                        variant: ActionButtonsVariant::Auth,
                    }
                }
            }
        }
    }
}