use crate::auth::{AuthState, User};
use crate::backend::db_backend::{delete_user, save_or_update_user};
use crate::backend::ui_utils::pick_and_process_avatar;
use crate::backend::utils::get_user_avatar_with_default;
use crate::components::{
    ActionButton, AvatarSelector, AvatarSize, ButtonSize, ButtonType, ButtonVariant, FormField,
    InputType, ToastType, ToastsState, UserDeletionDialog, add_toast,
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
    let pool_clone_on_submit = pool.clone();
    #[allow(unused_mut)]
    let mut toast_state = use_context::<Signal<ToastsState>>();
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

    // Inizializzazione dati utente (Semplificata con unwrap_or_default)
    #[allow(unused_mut)]
    let mut username = use_signal(|| {
        user_to_edit
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default()
    });
    #[allow(unused_mut)]
    let mut password = use_signal(|| String::new());
    #[allow(unused_mut)]
    let mut repassword = use_signal(|| String::new());
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
        if let Some(msg) = this_error() {
            add_toast(
                format!("Error saving user: {}", msg),
                3,
                ToastType::Error,
                toast_state,
            );

            this_error.set(None);
        }
    });

    // Gestione errori centralizzata
    use_effect(move || {
        let user = auth_state_delete_clone.get_user();
        let mut user_deleted = is_user_deleted.clone();
        if user_deleted() {
            add_toast(
                format!("User {} deleted successfully!", user.unwrap().username),
                2,
                ToastType::Success,
                toast_state,
            );

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
    let on_delete_user = move || {
        let mut is_user_deleted = is_user_deleted.clone();
        let pool_for_delete = pool.clone();
        let user = auth_state.get_user();
        let mut error = error.clone();
        let mut auth_state_logout_clone = auth_state_logout_clone.clone();
        match user {
            Some(user) => {
                spawn(async move {
                    match delete_user(&pool_for_delete, user.id).await {
                        Ok(()) => {
                            is_user_deleted.set(true);
                            auth_state_logout_clone.logout();
                        }
                        Err(e) => {
                            error.set(Some(e.to_string()));
                        }
                    }
                });
            }
            None => println!("No user to delete"),
        }
    };
    let on_submit = move |_| {
        let p = password.read().clone();
        let rp = repassword.read().clone();
        let u = username.read().clone();
        let a = new_avatar.read().clone();
        let pool = pool_clone_on_submit.clone();
        let mut auth_state = auth_state_submit_clone.clone();
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
                    let url = if is_updating {
                        "/login?new_user=true"
                    } else {
                        "/login?user_updated=true"
                    };
                    nav.push(url);
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
                            on_click: move |_| {on_delete_user()},
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
    }
}
