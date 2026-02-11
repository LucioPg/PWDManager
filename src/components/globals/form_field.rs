use dioxus::prelude::*;

/// Tipo di input per il FormField
#[derive(Clone, PartialEq, Debug)]
pub enum InputType {
    Text,
    Password,
    Email,
    Number,
    Tel,
    Url,
}

impl InputType {
    pub fn as_str(&self) -> &str {
        match self {
            InputType::Text => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
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
pub fn FormField(
    /// Etichetta del campo
    label: String,
    /// Tipo di input
    input_type: InputType,
    /// Testo placeholder
    placeholder: String,
    /// Signal per il valore del campo
    value: Signal<String>,
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
) -> Element {
    let input_class = if readonly {
        "input-base input-readonly"
    } else if disabled {
        "input-base input-disabled"
    } else {
        "input-base"
    };

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
                r#type: "{input_type.as_str()}",
                placeholder: "{placeholder}",
                value: "{value.read()}",
                oninput: move |e| value.set(e.value()),
                disabled: disabled,
                readonly: readonly,
                name: name,
                required: required,
                autocomplete: if autocomplete { "on" } else { "off" },
            }
        }
    }
}
