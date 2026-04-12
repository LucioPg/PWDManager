// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_settings;
#[cfg(target_os = "windows")]
use crate::backend::auto_start::{self, AutoStartError};
#[cfg(feature = "desktop")]
use crate::backend::hello_auth;

#[cfg(feature = "desktop")]
use crate::backend::db_backend::{get_auto_login_user, set_auto_login_enabled};
use crate::backend::settings_types::{AutoLogoutSettings, AutoUpdate, Theme, UserSettings};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::combobox::Combobox;
use pwd_dioxus::{Toggle, ToggleColor, ToggleSize};
use pwd_dioxus::{show_toast_error, show_toast_success, use_toast};
use sqlx::SqlitePool;

fn auto_logout_options() -> Vec<(&'static str, Option<AutoLogoutSettings>)> {
    vec![
        ("10 minutes", Some(AutoLogoutSettings::TenMinutes)),
        ("1 hour", Some(AutoLogoutSettings::OneHour)),
        ("5 hours", Some(AutoLogoutSettings::FiveHours)),
    ]
}

#[cfg(feature = "desktop")]
fn render_auto_login_toggle() -> Element {
    rsx! {
        AutoLoginToggle {}
    }
}

#[cfg(not(feature = "desktop"))]
fn render_auto_login_toggle() -> Element {
    rsx! {}
}

#[cfg(target_os = "windows")]
#[component]
fn AutoStartToggle() -> Element {
    let mut auto_start_enabled = use_signal(|| false);
    let toast = use_toast();

    let _autostart_resource = use_resource(move || {
        let mut auto_start_enabled = auto_start_enabled;
        async move {
            let enabled = tokio::task::spawn_blocking(auto_start::is_enabled)
                .await
                .unwrap_or(false);
            auto_start_enabled.set(enabled);
        }
    });

    let on_toggle = move |_| {
        let toast = toast;
        let currently_enabled = auto_start_enabled();
        spawn(async move {
            let result = match tokio::task::spawn_blocking(move || {
                if currently_enabled {
                    auto_start::disable()
                } else {
                    auto_start::enable()
                }
            })
            .await
            {
                Ok(inner) => inner,
                Err(e) => Err(AutoStartError::RegistryError(format!("Task failed: {}", e))),
            };

            match result {
                Ok(()) => {
                    auto_start_enabled.set(!auto_start_enabled());
                }
                Err(e) => {
                    show_toast_error(format!("Auto-start error: {}", e), toast);
                }
            }
        });
    };

    rsx! {
        div { class: "flex flex-row justify-between mb-2",
            label { class: "label cursor-pointer",
                strong {
                    span { class: "label-text", "Auto Start" }
                }
            }
            Toggle {
                checked: auto_start_enabled(),
                onchange: on_toggle,
                size: ToggleSize::Large,
                color: ToggleColor::Success,
            }
        }
    }
}

