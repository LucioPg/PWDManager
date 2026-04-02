// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::backend::vault_utils::update_vault;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant, show_toast_error, use_toast};
use dioxus::prelude::*;
use pwd_types::Vault;
use sqlx::SqlitePool;

#[component]
pub fn VaultEditDialog(
    /// Controls the modal visibility
    open: Signal<bool>,

    /// The vault being edited
    vault: Vault,

    /// Callback when the vault is successfully updated
    on_updated: EventHandler<()>,

    /// Callback when the user cancels
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let mut name = use_signal(String::new);
    let mut description = use_signal(|| Option::<String>::None);
    let mut open_clone = open;
    let mut updating = use_signal(|| false);

    // Clone vault fields for use across multiple closures
    let vault_name = vault.name.clone();
    let vault_description = vault.description.clone();
    let vault_id = vault.id;
    let vault_user_id = vault.user_id;
    let vault_created_at = vault.created_at.clone();

    // Pre-fill fields when dialog opens
    let effect_vault_name = vault_name.clone();
    let effect_vault_desc = vault_description.clone();
    use_effect(move || {
        if open() {
            name.set(effect_vault_name.clone());
            description.set(effect_vault_desc.clone());
            updating.set(false);
        }
    });

    let on_save = move |_| {
        let name_val = name();
        if name_val.trim().is_empty() {
            return;
        }
        updating.set(true);
        let pool = pool.clone();
        let desc = description();
        let mut open = open_clone;
        let on_updated = on_updated;
        let toast = toast;
        let created_at = vault_created_at.clone();
        spawn(async move {
            let updated_vault = Vault {
                id: vault_id,
                user_id: vault_user_id,
                name: name_val.trim().to_string(),
                description: desc,
                created_at,
            };
            match update_vault(&pool, updated_vault).await {
                Ok(()) => {
                    open.set(false);
                    updating.set(false);
                    on_updated.call(());
                }
                Err(e) => {
                    show_toast_error(format!("Failed to update vault: {}", e), toast);
                    updating.set(false);
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
            div { class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                p { class: "text-center", "Edit: \"{vault_name}\"" }
            }

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
                    text: "Save".to_string(),
                    variant: ButtonVariant::Success,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: on_save,
                }
            }
        }
    }
}
