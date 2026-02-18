use super::svgs::{EyeIcon, EyeOffIcon};
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::ops::Deref;

/// Tipo di value per il FormField
#[derive(Clone)]
pub struct FormSecret(pub SecretString);

// Implementiamo PartialEq manualmente per il wrapper
impl PartialEq for FormSecret {
    fn eq(&self, other: &Self) -> bool {
        // Confronto sicuro tra i contenuti esposti
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Deref for FormSecret {
    type Target = SecretString;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FormValue for FormSecret {
    fn to_form_string(&self) -> String {
        self.0.expose_secret().to_string()
    }
    fn from_form_string(s: String) -> Option<Self> {
        // Passiamo direttamente la String 's' a SecretString::new
        // .into() non serve se s è già String, ma assicurati che
        // SecretString::new riceva ownership della stringa.
        Some(FormSecret(SecretString::new(s.into())))
    }
}

pub trait FormValue: Clone + PartialEq + 'static {
    fn to_form_string(&self) -> String;
    fn from_form_string(s: String) -> Option<Self>;
}

// Implementazione per String standard
impl FormValue for String {
    fn to_form_string(&self) -> String {
        self.clone()
    }
    fn from_form_string(s: String) -> Option<Self> {
        Some(s)
    }
}

// Implementazione per i32 (numeri)
impl FormValue for i32 {
    fn to_form_string(&self) -> String {
        self.to_string()
    }
    fn from_form_string(s: String) -> Option<Self> {
        s.parse().ok()
    }
}

// Implementazione per Option<String> (campi opzionali come notes)
impl FormValue for Option<String> {
    fn to_form_string(&self) -> String {
        self.clone().unwrap_or_default()
    }
    fn from_form_string(s: String) -> Option<Self> {
        // Restituisce Some(Some(String)) se non vuota, altrimenti Some(None)
        Some(if s.is_empty() { None } else { Some(s) })
    }
}

/// Tipo di input per il FormField
#[derive(Clone, PartialEq, Debug)]
pub enum InputType {
    Text,
    Textarea,
    Password,
    #[allow(dead_code)]
    Email,
    #[allow(dead_code)]
    Number,
    #[allow(dead_code)]
    Tel,
    #[allow(dead_code)]
    Url,
}

impl InputType {
    pub fn as_str(&self) -> &str {
        match self {
            InputType::Text => "text",
            InputType::Textarea => "text", // textarea non usa type, ma serve per compatibilità
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
    }

    /// Ritorna true se il tipo richiede un elemento textarea
    pub fn is_textarea(&self) -> bool {
        matches!(self, InputType::Textarea)
    }
}

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
                            // Icona "occhio chiuso" - password visibile
                            EyeOffIcon { class: Some("text-current".to_string()) }
                        } else {
                            // Icona "occhio aperto" - password nascosta
                            EyeIcon { class: Some("text-current".to_string()) }
                        }
                    }
                }
            }
        }
    } else if input_type.is_textarea() {
        // Layout per textarea
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
        // Layout standard per input
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
