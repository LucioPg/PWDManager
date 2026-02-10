use crate::Route;
use crate::auth::AuthState;
use crate::backend::db_backend::{check_user, fetch_user_data};
use crate::components::{
    ActionButtons, ActionButtonsVariant, FormField, InputType, ToastType, ToastsState, add_toast,
};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use tracing::{debug, instrument};

#[component]
#[instrument]
pub fn Login(new_user: Option<bool>) -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error = use_signal(|| Option::<String>::None);
    let mut toast_state = use_context::<Signal<ToastsState>>();
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let auth_state = use_context::<AuthState>();
    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let mut auth_state = auth_state.clone();
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
                        Err(e) => error.set(Some(format!("Errore: {}", e))),
                    }
                }
                Err(e) => error.set(Some(format!("Errore login: {}", e))),
            }
        });
    };
    use_effect(move || {
        if Some(true) == new_user {
            add_toast(
                "User Registerd successfully!".to_string(),
                3,
                ToastType::Success,
                toast_state,
            );
            nav.replace(Route::Login { new_user: None });
        }
        if let Some(msg) = error.read().clone() {
            add_toast(msg.to_string(), 4, ToastType::Error, toast_state);
            nav.replace(Route::Login { new_user: None });
        }
    });
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
                    }
                    FormField {
                        label: "Password".to_string(),
                        input_type: InputType::Password,
                        placeholder: "Enter your password".to_string(),
                        value: password,
                        name: Some("password".to_string()),
                        required: true,
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
