use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::{Theme, UserSettings};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::{Toggle, ToggleColor, ToggleSize};
use pwd_dioxus::{show_toast_error, show_toast_success, use_toast};
use sqlx::SqlitePool;

#[component]
pub fn GeneralSettings() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let pool_for_resource = pool.clone();
    let mut app_theme = use_context::<Signal<Theme>>();
    let toast = use_toast();
    let user_id = auth_state.get_user_id();

    // Signal per lo stato del toggle (light = checked, dark = unchecked)
    let mut is_light = use_signal(|| *app_theme.read() == Theme::Light);

    // Fetch settings per ottenere l'id (necessario per upsert)
    let mut settings_id = use_signal(|| Option::<i64>::None);
    let mut error = use_signal(|| None::<String>);
    let mut ready = use_signal(|| false);

    let _settings_resource = use_resource(move || {
        let pool = pool_for_resource.clone();
        let user_id = user_id.clone();
        let mut settings_id = settings_id.clone();
        let mut ready = ready.clone();
        let mut error = error.clone();
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
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Sincronizza il Signal globale con il toggle locale
    let on_toggle = move |_| {
        let new_theme = if is_light() {
            Theme::Dark
        } else {
            Theme::Light
        };
        is_light.set(new_theme == Theme::Light);
        app_theme.set(new_theme);
    };

    let on_save = move |_| {
        let pool = pool.clone();
        let toast = toast.clone();
        let app_theme = app_theme.clone();
        let settings_id = settings_id.clone();
        spawn(async move {
            let theme = *app_theme.read();
            let settings = UserSettings {
                id: settings_id(),
                user_id,
                theme,
            };
            match UserSettings::upsert_by_id(&settings, &pool).await {
                Ok(_) => {
                    show_toast_success("Theme saved!".to_string(), toast);
                }
                Err(e) => {
                    show_toast_error(format!("Failed to save theme: {}", e), toast);
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
                    onchange: on_toggle,
                    size: ToggleSize::Large,
                    color: ToggleColor::Success,
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
