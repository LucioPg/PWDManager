use crate::components::globals::svgs::{ClipboardIcon, EyeIcon, EyeOffIcon};
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
    /// Stato per la visibilità della password (false = nascosta/pallini)
    let mut password_visible = use_signal(|| false);

    // Calcola il valore da mostrare
    let password_len = password.expose_secret().len();
    let display_value = if password_len == 0 {
        String::new()
    } else if password_visible() {
        password.expose_secret().to_string()
    } else {
        "•".repeat(password_len)
    };

    rsx! {
        div { class: "password-display-wrapper {class.clone().unwrap_or_default()}",
            // Input password read-only con toggle visibility
            input {
                class: "pwd-password-display font-mono",
                r#type: if password_visible() { "text" } else { "password" },
                value: "{display_value}",
                readonly: true,
                title: if password_visible() {
                    Some(password.expose_secret().to_string())
                } else {
                    None
                },
                style: "max-width: {max_width}",
            }

            // Actions container (toggle + clipboard)
            div { class: "password-display-actions flex gap-1",
                // Toggle visibility button
                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    onclick: move |_| password_visible.set(!password_visible()),
                    aria_label: if password_visible() { "Nascondi password" } else { "Mostra password" },
                    if password_visible() {
                        EyeOffIcon { class: Some("text-current".to_string()) }
                    } else {
                        EyeIcon { class: Some("text-current".to_string()) }
                    }
                }

                // Copy to clipboard button (placeholder for future implementation)
                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    disabled: on_copy.is_none(),
                    aria_label: "Copia password",
                    // TODO: Implement clipboard functionality
                    ClipboardIcon { class: Some("text-current".to_string()) }
                }
            }
        }
    }
}
