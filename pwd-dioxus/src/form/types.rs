use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::ops::Deref;

/// Wrapper per SecretString usato nei form
#[derive(Clone)]
pub struct FormSecret(pub SecretString);

impl PartialEq for FormSecret {
    fn eq(&self, other: &Self) -> bool {
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
        Some(FormSecret(SecretString::new(s.into())))
    }
}

/// Trait per valori usabili nei form
pub trait FormValue: Clone + PartialEq + 'static {
    fn to_form_string(&self) -> String;
    fn from_form_string(s: String) -> Option<Self>;
}

impl FormValue for String {
    fn to_form_string(&self) -> String { self.clone() }
    fn from_form_string(s: String) -> Option<Self> { Some(s) }
}

impl FormValue for i32 {
    fn to_form_string(&self) -> String { self.to_string() }
    fn from_form_string(s: String) -> Option<Self> { s.parse().ok() }
}

impl FormValue for Option<String> {
    fn to_form_string(&self) -> String { self.clone().unwrap_or_default() }
    fn from_form_string(s: String) -> Option<Self> {
        Some(if s.is_empty() { None } else { Some(s) })
    }
}

/// Tipo di input per FormField
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
    pub fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Textarea => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
    }

    pub fn is_textarea(&self) -> bool {
        matches!(self, InputType::Textarea)
    }
}
