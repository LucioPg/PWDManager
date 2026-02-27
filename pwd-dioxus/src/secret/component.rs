use crate::form::FormSecret;
use crate::icons::{ClipboardIcon, EyeIcon, EyeOffIcon};
use dioxus::prelude::*;
use secrecy::ExposeSecret;

/// Componente SecretDisplay - visualizza dati sensibili con toggle visibility
///
/// Usato per: password, location, e altri campi sensibili
#[component]
pub fn SecretDisplay(
    /// Il valore segreto da visualizzare (FormSecret wrappa SecretString)
    secret: FormSecret,
    /// Classe CSS aggiuntiva per il container (opzionale)
    #[props(default)]
    class: Option<String>,
    /// Larghezza massima del contenitore (default: 200px)
    #[props(default = "200px".to_string())]
    max_width: String,
    /// Callback quando si clicca sull'icona clipboard
    #[props(default)]
    on_copy: Option<EventHandler<()>>,
) -> Element {
    let mut visible = use_signal(|| false);

    let value_len = secret.expose_secret().len();
    let display_value = if value_len == 0 {
        String::new()
    } else if visible() {
        secret.expose_secret().to_string()
    } else {
        "•".repeat(value_len)
    };

    rsx! {
        div { class: "secret-display-wrapper {class.clone().unwrap_or_default()}",
            input {
                class: "pwd-secret-display font-mono",
                r#type: if visible() { "text" } else { "password" },
                value: "{display_value}",
                readonly: true,
                title: if visible() {
                    Some(secret.expose_secret().to_string())
                } else {
                    None
                },
                style: "max-width: {max_width}",
            }

            div { class: "secret-display-actions flex gap-1",
                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    onclick: move |_| visible.set(!visible()),
                    aria_label: if visible() { "Nascondi" } else { "Mostra" },
                    if visible() {
                        EyeOffIcon { class: Some("text-current".to_string()) }
                    } else {
                        EyeIcon { class: Some("text-current".to_string()) }
                    }
                }

                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    disabled: on_copy.is_none(),
                    aria_label: "Copia",
                    ClipboardIcon { class: Some("text-current".to_string()) }
                }
            }
        }
    }
}
