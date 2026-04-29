// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::AuthState;
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, Spinner, SpinnerSize,
    show_toast_error, use_toast,
};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use tracing::{debug, instrument};

#[cfg(feature = "desktop")]
use crate::backend::db_backend::{check_user, fetch_user_data, get_auto_login_user};

#[cfg(feature = "desktop")]
use crate::backend::hello_auth;

#[cfg(feature = "desktop")]
use secrecy::SecretString;

#[cfg(feature = "desktop")]
fn auth_method_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows Hello"
    } else {
        "Biometric Authentication"
    }
}


#[derive(Debug, Clone, PartialEq)]
enum LoginState {
    Checking,
    Attempting,
    Ready,
    Failed(String),
    NoAutoLogin,
}

/// Attempt Windows Hello login. Shared between mount-effect and retry button.
#[cfg(feature = "desktop")]
async fn attempt_hello_login(
    pool: SqlitePool,
    mut state: Signal<LoginState>,
    mut auth_state: AuthState,
    nav: dioxus_router::Navigator,
    toast: Signal<crate::components::ToastHubState>,
) {
    let auto_user = match get_auto_login_user(&pool).await {
        Ok(Some(username)) => username,
        Ok(None) => {
            state.set(LoginState::NoAutoLogin);
            return;
        }
        Err(e) => {
            state.set(LoginState::Failed(
                "Error loading auto-login settings".to_string(),
            ));
            tracing::warn!("Failed to check auto-login user: {}", e);
            return;
        }
    };

    state.set(LoginState::Attempting);
    let username_for_prompt = auto_user.clone();
    let username_for_keyring = auto_user.clone();

    if !hello_auth::is_hello_available() {
        tracing::debug!("Biometric auth not available, skipping Hello login");
        state.set(LoginState::NoAutoLogin);
        return;
    }

    let hello_result = tokio::task::spawn_blocking(move || {
        hello_auth::request_verification(&format!("Sign in as {}?", username_for_prompt))
    })
    .await
    .unwrap_or(hello_auth::HelloResult::Failed("Task spawn failed".into()));

    match &hello_result {
        hello_auth::HelloResult::Success => {
            match hello_auth::load_master_password(&username_for_keyring) {
                Ok(master_password) => {
                    let secret = SecretString::new(master_password.into());
                    match check_user(&pool, &username_for_keyring, &secret).await {
                        Ok(()) => match fetch_user_data(&pool, &username_for_keyring).await {
                            Ok((id, uname, created_at, avatar)) => {
                                debug!("Hello login {id} {uname} {created_at}");
                                auth_state.login(id, uname, created_at, avatar);
                                nav.push("/my-vaults");
                            }
                            Err(e) => {
                                show_toast_error(format!("Login error: {}", e), toast);
                                state.set(LoginState::Failed(
                                    "Login failed after Hello verification".to_string(),
                                ));
                            }
                        },
                        Err(e) => {
                            tracing::warn!("check_user failed for {}: {:?}", username_for_keyring, e);
                            hello_auth::clear_master_password(&username_for_keyring).ok();
                            state.set(LoginState::Failed(
                                "Password in keyring is outdated. Please re-register.".to_string(),
                            ));
                        }
                    }
                }
                Err(e) => {
                    state.set(LoginState::Failed(format!(
                        "Cannot read from keyring: {}",
                        e
                    )));
                }
            }
        }
        hello_auth::HelloResult::Cancelled => {
            state.set(LoginState::Ready);
        }
        hello_auth::HelloResult::NotEnrolled | hello_auth::HelloResult::NotAvailable => {
            state.set(LoginState::NoAutoLogin);
        }
        hello_auth::HelloResult::Failed(msg) => {
            state.set(LoginState::Failed(format!("{} failed: {}", auth_method_name(), msg)));
            tracing::debug!("Hello login result: {:?}", hello_result);
        }
    }
}

#[cfg(feature = "desktop")]
#[component]
#[instrument]
pub fn Login() -> Element {
    let state = use_signal(|| LoginState::Checking);
    let toast = use_toast();
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let auth_state = use_context::<AuthState>();

    let pool_effect = pool.clone();
    let auth_state_effect = auth_state.clone();
    let auth_name = auth_method_name();

    // Auto-attempt biometric/system auth on mount (runs once per component lifecycle)
    use_hook(move || {
        spawn(async move {
            attempt_hello_login(pool_effect, state, auth_state_effect, nav, toast).await;
        });
    });

    // Two retry closures — Dioxus EventHandler doesn't implement Clone
    let pool_r1 = pool.clone();
    let auth_r1 = auth_state.clone();
    let on_retry = move |_| {
        let mut state = state;
        let p = pool_r1.clone();
        let a = auth_r1.clone();
        let n = nav;
        spawn(async move {
            state.set(LoginState::Checking);
            attempt_hello_login(p, state, a, n, toast).await;
        });
    };

    let pool_r2 = pool.clone();
    let auth_r2 = auth_state.clone();
    let on_retry_failed = move |_| {
        let mut state = state;
        let p = pool_r2.clone();
        let a = auth_r2.clone();
        let n = nav;
        spawn(async move {
            state.set(LoginState::Checking);
            attempt_hello_login(p, state, a, n, toast).await;
        });
    };

    rsx! {
        div { class: "page-centered",
            div { class: "auth-form futuristic animate-scale-in",
                h1 { class: "text-h2 text-center", "Welcome Back" }
                p { class: "text-body mb-4 text-center", "Sign in with {auth_name}" }
                match state() {
                    LoginState::Checking | LoginState::Attempting => rsx! {
                        div { class: "flex flex-col items-center gap-4",
                            Spinner { size: SpinnerSize::Large, color_class: "text-primary" }
                            p { class: "text-sm text-base-content/70",
                                if state() == LoginState::Checking {
                                    "Checking auto-login settings..."
                                } else {
                                    "Verifying identity..."
                                }
                            }
                        }
                    },
                    LoginState::Ready => rsx! {
                        div { class: "flex flex-col items-center gap-4",
                            p { class: "text-sm text-base-content/70",
                                "{auth_name} verification was cancelled."
                            }
                            ActionButton {
                                text: "Try Again",
                                variant: ButtonVariant::Primary,
                                button_type: ButtonType::Button,
                                size: ButtonSize::Normal,
                                on_click: on_retry,
                            }
                        }
                    },
                    LoginState::Failed(ref msg) => rsx! {
                        div { class: "flex flex-col items-center gap-4",
                            div { class: "alert alert-error text-sm", "{msg}" }
                            ActionButton {
                                text: "Retry",
                                variant: ButtonVariant::Primary,
                                button_type: ButtonType::Button,
                                size: ButtonSize::Normal,
                                on_click: on_retry_failed,
                            }
                        }
                    },
                    LoginState::NoAutoLogin => rsx! {
                        div { class: "flex flex-col items-center gap-4",
                            div { class: "alert alert-info text-sm",
                                "No account configured. Tap below to begin setup."
                            }
                            ActionButton {
                                text: "Go to Setup",
                                variant: ButtonVariant::Primary,
                                button_type: ButtonType::Button,
                                size: ButtonSize::Normal,
                                on_click: move |_| {
                                    nav.push("/welcome");
                                },
                            }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(not(feature = "desktop"))]
#[component]
#[instrument]
pub fn Login() -> Element {
    rsx! {
        div { class: "page-centered",
            div { class: "auth-form futuristic animate-scale-in",
                h1 { class: "text-h2 text-center", "Not Supported" }
                p { class: "text-body text-center",
                    "Windows Hello login is only available in the desktop application."
                }
            }
        }
    }
}
