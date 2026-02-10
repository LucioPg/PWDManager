use crate::auth::User;
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
    let is_updating = user_to_edit.is_some();
    let user_id = user_to_edit
        .as_ref()
        .map(|u| Some(u.id.clone()))
        .unwrap_or_else(|| None);
    let mut header: &str;
    let mut paragraph: &str;
    if is_updating {
        header = "Account Settings";
        paragraph = "Update Your Profile";
    } else {
        header = "Create Account";
        paragraph = "Sign up to get started with your account";
    };
    let mut username = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_else(|| String::new())
    });
    let mut password = use_signal(|| String::new());
    let mut repassword = use_signal(|| String::new());
    let avatar = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.avatar.clone())
            .unwrap_or_else(|| "".to_string())
    });
    let new_avatar = use_signal(|| None::<Vec<u8>>);
    let mut toast_state = use_context::<Signal<ToastsState>>();
    let mut error = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    let pick_image = move |_evt: MouseEvent| {
        let mut err_signal = error;
        let mut img_signal = new_avatar;
        let mut is_loading_signal = is_loading;
        spawn(pick_and_process_avatar(
            img_signal,
            is_loading_signal,
            err_signal,
        ));
    };
    let mut avatar_writer = avatar.clone();
    use_memo(move || {
        if let Some(img) = new_avatar.read().clone() {
            avatar_writer.set(get_user_avatar_with_default(Some(img)))
        }
    });
    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let rp = repassword.read().clone();
        let a = new_avatar.read().clone();
        if p == rp {
            spawn(async move {
                // La tua funzione check_user ora ha il pool!
                let result = save_or_update_user(&pool, user_id, u, p, a).await;
                match result {
                    Ok(()) => {
                        println!("Successo!");
                        nav.push("/login?new_user=true");
                    }
                    Err(e) => {
                        error.set(Some(format!("Error saving user: {}", e.to_string())));
                    }
                }
            });
        } else {
            error.set(Some("Passwords do not match!".to_string()));
        }
    };

    use_effect(move || {
        // 1. Leggiamo il valore attuale del segnale (crea la sottoscrizione)
        let mut this_error = error.clone();
        if let Some(msg) = this_error() {
            // 2. Lanciamo il toast usando la tua funzione specifica
            add_toast(
                format!("Error saving user: {}", msg),
                4,
                ToastType::Error,
                toast_state,
            );

            // 3. OPZIONALE: Resettiamo l'errore subito dopo averlo mostrato
            // per evitare che il toast riappaia se il componente si ri-renderizza
            this_error.set(None);
        }
    });

    rsx! {
        div { class: "page-centered",
            div { class: "auth-form-lg animate-scale-in",
                h1 { class: "text-h2 text-center", "{&header}" }
                p { class: "text-body mb-4 text-center", "{&paragraph}" }
                AvatarSelector {
                    avatar_src: avatar.read().clone(),
                    on_pick: pick_image,
                    button_text: "Select Avatar".to_string(),
                    size: AvatarSize::XXLarge,
                    shadow: true,
                    show_border: true,
                    loading: is_loading,
                }
                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    FormField {
                        label: "Username".to_string(),
                        input_type: InputType::Text,
                        placeholder: "Choose a username".to_string(),
                        value: username,
                        name: Some("username".to_string()),
                        required: true,
                    }
                    FormField {
                        label: "Password".to_string(),
                        input_type: InputType::Password,
                        placeholder: "Create a password".to_string(),
                        value: password,
                        name: Some("password".to_string()),
                        required: true,
                    }
                    FormField {
                        label: "Confirm Password".to_string(),
                        input_type: InputType::Password,
                        placeholder: "Confirm your password".to_string(),
                        value: repassword,
                        name: Some("repassword".to_string()),
                        required: true,
                    }
                    ActionButtons {
                        primary_text: "Register".to_string(),
                        secondary_text: "Login".to_string(),
                        primary_on_click: move |_| {}, // Gestito dal form onsubmit
                        secondary_on_click: move |_| { nav.push("/login"); },
                        variant: ActionButtonsVariant::Auth,
                    }
                }
            }
        }
    }
}
