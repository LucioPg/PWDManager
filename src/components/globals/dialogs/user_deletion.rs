use dioxus::prelude::*;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use super::base_modal::ModalVariant;

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
                class: "alert alert-error mb-4",
                svg {
                    class: "w-6 h-6",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    path {
                        d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
                        "stroke-linecap": "round",
                        "stroke-linejoin": "round",
                        "stroke-width": "2"
                    }
                }
                span { class: "ml-2", "Zona Pericolosa" }
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
