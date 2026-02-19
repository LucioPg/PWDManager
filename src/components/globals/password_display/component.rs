use crate::components::globals::svgs::{EyeIcon, EyeOffIcon};
use crate::components::globals::form_field::FormSecret;
use dioxus::prelude::*;
use secrecy::ExposeSecret;

/// Componente PasswordDisplay - visualizza password con toggle visibility
///
/// # Esempio
/// ```rust
/// rsx! {
///     PasswordDisplay {
///         password: FormSecret(SecretString::new("my-password".into())),
///         max_width: "200px".to_string(),
///     }
/// }
/// ```
#[component]
pub fn PasswordDisplay(
    /// La password da visualizzare (FormSecret per sicurezza)
    password: FormSecret,
    /// Classe CSS aggiuntiva per il container (opzionale)
    #[props(default)]
    class: Option<String>,
    /// Larghezza massima del contenitore (default: 200px come in table_row)
    #[props(default = "200px".to_string())]
    max_width: String,
    /// Callback quando si clicca sull'icona clipboard (TODO: implementare copia)
    /// Se None, il button clipboard viene mostrato ma disabilitato
    #[props(default)]
    on_copy: Option<EventHandler<()>>,
) -> Element {
    // TODO: Implement component
    rsx! {
        div { "PasswordDisplay placeholder" }
    }
}
