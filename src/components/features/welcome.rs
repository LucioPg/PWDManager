// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::AuthState;
use crate::backend::avatar_utils::get_user_avatar_with_default;
use crate::backend::db_backend::{
    fetch_user_data, get_system_username, has_any_user, register_user_with_settings,
    set_auto_login_enabled,
};
use crate::backend::password_utils::generate_suggested_password;
use crate::backend::ui_utils::pick_and_process_avatar;
use crate::backend::vault_utils::create_vault;
use crate::components::{
    ActionButton, AvatarSelector, AvatarSize, ButtonSize, ButtonType, ButtonVariant,
    Spinner, SpinnerSize,
    schedule_toast_success, show_toast_error, use_toast,
};
use dioxus::prelude::*;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;

#[cfg(feature = "desktop")]
use crate::backend::hello_auth;

#[derive(Debug, Clone, PartialEq)]
enum WelcomeState {
    Checking, // Checking has_any_user
    Ready,    // No users exist — show form
    Loading,  // Registration in progress
    Error(String),
}

#[cfg(feature = "desktop")]
#[component]
pub fn WelcomePage() -> Element {
    let nav = use_navigator();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let auth_state = use_context::<AuthState>();

    let mut state = use_signal(|| WelcomeState::Checking);
    let mut avatar = use_signal(|| None::<Vec<u8>>);
    let mut avatar_src =
        use_signal(|| get_user_avatar_with_default(None));
    let mut is_picking = use_signal(|| false);
    let avatar_loading = use_signal(|| false);
    let avatar_error = use_signal(|| Option::<String>::None);

    // Check if users exist — show spinner during async check
    let pool_for_check = pool.clone();
    let nav_for_check = nav.clone();
    use_effect(move || {
        let pool = pool_for_check.clone();
        let nav = nav_for_check.clone();
        let mut state = state;
        spawn(async move {
            match has_any_user(&pool).await {
                Ok(true) => {
                    nav.push("/login");
                }
                Ok(false) => {
                    state.set(WelcomeState::Ready);
                }
                Err(e) => {
                    state.set(WelcomeState::Error(format!(
                        "Failed to check database: {}",
                        e
                    )));
                }
            }
        });
    });

    // Update avatar preview
    use_effect(move || {
        if let Some(img) = avatar.read().clone() {
            avatar_src.set(get_user_avatar_with_default(Some(img)));
        }
    });

    let pick_image = move |_| {
        if is_picking() {
            return;
        }
        spawn(pick_and_process_avatar(
            avatar,
            avatar_loading,
            is_picking,
            avatar_error,
        ));
    };

    let on_submit = move |_| {
        let u = match get_system_username() {
            Ok(name) => name,
            Err(e) => {
                state.set(WelcomeState::Error(e.to_string()));
                return;
            }
        };

        let pool = pool.clone();
        let mut auth_state = auth_state.clone();
        let nav = nav.clone();
        let toast = toast;
        let a = avatar.read().clone();

        state.set(WelcomeState::Loading);
        spawn(async move {
            // Verify identity via Windows Hello before creating account
            let username_for_hello = u.clone();
            let hello_result = tokio::task::spawn_blocking(move || {
                hello_auth::request_verification(&format!(
                    "Confirm account creation for {}?",
                    username_for_hello
                ))
            })
            .await
            .unwrap_or(hello_auth::HelloResult::Failed("Task spawn failed".into()));

            match hello_result {
                hello_auth::HelloResult::Success => {}
                hello_auth::HelloResult::Cancelled => {
                    state.set(WelcomeState::Ready);
                    return;
                }
                hello_auth::HelloResult::NotEnrolled => {
                    state.set(WelcomeState::Error(
                        "Windows Hello is not configured. Set it up in Windows Settings first."
                            .into(),
                    ));
                    return;
                }
                hello_auth::HelloResult::NotAvailable => {
                    state.set(WelcomeState::Error(
                        "Windows Hello is not available on this device.".into(),
                    ));
                    return;
                }
                hello_auth::HelloResult::Failed(msg) => {
                    state.set(WelcomeState::Error(msg));
                    return;
                }
            }

            let password = generate_suggested_password(None);

            match register_user_with_settings(
                &pool,
                u.clone(),
                Some(password.clone()),
                a,
                pwd_types::PasswordPreset::God,
            )
            .await
            {
                Ok(saved_user_id) => {
                    if create_vault(&pool, saved_user_id, "Default".to_string(), None)
                        .await
                        .is_err()
                    {
                        tracing::warn!("Failed to create default vault");
                    }

                    if let Err(e) = hello_auth::store_master_password(&u, password.expose_secret())
                    {
                        tracing::warn!("Failed to store master password in keyring: {}", e);
                    }

                    if let Err(e) = set_auto_login_enabled(&pool, &u, true).await {
                        tracing::warn!("Failed to enable auto-login: {}", e);
                    }

                    match fetch_user_data(&pool, &u).await {
                        Ok((id, uname, created_at, avatar_data)) => {
                            auth_state.login(id, uname, created_at, avatar_data);
                            schedule_toast_success(
                                "Account created successfully!".to_string(),
                                toast,
                            );
                            nav.push("/my-vaults");
                        }
                        Err(e) => {
                            show_toast_error(format!("Login error: {}", e), toast);
                            nav.push("/login");
                        }
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("UNIQUE constraint") || msg.contains("duplicate") {
                        state.set(WelcomeState::Error(
                            "A user with this system username already exists. \
                             Please use the recovery key or reset the database."
                                .into(),
                        ));
                    } else {
                        state.set(WelcomeState::Error(msg));
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "page-centered",
            div { class: "auth-form futuristic animate-scale-in",
                h1 { class: "text-h2 text-center", "Welcome to PWDManager" }
                p { class: "text-body mb-2 text-center",
                    "A military-grade (GOD level) master password has been "
                    "automatically generated and securely stored. "
                    "It will be used to encrypt your stored passwords."
                }
                p { class: "text-sm text-base-content/70 mb-4 text-center",
                    "You can change it later in Settings."
                }

                match state() {
                    WelcomeState::Checking => rsx! {
                        div { class: "flex justify-center my-4",
                            Spinner { size: SpinnerSize::Large, color_class: "text-primary" }
                        }
                    },
                    WelcomeState::Loading => rsx! {
                        div { class: "flex justify-center my-4",
                            Spinner { size: SpinnerSize::Large, color_class: "text-primary" }
                        }
                    },
                    WelcomeState::Ready => rsx! {
                        AvatarSelector {
                            avatar_src: avatar_src.read().clone(),
                            on_pick: pick_image,
                            button_text: "Select Avatar",
                            size: AvatarSize::XXLarge,
                            shadow: true,
                            show_border: true,
                            loading: avatar_loading,
                            is_picking,
                        }
                        form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                            div { class: "flex flex-col gap-2 w-full mt-2",
                                ActionButton {
                                    text: "Get Started",
                                    variant: ButtonVariant::Success,
                                    button_type: ButtonType::Submit,
                                    size: ButtonSize::Normal,
                                    on_click: move |_| {},
                                }
                            }
                        }
                    },
                    WelcomeState::Error(ref msg) => rsx! {
                        div { class: "alert alert-error mb-4 text-sm",
                            p { "{msg}" }
                        }
                        ActionButton {
                            text: "Retry",
                            variant: ButtonVariant::Primary,
                            button_type: ButtonType::Button,
                            size: ButtonSize::Normal,
                            on_click: move |_| {
                                state.set(WelcomeState::Ready);
                            },
                        }
                    },
                }
            }
        }
    }
}

// Non-desktop stub — feature "desktop" is required for keyring (hello_auth)
#[cfg(not(feature = "desktop"))]
#[component]
pub fn WelcomePage() -> Element {
    rsx! {
        div { class: "page-centered",
            div { class: "auth-form futuristic animate-scale-in",
                h1 { class: "text-h2 text-center", "Welcome to PWDManager" }
                p { class: "text-body text-center",
                    "Desktop application required for initial setup."
                }
            }
        }
    }
}
