use crate::auth::User;
use crate::backend::password_types_helper::PasswordPreset;
use crate::components::globals::toggle::{Toggle, ToggleColor, ToggleSize};

use dioxus::prelude::*;
use pwd_dioxus::InputType;
use pwd_dioxus::form::{FormField, PositiveInt};
// Self::God => PasswordGeneratorConfig {
// id: Some(settings_id),
// settings_id,
// length: 26,
// symbols: 2,
// numbers: true,
// uppercase: true,
// lowercase: true,
// excluded_symbols: ExcludedSymbolSet::default(),
// },

#[component]
pub fn StoredPasswordSettings(user_to_edit: Option<User>) -> Element {
    let mut with_numbers = use_signal(|| true);
    let mut with_uppercase = use_signal(|| true);
    let mut with_lowercase = use_signal(|| true);
    let mut with_symbols = use_signal(|| PositiveInt(2));
    let mut with_excluded_symbols = use_signal(|| String::new());
    let mut with_length = use_signal(|| PositiveInt(26));

    rsx! {
        h3 { "Generation Stored Password Settings " }
        form { class: "flex ",
            div { class: "flex  flex-col gap-4 rounded rounded-lg bg-base-200 p-4 rounded-box w-full",
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
                    
                    }
                }
            }
        }
    }
}
#[component]
fn Base() -> Element {
    rsx! {
        div { "StoredPasswordSettings" }
    }
}

#[component]
fn Custom() -> Element {
    rsx! {
        div { "StoredPasswordSettings" }
    }
}
