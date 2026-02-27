use super::types::{FormValue, InputType};
use crate::icons::{EyeIcon, EyeOffIcon};
use dioxus::prelude::*;

/// Componente campo form riutilizzabile e configurabile
///
/// # Esempio
/// ```rust
/// FormField {
///     label: "Username".to_string(),
///     input_type: InputType::Text,
///     placeholder: "Enter your username".to_string(),
///     value: username,
///     name: Some("username".to_string()),
///     required: true,
///     disabled: false,
/// }
/// ```
#[component]
pub fn FormField<T: FormValue>(
    /// Etichetta del campo
    label: String,
    /// Tipo di input
    input_type: InputType,
    /// Testo placeholder
    placeholder: String,
    /// Signal per il valore del campo
    value: Signal<T>,
    /// Nome del campo (utile per form submission)
    #[props(default)]
    name: Option<String>,
    /// Se il campo è richiesto
    #[props(default)]
    required: bool,
    /// Se il campo è disabilitato
    #[props(default)]
    disabled: bool,
    /// Classe CSS aggiuntiva per il container
    #[props(default)]
    class: Option<String>,
    /// Readonly attribute
    #[props(default)]
    readonly: bool,
    #[props(default)] autocomplete: bool,
    /// Callback chiamato quando il valore cambia (opzionale)
    #[props(default)]
    on_change: Option<Callback<T>>,
    /// Mostra il pulsante per toggle visibilità password (solo per InputType::Password)
    #[props(default)]
    show_visibility_toggle: bool,
    /// Rifiuta spazi nel campo
    #[props(default)]
    forbid_spaces: bool,
    /// Accetta solo caratteri alfanumerici (lettere e numeri)
    #[props(default)]
    alphanumeric_only: bool,
) -> Element {
    let input_class = "pwd-input";

    // Funzione di filtro per i caratteri
    let filter_input = move |input: String| -> String {
        let mut result = input;
        if forbid_spaces {
            result = result.chars().filter(|c| !c.is_whitespace()).collect();
        }
        if alphanumeric_only {
            result = result.chars().filter(|c| c.is_alphanumeric()).collect();
        }
        result
    };

    // Stato per la visibilità della password
    let mut password_visible = use_signal(|| false);

    // Determina il tipo di input effettivo
    let effective_type = if input_type == InputType::Password && password_visible() {
        "text"
    } else {
        input_type.as_str()
    };

    // Se c'è il toggle di visibilità, usa un layout con wrapper
    if show_visibility_toggle && input_type == InputType::Password {
        rsx! {
            div { class: if let Some(custom_class) = class {
                format!("form-group {}", custom_class)
            } else {
                "form-group".to_string()
            },
                label { class: "form-label",
                    "{label}"
                    if required {
                        span { class: "text-error ml-1", "*" }
                    }
                }
                div { class: "password-input-wrapper",
                    input {
                        class: "{input_class} password-input-with-toggle",
                        r#type: "{effective_type}",
                        placeholder: "{placeholder}",
                        value: "{value.read().to_form_string()}",
                        oninput: move |e| {
                            let filtered = filter_input(e.value());
                            if let Some(new_value) = T::from_form_string(filtered) {
                                value.set(new_value.clone());
                                if let Some(callback) = on_change {
                                    callback.call(new_value);
                                }
                            }
                        },
                        disabled: disabled,
                        readonly: readonly,
                        name: name,
                        required: required,
                        autocomplete: if autocomplete { "on" } else { "off" },
                    }
                    button {
                        class: "password-visibility-toggle",
                        r#type: "button",
                        onclick: move |_| {
                            password_visible.set(!password_visible());
                        },
                        disabled: disabled || readonly,
                        aria_label: if password_visible() { "Nascondi password" } else { "Mostra password" },
                        if password_visible() {
                            EyeOffIcon { class: Some("text-current".to_string()) }
                        } else {
                            EyeIcon { class: Some("text-current".to_string()) }
                        }
                    }
                }
            }
        }
    } else if input_type.is_textarea() {
        rsx! {
            div { class: if let Some(custom_class) = class {
                format!("form-group {}", custom_class)
            } else {
                "form-group".to_string()
            },
                label { class: "form-label",
                    "{label}"
                    if required {
                        span { class: "text-error ml-1", "*" }
                    }
                }
                textarea {
                    class: "{input_class}",
                    placeholder: "{placeholder}",
                    value: "{value.read().to_form_string()}",
                    oninput: move |e| {
                        let filtered = filter_input(e.value());
                        if let Some(new_value) = T::from_form_string(filtered) {
                            value.set(new_value.clone());
                            if let Some(callback) = on_change {
                                callback.call(new_value);
                            }
                        }
                    },
                    disabled: disabled,
                    readonly: readonly,
                    name: name,
                    required: required,
                    autocomplete: if autocomplete { "on" } else { "off" },
                }
            }
        }
    } else {
        rsx! {
            div { class: if let Some(custom_class) = class {
                format!("form-group {}", custom_class)
            } else {
                "form-group".to_string()
            },
                label { class: "form-label",
                    "{label}"
                    if required {
                        span { class: "text-error ml-1", "*" }
                    }
                }
                input {
                    class: "{input_class}",
                    r#type: "{effective_type}",
                    placeholder: "{placeholder}",
                    value: "{value.read().to_form_string()}",
                    oninput: move |e| {
                        let filtered = filter_input(e.value());
                        if let Some(new_value) = T::from_form_string(filtered) {
                            value.set(new_value.clone());
                            if let Some(callback) = on_change {
                                callback.call(new_value);
                            }
                        }
                    },
                    disabled: disabled,
                    readonly: readonly,
                    name: name,
                    required: required,
                    autocomplete: if autocomplete { "on" } else { "off" },
                }
            }
        }
    }
}
