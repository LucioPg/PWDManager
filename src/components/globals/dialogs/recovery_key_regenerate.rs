use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeyRegenerateDialog(
    open: Signal<bool>,
    on_confirm: EventHandler<()>,
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
            class: "futuristic",

            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            h3 { class: "font-bold text-lg mb-2", "Regenerate recovery key?" }

            p { class: "py-4",
                "A new recovery key will be generated. Your data will not be lost, but the old recovery key will no longer work."
            }

            div { class: "modal-action",

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

                ActionButton {
                    text: "Regenerate".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-warning hover:bg-warning/10".to_string(),
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
