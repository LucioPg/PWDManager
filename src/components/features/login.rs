use crate::auth::AuthState;
use crate::backend::db_backend::{check_user, fetch_user_data};
use crate::components::{
    ActionButtons, ActionButtonsVariant, FormField, FormSecret, InputType, show_toast_error,
    use_toast,
};
use dioxus::prelude::*;
use secrecy::SecretString;
use sqlx::SqlitePool;
use tracing::{debug, instrument};

#[component]
#[instrument]
pub fn Login() -> Element {
    #[allow(unused_mut)]
    let mut username = use_signal(|| String::new());
    #[allow(unused_mut)]
    let mut password = use_signal(|| FormSecret(SecretString::new("".into())));
    let toast = use_toast();
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let auth_state = use_context::<AuthState>();
    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let mut auth_state = auth_state.clone();
        let toast = toast.clone();
        spawn(async move {
            // La tua funzione check_user ora ha il pool!
            match check_user(&pool, &u, &p).await {
                Ok(()) => {
                    println!("Successo!");
                    let result = fetch_user_data(&pool, &u).await;
                    match result {
                        Ok((id, username, created_at, avatar)) => {
                            debug!("Login {id} {username} {created_at}");
                            auth_state.login(id, username, created_at, avatar);
                            let nav_dashboard = nav.clone();
                            nav_dashboard.push("/dashboard");
                        }
                        Err(e) => show_toast_error(format!("Errore: {}", e), toast),
                    }
                }
                Err(e) => show_toast_error(format!("Errore login: {}", e), toast),
            }
        });
    };
    rsx! {
        div { class: "page-centered",
            div { class: "auth-form animate-scale-in",
                h1 { class: "text-h2 text-center", "Welcome Back" }
                p { class: "text-body mb-4 text-center", "Sign in to your account to continue" }
                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    FormField {
                        label: "Username".to_string(),
                        input_type: InputType::Text,
                        placeholder: "Enter your username".to_string(),
                        value: username,
                        name: Some("username".to_string()),
                        required: true,
                        alphanumeric_only: true,
                    }
                    FormField {
                        label: "Password".to_string(),
                        input_type: InputType::Password,
                        placeholder: "Enter your password".to_string(),
                        value: password,
                        name: Some("password".to_string()),
                        required: true,
                        show_visibility_toggle: true,
                        forbid_spaces: true,
                    }
                    ActionButtons {
                        primary_text: "Login".to_string(),
                        secondary_text: "Register".to_string(),
                        primary_on_click: move |_| {}, // Gestito dal form onsubmit
                        secondary_on_click: move |_| { nav.push("/register"); },
                        variant: ActionButtonsVariant::Auth,
                    }
                }
            }
        }
    }
}
