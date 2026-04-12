// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::AuthState;
use crate::backend::db_backend::{check_user, fetch_user_data};
use crate::components::{ActionButtons, ActionButtonsVariant, show_toast_error, use_toast};
use dioxus::prelude::*;
use pwd_dioxus::form::FormField;
use pwd_dioxus::{FormSecret, InputType};
use secrecy::SecretString;
use sqlx::SqlitePool;
use tracing::{debug, instrument};

#[cfg(feature = "desktop")]
use crate::backend::db_backend::get_auto_login_user;

#[cfg(feature = "desktop")]
use crate::backend::hello_auth;

#[cfg(feature = "desktop")]
use std::sync::OnceLock;

#[cfg(feature = "desktop")]
#[derive(Debug, Clone, PartialEq)]
enum HelloLoginState {
    Idle,
    Attempting,
    Failed(String),
}

#[component]
#[instrument]
pub fn Login() -> Element {
    #[allow(unused_mut, clippy::redundant_closure)]
    let mut username = use_signal(|| String::new());
    #[allow(unused_mut)]
    let mut password = use_signal(|| FormSecret(SecretString::new("".into())));
    let toast = use_toast();
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let auth_state = use_context::<AuthState>();

    #[cfg(feature = "desktop")]
    #[allow(unused_mut)]
    let mut hello_state = use_signal(|| HelloLoginState::Idle);

    #[cfg(feature = "desktop")]
    let pool_for_effect = pool.clone();
    #[cfg(feature = "desktop")]
    let auth_state_for_effect = auth_state.clone();
    #[cfg(feature = "desktop")]
    let nav_for_effect = nav.clone();

    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let mut auth_state = auth_state.clone();
        let toast = toast;
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
                            let nav_dashboard = nav;
                            nav_dashboard.push("/dashboard");
                        }
                        Err(e) => show_toast_error(format!("Errore: {}", e), toast),
                    }
                }
                Err(e) => show_toast_error(format!("Errore login: {}", e), toast),
            }
        });
    };
    #[cfg(feature = "desktop")]
    use_effect(move || {
        static INIT: OnceLock<bool> = OnceLock::new();
        if INIT.get().is_some() {
            return;
        }
        let _ = INIT.set(true);

        let pool = pool_for_effect.clone();
        let mut hello_state = hello_state;
        let mut auth_state = auth_state_for_effect.clone();
        let nav = nav_for_effect.clone();
        let toast = toast;

        spawn(async move {
            // Check if any user has auto-login enabled
            let auto_user = match get_auto_login_user(&pool).await {
                Ok(Some(username)) => username,
                Ok(None) => {
                    hello_state.set(HelloLoginState::Idle);
                    return;
                }
                Err(e) => {
                    hello_state.set(HelloLoginState::Failed(
                        "Errore nel caricamento delle impostazioni".to_string()
                    ));
                    tracing::warn!("Failed to check auto-login user: {}", e);
                    return;
                }
            };

            hello_state.set(HelloLoginState::Attempting);
            let username_for_prompt = auto_user.clone();
            let username_for_keyring = auto_user.clone();

            // Request Windows Hello verification (blocking call in spawn_blocking)
            let hello_result = tokio::task::spawn_blocking(move || {
                hello_auth::request_verification(&format!("Accedi come {}?", username_for_prompt))
            })
            .await
            .unwrap_or(hello_auth::HelloResult::Failed("Task spawn fallito".into()));

            match hello_result {
                hello_auth::HelloResult::Success => {
                    // Load master password from keyring
                    match hello_auth::load_master_password(&username_for_keyring) {
                        Ok(master_password) => {
                            let secret = SecretString::new(master_password.into());
                            match check_user(&pool, &username_for_keyring, &secret).await {
                                Ok(()) => {
                                    match fetch_user_data(&pool, &username_for_keyring).await {
                                        Ok((id, uname, created_at, avatar)) => {
                                            auth_state.login(id, uname, created_at, avatar);
                                            nav.push("/dashboard");
                                        }
                                        Err(e) => {
                                            show_toast_error(format!("Errore login: {}", e), toast);
                                            hello_state.set(HelloLoginState::Failed(
                                                "Login fallito dopo verifica Hello".to_string()
                                            ));
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Hello succeeded but password in keyring is outdated
                                    hello_auth::clear_master_password(&username_for_keyring).ok();
                                    let _ = crate::backend::db_backend::set_auto_login_enabled(
                                        &pool, &username_for_keyring, false
                                    ).await;
                                    hello_state.set(HelloLoginState::Failed(
                                        "Password nel keyring obsoleta. Effettua il login manuale.".to_string()
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            hello_state.set(HelloLoginState::Failed(
                                format!("Impossibile leggere dal keyring: {}", e)
                            ));
                        }
                    }
                }
                hello_auth::HelloResult::Cancelled => {
                    hello_state.set(HelloLoginState::Idle);
                }
                _ => {
                    hello_state.set(HelloLoginState::Failed(
                        "Auto-login non disponibile".to_string()
                    ));
                    tracing::debug!("Hello login result: {:?}", hello_result);
                }
            }
        });
    });
    rsx! {
        div { class: "page-centered",
            div { class: "auth-form futuristic animate-scale-in",
                h1 { class: "text-h2 text-center", "Welcome Back " }
                p { class: "text-body mb-4 text-center", "Sign in to your account to continue" }
                HelloLoginStatus { hello_state }
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
                        secondary_on_click: move |_| {
                            nav.push("/register");
                        },
                        variant: ActionButtonsVariant::Auth,
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
#[component]
fn HelloLoginStatus(hello_state: Signal<HelloLoginState>) -> Element {
    match hello_state() {
        HelloLoginState::Attempting => {
            rsx! {
                div { class: "flex flex-col items-center gap-3 mb-4",
                    span { class: "loading loading-spinner loading-lg text-primary" }
                    p { class: "text-sm text-base-content/70", "Verifica identità in corso..." }
                }
            }
        }
        HelloLoginState::Failed(ref msg) => {
            rsx! {
                div { class: "alert alert-info mb-4 text-sm",
                    p { "{msg}" }
                }
            }
        }
        HelloLoginState::Idle => {
            rsx! {}
        }
    }
}

#[cfg(not(feature = "desktop"))]
#[component]
fn HelloLoginStatus() -> Element {
    Ok(VNode::placeholder())
}
