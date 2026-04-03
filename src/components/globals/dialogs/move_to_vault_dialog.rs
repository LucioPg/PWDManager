// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::auth::AuthState;
use crate::backend::vault_utils::fetch_vaults_by_user;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::Combobox;
use pwd_types::StoredRawPassword;
use sqlx::SqlitePool;

const MAX_DISPLAYED_NAMES: usize = 5;

#[component]
pub fn MoveToVaultDialog(
    /// Controls the modal visibility
    open: Signal<bool>,

    /// IDs of the selected passwords
    selected_ids: Vec<i64>,

    /// The selected password entries (for display purposes)
    selected_passwords: Vec<StoredRawPassword>,

    /// The current vault ID (will be excluded from target options)
    current_vault_id: i64,

    /// Callback when the user confirms the move - passes the target vault_id
    on_confirm: EventHandler<i64>,

    /// Callback when the user cancels
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let user_state = use_context::<AuthState>();
    let user_id = user_state.get_user_id();
    let mut open_clone = open;
    let mut target_vault_id: Signal<Option<i64>> = use_signal(|| None);

    // Reset state when dialog opens
    let effect_open = open;
    use_effect(move || {
        if effect_open() {
            target_vault_id.set(None);
        }
    });

    // Fetch vault list (excluding current vault)
    let vaults_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id;
        async move {
            if user_id == -1 {
                return Vec::new();
            }
            fetch_vaults_by_user(&pool, user_id)
                .await
                .unwrap_or_default()
        }
    });

    // Build combobox options, excluding the current vault
    let vault_options = use_memo(move || {
        let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        let opts: Vec<(&'static str, Option<i64>)> = vaults
            .iter()
            .filter(|v| v.id.is_some_and(|id| id != current_vault_id))
            .map(|v| {
                let name = Box::leak(v.name.clone().into_boxed_str()) as &'static str;
                (name, v.id)
            })
            .collect();
        opts
    });

    let count = selected_passwords.len();

    // Build the displayed password names (max 5, then "and X more...")
    let display_names: Vec<String> = selected_passwords
        .iter()
        .take(MAX_DISPLAYED_NAMES)
        .map(|p| p.name.clone())
        .collect();
    let remaining = count.saturating_sub(MAX_DISPLAYED_NAMES);

    let on_move = move |_| {
        let Some(target_id) = target_vault_id() else {
            return;
        };
        let mut open = open_clone;
        let on_confirm = on_confirm;
        spawn(async move {
            on_confirm.call(target_id);
            open.set(false);
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
            h3 { class: "font-bold text-lg mb-4",
                "Move {count} password{if count != 1 { "s
                " }} to vault"
            }

            // Selected passwords list
            if !display_names.is_empty() {
                div { class: "mb-4",
                    p { class: "text-sm opacity-70 mb-1", "Selected passwords:" }
                    ul { class: "list-disc list-inside text-sm space-y-1",
                        for name in display_names.iter() {
                            li { "{name}" }
                        }
                        if remaining > 0 {
                            li { class: "opacity-60", "and {remaining} more..." }
                        }
                    }
                }
            }

            // Target vault selector
            div { class: "form-control w-full mb-6",
                label { class: "label",
                    span { class: "label-text", "Target vault" }
                }
                Combobox::<i64> {
                    options: vault_options(),
                    placeholder: "Select vault...".to_string(),
                    on_change: move |v| {
                        target_vault_id.set(v);
                    },
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
                    text: "Move".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: on_move,
                }
            }
        }
    }
}
