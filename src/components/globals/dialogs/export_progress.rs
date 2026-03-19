use super::base_modal::ModalVariant;
use crate::components::ExportProgressChn;
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

/// Dialog che mostra il progresso dell'export.
///
/// Non può essere chiuso durante l'export (on_close vuoto).
#[component]
pub fn ExportProgressDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Signal che diventa true quando l'export è completato
    on_completed: Signal<bool>,

    /// Signal che diventa true se l'export fallisce
    #[props(default)]
    on_failed: Signal<bool>,
) -> Element {
    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {},
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Icona warning
            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Exporting Passwords" }

            // Messaggio
            p { class: "py-4", "Your passwords are being exported. Please wait..." }

            p { class: "text-warning py-2",
                "The dialog will close automatically when the export is complete."
            }

            ExportProgressChn { on_completed, on_failed }
        }
    }
}
