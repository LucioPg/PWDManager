// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::backend::vault_utils::delete_vault;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant, show_toast_error, use_toast};
use dioxus::prelude::*;
use pwd_types::Vault;
use sqlx::SqlitePool;

#[component]
pub fn VaultDeleteDialog(
    /// Controls the modal visibility
    open: Signal<bool>,

    /// The vault to delete
    vault: Vault,

    /// Number of passwords in the vault (for the warning message)
    password_count: u64,

    /// Callback when the vault is successfully deleted
    on_deleted: EventHandler<()>,

    /// Callback when the user cancels
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let mut open_clone = open;
    let mut deleting = use_signal(|| false);

    let vault_id = vault.id.unwrap_or(0);
    let vault_name = vault.name.clone();
    let password_word = if password_count == 1 { "password" } else { "passwords" };

    let on_delete = move |_| {
        if deleting() {
            return;
        }
        deleting.set(true);
        let pool = pool.clone();
        let mut open = open_clone;
        let on_deleted = on_deleted;
        let toast = toast;
        spawn(async move {
            match delete_vault(&pool, vault_id).await {
                Ok(()) => {
                    open.set(false);
                    deleting.set(false);
                    on_deleted.call(());
                }
                Err(e) => {
                    show_toast_error(format!("Failed to delete vault: {}", e), toast);
                    deleting.set(false);
                }
            }
        });
    };

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Close button "X"
            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "\u{2715}"
            }

            // Warning icon
            div { class: "alert alert-error mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Title
            h3 { class: "font-bold text-lg mb-2", "Delete Vault" }

            // Warning message
            p { class: "py-4",
                "Are you sure you want to delete the vault "
                strong { "{vault_name}" }
                " and all of its contents?"
            }

            if password_count > 0 {
                p { class: "text-error py-2",
                    strong { "Attention: " }
                    "This vault contains {password_count} {password_word}. "
                    "All stored passwords will be permanently deleted. This action cannot be undone."
                }
            } else {
                p { class: "text-warning py-2",
                    strong { "Attention: " }
                    "This vault is empty, but deletion cannot be undone."
                }
            }

            // Action buttons
            div { class: "modal-action",
                ActionButton {
                    text: "Delete".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-error hover:bg-error/10".to_string(),
                    on_click: on_delete,
                }
                ActionButton {
                    text: "Abort".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
