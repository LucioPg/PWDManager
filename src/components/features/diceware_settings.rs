// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::auth::AuthState;
use crate::backend::db_backend::{fetch_diceware_settings, upsert_diceware_settings};
use crate::backend::password_utils::{DicewareGenConfig, generate_diceware_password};
use crate::backend::settings_types::{DicewareGenerationSettings, DicewareLanguage};
use crate::components::globals::toggle::{Toggle, ToggleColor, ToggleSize};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::combobox::Combobox;
use pwd_dioxus::form::{FormField, NonNegativeInt, PositiveInt};
use pwd_dioxus::spinner::{Spinner, SpinnerSize};
use pwd_dioxus::{InputType, show_toast_error, show_toast_success, use_toast};
use sqlx::SqlitePool;

fn language_options() -> Vec<(&'static str, Option<DicewareLanguage>)> {
    vec![
        ("English", Some(DicewareLanguage::EN)),
        ("Français", Some(DicewareLanguage::FR)),
        ("Italiano", Some(DicewareLanguage::IT)),
    ]
}

#[component]
pub fn DicewareSettings() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let pool_for_submit = pool.clone();
    let toast = use_toast();
    let user_id = auth_state.get_user_id();
    let error = use_signal(|| <Option<String>>::None);
    let word_count = use_signal(|| PositiveInt(6));
    let mut add_special_char = use_signal(|| false);
    let numbers = use_signal(|| NonNegativeInt(0));
    let mut current_language = use_signal(|| Option::<DicewareLanguage>::None);
    let settings_ready = use_signal(|| false);
    let settings_id = use_signal(|| -1i64);

    let options = language_options();

    let _current_settings = use_resource(move || {
        let user_id = user_id;
        let pool = pool.clone();
        let mut word_count = word_count;
        let mut add_special_char = add_special_char;
        let mut numbers = numbers;
        let mut current_language = current_language;
        let mut settings_id = settings_id;
        let mut settings_ready = settings_ready;
        let mut error = error;
        async move {
            match fetch_diceware_settings(&pool, user_id).await {
                Ok(settings) => {
                    word_count.set(PositiveInt(settings.word_count as u32));
                    add_special_char.set(settings.add_special_char);
                    numbers.set(NonNegativeInt(settings.numbers as u32));
                    current_language.set(Some(settings.language));
                    settings_id.set(settings.settings_id);
                    settings_ready.set(true);
                    settings
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    settings_ready.set(true);
                    DicewareGenerationSettings::default()
                }
            }
        }
    });

    use_effect(move || {
        let mut this_error = error;
        let toast = toast;
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching diceware settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    let on_submit = move |_| {
        let word_count = word_count();
        let add_special_char = add_special_char();
        let numbers = numbers();
        let language = current_language().unwrap_or(DicewareLanguage::EN);
        let sid = settings_id();

        let settings = DicewareGenerationSettings {
            id: Some(sid),
            settings_id: sid,
            word_count: word_count.into(),
            add_special_char,
            numbers: numbers.into(),
            language,
        };

        let pool = pool_for_submit.clone();
        let toast = toast;
        spawn(async move {
            // Validate settings by attempting a silent generation
            let gen_config = DicewareGenConfig::from(settings.clone());
            match generate_diceware_password(gen_config) {
                Ok(_) => {} // Settings are valid, proceed to save
                Err(e) => {
                    show_toast_error(
                        format!("Settings saved, but: {}", e),
                        toast,
                    );
                }
            }

            match upsert_diceware_settings(&pool, settings).await {
                Ok(()) => {
                    show_toast_success("Diceware settings saved!".to_string(), toast);
                }
                Err(e) => {
                    show_toast_error(format!("Error saving diceware settings: {}", e), toast);
                }
            }
        });
    };

    if !settings_ready() {
        return rsx! {
            div { class: "flex flex-col gap-4",
                Spinner { size: SpinnerSize::Medium, color_class: "text-info" }
            }
        };
    }

    rsx! {
        form { class: "flex flex-col gap-4  mb-[1rem]", onsubmit: on_submit,

            // div { class: "pwd-diceware-settings flex flex-col gap-4 rounded-box bg-base-200 p-4 w-full",
            div { class: "auth-form-tabbed rounded-box w-full",
                strong {
                    h2 { class: "text-center mb-4", "Diceware Settings" }
                }

                // Language
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Language" }
                        }
                    }
                    Combobox::<DicewareLanguage> {
                        options: options.clone(),
                        placeholder: "Select language".to_string(),
                        on_change: move |v| current_language.set(v),
                        selected_value: current_language,
                    }
                }

                // Word count
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Word count" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::PositiveInt,
                        value: word_count,
                        placeholder: String::new(),
                        label: String::new(),
                    }
                }

                // Add special char toggle
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Add 1 special char" }
                        }
                    }
                    Toggle {
                        checked: add_special_char(),
                        onchange: move |_| add_special_char.toggle(),
                        size: ToggleSize::Large,
                        color: ToggleColor::Success,
                    }
                }

                // Numbers
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Numbers" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::NonNegativeInt,
                        value: numbers,
                        placeholder: String::new(),
                        label: String::new(),
                    }
                }
                div { class: "flex flex-row justify-end gap-2",
                    ActionButton {
                        text: "Save".to_string(),
                        variant: ButtonVariant::Success,
                        button_type: ButtonType::Submit,
                        size: ButtonSize::Normal,
                        on_click: move |_| {},
                    }
                }
            }
        }
    }
}
