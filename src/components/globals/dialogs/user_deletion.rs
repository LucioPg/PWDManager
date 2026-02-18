use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn UserDeletionDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Callback quando l'utente conferma la cancellazione
    on_confirm: EventHandler<()>,

    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,

    /// Username da mostrare nel messaggio di warning
    username: String,
) -> Element {
    // Cloni per le closure
    let mut open_clone = open.clone();

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open: open,
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

            // Icona di warning
            div {
                class: "alert alert-error mb-4 flex items-center justify-center mx-10",
                WarningIcon {
                    class: Some("w-6 h-6".to_string()),
                }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Elimina Account" }

            // Messaggio di conferma
            p {
                class: "py-4",
                "Sei sicuro di voler eliminare l'account di "
                strong { "{username}" }
                "? Questa azione non può essere annullata."
            }

            p {
                class: "text-error-600 py-2",
                strong { "Attenzione: " }
                "Tutti i tuoi dati verranno eliminati permanentemente."
            }

            // Action buttons
            div {
                class: "modal-action",

                ActionButton {
                    text: "Annulla".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }

                ActionButton {
                    text: "Elimina Account".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error-600 hover:bg-error-50".to_string(),
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
