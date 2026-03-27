use crate::backend::db_backend::rekey_database;
use crate::components::globals::WarningIcon;
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, RecoveryKeyRegenerateDialog,
    RecoveryKeySetupDialog,
};
use dioxus::prelude::*;
use pwd_dioxus::spinner::{Spinner, SpinnerSize};
use secrecy::ExposeSecret;

#[component]
pub fn RecoveryKeySettings() -> Element {
    let mut show_confirm_dialog = use_signal(|| false);
    let mut show_new_passphrase = use_signal(|| false);
    let mut new_passphrase = use_signal(|| String::new());
    let mut is_rekeying = use_signal(|| false);

    let handle_confirm = move |_| {
        show_confirm_dialog.set(true);
    };

    let mut handle_rekey = move |_| {
        show_confirm_dialog.set(false);
        is_rekeying.set(true);
        spawn(async move {
            match rekey_database().await {
                Ok(phrase) => {
                    new_passphrase.set(phrase.expose_secret().to_string());
                    show_new_passphrase.set(true);
                }
                Err(e) => {
                    tracing::error!("Rekey failed: {}", e);
                }
            }
            is_rekeying.set(false);
        });
    };

    rsx! {
        div { class: "space-y-6",

            // Info section
            div { class: "space-y-2",
                h3 { class: "font-semibold text-lg", "Recovery Key" }
                p { class: "text-sm opacity-70",
                    "The recovery key is a 6-word passphrase that allows you to restore access "
                    "to your database if the encryption key stored in the system keyring is lost or corrupted."
                }
            }

            // Warning
            div { class: "alert alert-warning",
                WarningIcon { class: Some("w-5 h-5 shrink-0".to_string()) }
                p { class: "text-sm",
                    "Regenerating the recovery key will create a new passphrase. "
                    "Your data will not be lost, but the old passphrase will no longer work."
                }
            }

            // Action button
            if is_rekeying() {
                div { class: "flex items-center gap-3",
                    Spinner { size: SpinnerSize::Small, color_class: "text-base-content" }
                    span { class: "text-sm opacity-70", "Re-encrypting database..." }
                }
            } else {
                ActionButton {
                    text: "Regenerate Recovery Key".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-warning hover:bg-warning/10".to_string(),
                    on_click: handle_confirm,
                }
            }
        }

        // Confirm dialog
        RecoveryKeyRegenerateDialog {
            open: show_confirm_dialog,
            on_confirm: move |_| handle_rekey(()),
            on_cancel: move |_| show_confirm_dialog.set(false),
        }

        // New passphrase dialog
        RecoveryKeySetupDialog {
            open: show_new_passphrase,
            passphrase: new_passphrase.read().clone(),
            on_confirm: move |_| show_new_passphrase.set(false),
        }
    }
}
