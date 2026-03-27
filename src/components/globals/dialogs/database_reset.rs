use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn DatabaseResetDialog(
    open: Signal<bool>,
    on_confirm: EventHandler<()>,
    #[props(default)] on_cancel: EventHandler<()>,
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

            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            div { class: "alert alert-error mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            h3 { class: "font-bold text-lg mb-2", "Reset database?" }

            p { class: "py-4", "All data will be permanently deleted." }

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
                    text: "Reset".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error hover:bg-error/10".to_string(),
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
