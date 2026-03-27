use crate::auth::AuthState;
use crate::backend::db_backend::{fetch_diceware_settings, upsert_diceware_settings};
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
    let mut error = use_signal(|| <Option<String>>::None);
    let mut word_count = use_signal(|| PositiveInt(6));
    let mut special_chars = use_signal(|| NonNegativeInt(0));
    let mut force_special_chars = use_signal(|| false);
    let mut numbers = use_signal(|| NonNegativeInt(0));
    let mut current_language = use_signal(|| Option::<DicewareLanguage>::None);
    let mut settings_ready = use_signal(|| false);
    let mut settings_id = use_signal(|| -1i64);

    let options = language_options();

    let mut current_settings = use_resource(move || {
        let user_id = user_id.clone();
        let pool = pool.clone();
        let mut word_count = word_count.clone();
        let mut special_chars = special_chars.clone();
        let mut force_special_chars = force_special_chars.clone();
        let mut numbers = numbers.clone();
        let mut current_language = current_language.clone();
        let mut settings_id = settings_id.clone();
        let mut settings_ready = settings_ready.clone();
        let mut error = error.clone();
        async move {
            match fetch_diceware_settings(&pool, user_id).await {
                Ok(settings) => {
                    word_count.set(PositiveInt(settings.word_count as u32));
                    special_chars.set(NonNegativeInt(settings.special_chars as u32));
                    force_special_chars.set(settings.force_special_chars);
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
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching diceware settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    let on_submit = move |_| {
        let word_count = word_count();
        let special_chars = special_chars();
        let force_special_chars = force_special_chars();
        let numbers = numbers();
        let language = current_language().unwrap_or(DicewareLanguage::EN);
        let sid = settings_id();

        let settings = DicewareGenerationSettings {
            id: Some(sid),
            settings_id: sid,
            word_count: word_count.into(),
            special_chars: special_chars.into(),
            force_special_chars,
            numbers: numbers.into(),
            language,
        };

        let pool = pool_for_submit.clone();
        let toast = toast.clone();
        spawn(async move {
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

    let force_disabled: bool = special_chars().0 == 0;

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

                // Special chars
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Special chars" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::NonNegativeInt,
                        value: special_chars,
                        placeholder: String::new(),
                        label: String::new(),
                    }
                }

                // Force special chars toggle
                div { class: "flex flex-row justify-between mb-2",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Force special chars" }
                        }
                    }
                    Toggle {
                        checked: force_special_chars(),
                        onchange: move |_| force_special_chars.toggle(),
                        size: ToggleSize::Large,
                        color: ToggleColor::Success,
                        disabled: force_disabled,
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
