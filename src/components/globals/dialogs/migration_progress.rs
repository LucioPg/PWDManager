use super::base_modal::ModalVariant;
use crate::components::ProgressMigrationChn;
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

#[component]
pub fn MigrationProgressDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Signal che diventa true quando la migrazione è completata
    on_completed: Signal<bool>,

    /// Signal che diventa true se la migrazione fallisce
    #[props(default)]
    on_failed: Signal<bool>,

    #[props(default)] on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        crate::components::globals::dialogs::BaseModal { open, on_close: move |_| {}, variant: ModalVariant::Middle,

            // Icona di warning
            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Password Migration" }

            // Messaggio di conferma
            p { class: "py-4",
                "The system is updating your data. Please wait until the process is completed."
            }

            p { class: "text-warning py-2",
                strong { "Attention: " }
                "Do not close the app or shut down the computer until the process is completed "
                "or you will lose access to your data!"
            }
            ProgressMigrationChn { on_completed, on_failed }
        }
    }
}
