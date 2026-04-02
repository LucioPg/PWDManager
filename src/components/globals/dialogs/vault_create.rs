// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::backend::vault_utils::create_vault;
use crate::auth::AuthState;
use crate::components::globals::auth_wrapper::ActiveVaultState;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant, show_toast_error, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;

#[component]
pub fn VaultCreateDialog(
    /// Controls the modal visibility
    open: Signal<bool>,

    /// Callback when a vault is successfully created
    on_created: EventHandler<()>,

    /// Callback when the user cancels
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let user_state = use_context::<AuthState>();
    let active_vault_state = use_context::<ActiveVaultState>();
    let mut name = use_signal(String::new);
    let mut description = use_signal(|| Option::<String>::None);
    let mut open_clone = open;
    let mut creating = use_signal(|| false);

    // Reset fields when dialog opens
    use_effect(move || {
        if open() {
            name.set(String::new());
            description.set(None);
            creating.set(false);
        }
    });

    let on_create = move |_| {
        let name_val = name();
        if name_val.trim().is_empty() {
            return;
        }
        creating.set(true);
        let pool = pool.clone();
        let user_id = user_state.get_user_id();
        let desc = description();
        let mut open = open_clone;
        let on_created = on_created;
        let mut active_vault = active_vault_state;
        let toast = toast;
        spawn(async move {
            match create_vault(&pool, user_id, name_val.trim().to_string(), desc).await {
                Ok(vault) => {
                    // Set as active vault if it's the first one
                    active_vault.0.set(vault.id);
                    open.set(false);
                    creating.set(false);
                    on_created.call(());
                }
                Err(e) => {
                    show_toast_error(format!("Failed to create vault: {}", e), toast);
                    creating.set(false);
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

            // Title
            h3 { class: "font-bold text-lg mb-4", "Create New Vault" }

            // Name field
            div { class: "form-control w-full",
                label { class: "label", span { class: "label-text", "Name *" } }
                input {
                    class: "input input-bordered w-full",
                    r#type: "text",
                    placeholder: "Vault name",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                }
            }

            // Description field
            div { class: "form-control w-full mt-4",
                label { class: "label", span { class: "label-text", "Description" } }
                input {
                    class: "input input-bordered w-full",
                    r#type: "text",
                    placeholder: "Optional description",
                    value: "{description().unwrap_or_default()}",
                    oninput: move |e| description.set(Some(e.value())),
                }
            }

            // Action buttons
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
                    text: "Create".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: on_create,
                }
            }
        }
    }
}
