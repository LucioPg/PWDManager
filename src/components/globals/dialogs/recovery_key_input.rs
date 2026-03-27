use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::html::input_data::keyboard_types::Code;
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeyInputDialog(
    open: Signal<bool>,
    error: Signal<bool>,
    on_recover: EventHandler<String>,
    on_reset: EventHandler<()>,
) -> Element {
    #[allow(redundant_closure)]
    let mut input_value = use_signal(|| String::new());

    let on_recover_clone = on_recover;
    let mut input_value_clone = input_value;

    let on_recover_clone2 = on_recover;
    let mut input_value_clone2 = input_value;

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {},
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Title
            h3 { class: "font-bold text-lg mb-2", "Recovery Key" }

            // Info text
            p { class: "py-2", "The encryption key is not available or invalid." }
            p { class: "py-2", "Enter your recovery key to restore access." }

            // Text input
            input {
                class: "input input-bordered w-full my-4 font-mono",
                r#type: "text",
                autocomplete: "off",
                aria_label: "Recovery key",
                placeholder: "Enter your recovery key...",
                value: "{input_value}",
                oninput: move |e| {
                    input_value.set(e.value());
                    error.set(false);
                },
                onkeydown: move |e: KeyboardEvent| {
                    if e.code() == Code::Enter {
                        let passphrase = input_value_clone.read().clone();
                        if !passphrase.trim().is_empty() {
                            on_recover_clone.call(passphrase);
                            input_value_clone.set(String::new());
                        }
                    }
                },
            }

            // Error message (conditional)
            if error() {
                p { class: "text-error py-2 text-sm", "Invalid recovery key. Please try again." }
            }

            // Action buttons
            div { class: "modal-action",

                ActionButton {
                    text: "Reset database".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error hover:bg-error/10".to_string(),
                    on_click: move |_| {
                        on_reset.call(());
                    },
                }

                ActionButton {
                    text: "Recover".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        let passphrase = input_value_clone2.read().clone();
                        if !passphrase.trim().is_empty() {
                            on_recover_clone2.call(passphrase);
                            input_value_clone2.set(String::new());
                        }
                    },
                }
            }
        }
    }
}
