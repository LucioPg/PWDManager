use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

/// Dialog di conferma per l'export delle password.
///
/// Mostra un warning che le password saranno esportate in chiaro.
#[component]
pub fn ExportWarningDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Path del file di export (solo display)
    output_path: String,

    /// Formato di export (solo display)
    format: String,

    /// Callback quando l'utente conferma l'export
    on_confirm: EventHandler<()>,

    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let mut open_clone = open;

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

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
            h3 { class: "font-bold text-lg mb-2", "Export Passwords" }

            // Dettagli export
            p { class: "py-2", "You are about to export your passwords to:" }
            p { class: "font-mono text-sm bg-base-200 p-2 rounded mb-2 break-all",
                "{output_path}"
            }
            p { class: "text-sm opacity-70 mb-4", "Format: {format}" }

            // Warning
            p { class: "text-warning py-2",
                strong { "Warning: " }
                "Your passwords will be exported in plaintext. Keep the file secure!"
            }

            // Action buttons
            div { class: "modal-action",

                ActionButton {
                    text: "Export".to_string(),
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
