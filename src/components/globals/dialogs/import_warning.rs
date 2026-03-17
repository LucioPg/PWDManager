use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

/// Dialog di conferma per l'import delle password.
///
/// Mostra info sul comportamento dell'import:
/// - I duplicati (url+password) vengono saltati
/// - Password con stessa url ma password diversa vengono importate come nuove
#[component]
pub fn ImportWarningDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Path del file di import (solo display)
    input_path: String,

    /// Formato di import (solo display)
    format: String,

    /// Callback quando l'utente conferma l'import
    on_confirm: EventHandler<()>,

    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let mut open_clone = open.clone();

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,

            // Close button "X" in alto a destra
            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            // Icona warning
            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Import Passwords" }

            // Dettagli import
            p { class: "py-2", "You are about to import passwords from:" }
            p { class: "font-mono text-sm bg-base-200 p-2 rounded mb-2 break-all",
                "{input_path}"
            }
            p { class: "text-sm opacity-70 mb-4", "Format: {format}" }

            // Warning su duplicati
            p { class: "text-warning-600 py-2",
                strong { "Note: " }
                "Duplicate passwords (same url and password) in the file will be skipped. "
                "Passwords that already exist in your database will also be skipped."
            }

            p { class: "text-info-600 py-2",
                strong { "Info: " }
                "Passwords with the same url but different password will be imported as new entries."
            }

            // Action buttons
            div { class: "modal-action",

                ActionButton {
                    text: "Import".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
                ActionButton {
                    text: "Cancel".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
