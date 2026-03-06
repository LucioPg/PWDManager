use super::base_modal::ModalVariant;
use crate::components::ImportProgressChn;
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

/// Dialog che mostra il progresso dell'import.
///
/// Non può essere chiuso durante l'import (on_close vuoto).
#[component]
pub fn ImportProgressDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Signal che diventa true quando l'import è completato
    on_completed: Signal<bool>,

    /// Signal che diventa true se l'import fallisce
    #[props(default)]
    on_failed: Signal<bool>,
) -> Element {
    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {},
            variant: ModalVariant::Middle,

            // Icona warning
            div {
                class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Importing Passwords" }

            // Messaggio
            p { class: "py-4",
                "Your passwords are being imported. Please wait..."
            }

            p { class: "text-warning-600 py-2",
                "The dialog will close automatically when the import is complete."
            }

            ImportProgressChn { on_completed, on_failed }
        }
    }
}
