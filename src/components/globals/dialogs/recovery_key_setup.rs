// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn RecoveryKeySetupDialog(
    open: Signal<bool>,
    passphrase: String,
    on_confirm: EventHandler<()>,
) -> Element {
    // Non-dismissable: no X button, no cancel
    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {}, // Intentionally empty — non-dismissable
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Title
            h3 { class: "font-bold text-lg mb-4 text-center", "Recovery Key" }

            // Info text
            p { class: "py-2 text-center",
                "Save these words in a safe place."
            }
            p { class: "py-2 text-center",
                "You will need them if the encryption key is lost."
            }

            // Passphrase display
            div { class: "bg-base-300 rounded-lg p-4 my-4 mx-4 text-center font-mono text-lg break-all select-all",
                "{passphrase}"
            }

            // Warning
            p { class: "text-warning py-2 text-center text-sm",
                strong { "Warning: " }
                "Without this recovery key, your data will be permanently lost if the encryption key is lost."
            }

            // Action button
            div { class: "modal-action",
                ActionButton {
                    text: "I have saved the recovery key".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_confirm.call(());
                        open.set(false);
                    },
                }
            }
        }
    }
}