#[cfg(feature = "desktop")]
#[component]
fn AutoLoginToggle() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let mut auto_login_enabled = use_signal(|| false);
    let toast = use_toast();
    let username = auth_state.get_username();
    let pool_for_load = pool.clone();
    let username_for_load = username.clone();

    // Load current auto-login state on mount
    let _load_resource = use_resource(move || {
        let pool = pool_for_load.clone();
        let mut auto_login_enabled = auto_login_enabled;
        let username = username_for_load.clone();
        async move {
            match get_auto_login_user(&pool).await {
                Ok(Some(auto_user)) => {
                    auto_login_enabled.set(auto_user == username);
                }
                Ok(None) => {
                    auto_login_enabled.set(false);
                }
                Err(_) => {
                    auto_login_enabled.set(false);
                }
            }
        }
    });

    let on_toggle = move |_| {
        let pool = pool.clone();
        let mut auto_login_enabled = auto_login_enabled;
        let toast = toast;
        let username = username.clone();
        spawn(async move {
            if !auto_login_enabled() {
                // Enabling: require Hello verification first
                let hello_result = tokio::task::spawn_blocking(move || {
                    hello_auth::request_verification("Verifica identità per attivare auto-login")
                })
                .await
                .unwrap_or(hello_auth::HelloResult::Failed("Task spawn fallito".into()));

                match hello_result {
                    hello_auth::HelloResult::Success => {
                        // Enable the DB flag. The password will be stored on next login.
                        match set_auto_login_enabled(&pool, &username, true).await {
                            Ok(()) => {
                                auto_login_enabled.set(true);
                                show_toast_success("Auto-login attivato. Effettua il login per completare la configurazione.".to_string(), toast);
                            }
                            Err(e) => {
                                show_toast_error(format!("Impossibile attivare auto-login: {}", e), toast);
                            }
                        }
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
                        show_toast_error("Windows Hello non è disponibile su questo dispositivo".to_string(), toast);
                    }
                }
            } else {
                // Disabling: clear keyring and disable
                let username_for_clear = username.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    hello_auth::clear_master_password(&username_for_clear)
                }).await;

                match set_auto_login_enabled(&pool, &username, false).await {
                    Ok(()) => {
                        auto_login_enabled.set(false);
                        show_toast_success("Auto-login disattivato".to_string(), toast);
                    }
                    Err(e) => {
                        show_toast_error(format!("Impossibile disattivare auto-login: {}", e), toast);
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "flex flex-row justify-between mb-2",
            label { class: "label cursor-pointer",
                strong {
                    span { class: "label-text", "Auto-login con Windows Hello" }
                }
            }
            Toggle {
                checked: auto_login_enabled(),
                onchange: on_toggle,
                size: ToggleSize::Large,
                color: ToggleColor::Success,
            }
        }
    }
}

#[component]
pub fn GeneralSettings() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let pool_for_resource = pool.clone();
    let mut app_theme = use_context::<Signal<Theme>>();
    let mut auto_update = use_context::<Signal<AutoUpdate>>();
    let mut auto_logout_settings = use_context::<Signal<Option<AutoLogoutSettings>>>();
    let mut auto_logout_toggle = use_signal(|| auto_logout_settings.read().is_none());
    let toast = use_toast();
    let user_id = auth_state.get_user_id();

    // Signal per lo stato del toggle (light = checked, dark = unchecked)
    let mut is_light = use_signal(|| *app_theme.read() == Theme::Light);
    let mut auto_update_sig = use_signal(|| *auto_update.read() == AutoUpdate(true));
    // Fetch settings per ottenere l'id (necessario per upsert)
    let settings_id = use_signal(|| Option::<i64>::None);
    let error = use_signal(|| None::<String>);
    let ready = use_signal(|| false);
    let options = auto_logout_options();
    let _settings_resource = use_resource(move || {
        let pool = pool_for_resource.clone();
        let user_id = user_id;
        let mut settings_id = settings_id;
        let mut ready = ready;
        let mut error = error;
        async move {
            match fetch_user_settings(&pool, user_id).await {
                Ok(Some(settings)) => {
                    settings_id.set(settings.id);
                    ready.set(true);
                }
                Ok(None) => {
                    // Nessun record: primo save creera il record
                    ready.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    ready.set(true);
                }
            }
        }
    });

    use_effect(move || {
        let mut this_error = error;
        let toast = toast;
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Sincronizza il Signal globale con il toggle locale
    let on_toggle_theme = move |_| {
        let new_theme = if is_light() {
            Theme::Dark
        } else {
            Theme::Light
        };
        is_light.set(new_theme == Theme::Light);
        app_theme.set(new_theme);
    };

    let on_toggle_auto_update = move |_| {
        let new_auto_update = if auto_update_sig() {
            AutoUpdate(false)
        } else {
            AutoUpdate(true)
        };
        auto_update.set(new_auto_update);
        auto_update_sig.set(new_auto_update.into());
    };
    let on_toggle_auto_logout = move |_| {
        if auto_logout_toggle() {
            // Nessun timer configurato → si abilita il feature
            auto_logout_toggle.set(false);
        } else {
            // Timer configurato → si disabilita il feature
            auto_logout_toggle.set(true);
            auto_logout_settings.set(None);
        }
    };

    let on_save = move |_| {
        let pool = pool.clone();
        let toast = toast;
        let app_theme = app_theme;
        let settings_id = settings_id;
        spawn(async move {
            let theme = *app_theme.read();
            let auto_update = *auto_update.read();
            let auto_logout_settings = *auto_logout_settings.read();
            let settings = UserSettings {
                id: settings_id(),
                user_id,
                theme,
                auto_update,
                auto_logout_settings,
                active_vault_id: None,
            };
            match UserSettings::upsert_by_id(&settings, &pool).await {
                Ok(_) => {
                    show_toast_success("Settings saved!".to_string(), toast);
                }
                Err(e) => {
                    show_toast_error(format!("Failed to save settings: {}", e), toast);
                }
            }
        });
    };

    if !ready() {
        return rsx! {};
    }

    rsx! {
        div { class: "auth-form-tabbed rounded-box w-full",
            div { class: "flex flex-row justify-between mb-2",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text", "Light Theme" }
                    }
                }
                Toggle {
                    checked: is_light(),
                    onchange: on_toggle_theme,
                    size: ToggleSize::Large,
                    color: ToggleColor::Success,
                }
            }
            div { class: "flex flex-row justify-between mb-2",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text", "Auto Update" }
                    }
                }
                Toggle {
                    checked: auto_update_sig(),
                    onchange: on_toggle_auto_update,
                    size: ToggleSize::Large,
                    color: ToggleColor::Success,
                }
            }
            AutoStartToggle {}
            {render_auto_login_toggle()}
            div { class: "flex flex-row justify-between mb-2",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text", "Auto Logout" }
                    }
                }
                Toggle {
                    checked: !auto_logout_toggle(),
                    onchange: on_toggle_auto_logout,
                    size: ToggleSize::Large,
                    color: ToggleColor::Success,
                }
            }
            div { class: "flex flex-row justify-between mb-2 ml-4",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text", "Auto Logout Timer" }
                    }
                }
                Combobox::<AutoLogoutSettings> {
                    options: options.clone(),
                    placeholder: "Select timer".to_string(),
                    on_change: move |v| auto_logout_settings.set(v),
                    disabled: auto_logout_toggle,
                    selected_value: *auto_logout_settings.read(),
                }
            }
            div { class: "flex flex-row justify-end gap-2 mt-2",
                ActionButton {
                    text: "Save".to_string(),
                    variant: ButtonVariant::Success,
                    button_type: ButtonType::Submit,
                    size: ButtonSize::Normal,
                    on_click: on_save,
                }
            }
        }
    }
}
