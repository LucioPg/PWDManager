use crate::auth::{AuthState, User};
use crate::backend::db_backend::fetch_user_passwords_generation_settings;
use crate::backend::password_types_helper::{PasswordGeneratorConfig, PasswordPreset};
use crate::components::globals::toggle::{Toggle, ToggleColor, ToggleSize};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::combobox::{AnyPreset, Combobox};
use pwd_dioxus::form::{FormField, PositiveInt};
use pwd_dioxus::spinner::{Spinner, SpinnerSize};
use pwd_dioxus::{InputType, show_toast_error, use_toast};
use pwd_types::ExcludedSymbolSet;
use sqlx::SqlitePool;

fn preset_options() -> Vec<(&'static str, Option<AnyPreset>)> {
    vec![
        ("Medium", Some(AnyPreset::Standard(PasswordPreset::Medium))),
        ("Strong", Some(AnyPreset::Standard(PasswordPreset::Strong))),
        ("Epic", Some(AnyPreset::Standard(PasswordPreset::Epic))),
        ("God", Some(AnyPreset::Standard(PasswordPreset::God))),
        ("Custom", Some(AnyPreset::Custom)),
    ]
}

#[component]
pub fn StoredPasswordSettings(user_to_edit: Option<User>) -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let user_id = auth_state.get_user_id();
    let mut error = use_signal(|| <Option<String>>::None);
    let mut with_numbers = use_signal(|| true);
    let mut with_uppercase = use_signal(|| true);
    let mut with_lowercase = use_signal(|| true);
    let mut with_symbols = use_signal(|| PositiveInt(2));
    let mut with_excluded_symbols = use_signal(|| String::new());
    let mut with_length = use_signal(|| PositiveInt(26));
    let mut readonly = use_signal(|| true);
    let options = preset_options();
    let mut current_preset = use_signal(|| Option::<AnyPreset>::None);
    let mut settings_ready = use_signal(|| false);
    let mut settings_id = use_signal(|| -1);
    let mut current_settings = use_resource(move || {
        let user_id = user_id.clone();
        let pool = pool.clone();
        let mut with_numbers = with_numbers.clone();
        let mut with_uppercase = with_uppercase.clone();
        let mut with_lowercase = with_lowercase.clone();
        let mut with_symbols = with_symbols.clone();
        let mut with_excluded_symbols = with_excluded_symbols.clone();
        let mut with_length = with_length.clone();
        let mut settings_id = settings_id.clone();
        let mut settings_ready = settings_ready.clone();
        let mut error = error.clone();
        let mut readonly = readonly.clone();
        async move {
            let user_id = user_id.clone();
            match fetch_user_passwords_generation_settings(&pool, user_id).await {
                Ok(settings) => {
                    let s = settings.clone();
                    with_numbers.set(settings.numbers);
                    with_uppercase.set(settings.uppercase);
                    with_lowercase.set(settings.lowercase);
                    with_symbols.set(PositiveInt(settings.symbols as u32));
                    with_excluded_symbols.set(settings.excluded_symbols.into());
                    with_length.set(PositiveInt(settings.length as u32));
                    settings_id.set(settings.settings_id);
                    settings_ready.set(true);
                    readonly.set(false);

                    s
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    settings_ready.set(true);
                    readonly.set(false);
                    PasswordPreset::God.to_config(1) // dummy
                }
            }
        }
    });

    use_effect(move || {
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching password settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    use_effect(move || {
        let custom_preset = current_preset.clone();
        let mut readonly = readonly.clone();
        if let Some(preset) = custom_preset() {
            match preset {
                AnyPreset::Standard(preset) => {
                    let settings = preset.to_config(settings_id());
                    with_numbers.set(settings.numbers);
                    with_uppercase.set(settings.uppercase);
                    with_lowercase.set(settings.lowercase);
                    with_symbols.set(settings.symbols.into());
                    with_excluded_symbols.set(settings.excluded_symbols.into());
                    with_length.set(PositiveInt(settings.length as u32));
                    readonly.set(true);
                }
                AnyPreset::Custom => {
                    readonly.set(false);
                }
            }
        } else {
            readonly.set(false);
        }
    });

    let on_submit = move |_| {
        let with_numbers = with_numbers.clone();
        let with_uppercase = with_uppercase.clone();
        let with_lowercase = with_lowercase.clone();
        let with_symbols = with_symbols.clone();
        let with_excluded_symbols = with_excluded_symbols.clone();
        let with_length = with_length.clone();
        let settings_id = settings_id.clone();

        let result = PasswordGeneratorConfig {
            id: Some(settings_id()),
            settings_id: settings_id(),
            length: with_length().into(),
            symbols: with_symbols().into(),
            numbers: with_numbers(),
            uppercase: with_uppercase(),
            lowercase: with_lowercase(),
            excluded_symbols: ExcludedSymbolSet::from(with_excluded_symbols()),
        };
        println!("{:#?}", result);
    };
    if !settings_ready() {
        return rsx! {
            div { class: "flex flex-col gap-4",
                Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }
            }
        };
    }
    rsx! {

        form { class: "flex flex-col gap-4", onsubmit: on_submit,

            div { class: "flex  flex-col gap-4 rounded rounded-lg bg-base-200 p-4 rounded-box w-full",
                strong {
                    h2 { class: "text-center", "Generation Stored Password Settings " }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Presets" }
                        }
                    }
                    Combobox::<AnyPreset> {
                        options: options.clone(),
                        placeholder: "Select a preset".to_string(),
                        on_change: move |v| current_preset.set(v),
                    }
                }


                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text", "Include numbers" }
                        }
                    }
                    // Toggle con dimensione e colore personalizzati
                    Toggle {
                        checked: with_numbers(),
                        onchange: move |_| with_numbers.toggle(),
                        size: ToggleSize::Large,
                        color: ToggleColor::Success,
                        disabled: readonly(),
                    }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text strong", "Include lowercase" }
                        }
                    }
                    // Toggle con dimensione e colore personalizzati
                    Toggle {
                        checked: with_lowercase(),
                        onchange: move |_| with_lowercase.toggle(),
                        size: ToggleSize::Large,
                        color: ToggleColor::Success,
                        disabled: readonly(),
                    }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text strong", "Include uppercase" }
                        }
                    }
                    // Toggle con dimensione e colore personalizzati
                    Toggle {
                        checked: with_uppercase(),
                        onchange: move |_| with_uppercase.toggle(),
                        size: ToggleSize::Large,
                        color: ToggleColor::Success,
                        disabled: readonly(),
                    }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text strong", "Password length" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::PositiveInt,
                        value: with_length,
                        placeholder: String::new(),
                        label: String::new(),
                        readonly: readonly(),
                    }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text strong", "Symbols amount" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::PositiveInt,
                        value: with_symbols,
                        placeholder: String::new(),
                        label: String::new(),
                        readonly: readonly(),
                    }
                }
                div { class: "flex flex-row justify-between",
                    label { class: "label cursor-pointer",
                        strong {
                            span { class: "label-text strong", "Excluded symbols" }
                        }
                    }
                    FormField {
                        class: "min-w-[50%]",
                        input_type: InputType::Text,
                        value: with_excluded_symbols,
                        placeholder: String::new(),
                        label: String::new(),
                        forbid_spaces: true,
                        readonly: readonly(),
                    }
                }
            }
            ActionButton {
                text: "Save".to_string(),
                variant: ButtonVariant::Primary,
                button_type: ButtonType::Submit,
                size: ButtonSize::Normal,
                on_click: move |_| {},
            }
        }
    }
}
