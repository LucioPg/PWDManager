// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::auth::AuthState;
use crate::backend::vault_utils::create_vault;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant, VaultListState, show_toast_error, use_toast};
use dioxus::prelude::*;
use pwd_dioxus::Combobox;
use pwd_types::StoredRawPassword;
use sqlx::SqlitePool;

const MAX_DISPLAYED_NAMES: usize = 5;

#[derive(Clone, Copy, PartialEq)]
pub enum VaultAction {
    Move,
    Clone,
}

#[component]
pub fn VaultActionDialog(
    open: Signal<bool>,
    action: VaultAction,
    selected_passwords: Vec<StoredRawPassword>,
    current_vault_id: i64,
    on_confirm: EventHandler<i64>,
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let user_state = use_context::<AuthState>();
    let user_id = user_state.get_user_id();
    let toast = use_toast();
    let mut open_clone = open;
    let mut target_vault_id: Signal<Option<i64>> = use_signal(|| None);
    let mut show_inline_form: Signal<bool> = use_signal(|| false);
    let mut new_vault_name: Signal<String> = use_signal(String::new);
    let mut new_vault_desc: Signal<Option<String>> = use_signal(|| None);
    let mut is_creating: Signal<bool> = use_signal(|| false);

    // Reset all state when dialog opens
    use_effect(move || {
        if open() {
            target_vault_id.set(None);
            show_inline_form.set(false);
            new_vault_name.set(String::new());
            new_vault_desc.set(None);
            is_creating.set(false);
        }
    });

    // Shared vault list from context (provided by AuthWrapper)
    let vaults_resource = use_context::<VaultListState>().0;

    // Build combobox options
    // Move: filter out current vault. Clone: include all vaults.
    let vault_options = use_memo(move || {
        let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        let opts: Vec<(&'static str, Option<i64>)> = vaults
            .iter()
            .filter(|v| {
                if action == VaultAction::Move {
                    v.id.is_some_and(|id| id != current_vault_id)
                } else {
                    true
                }
            })
            .map(|v| {
                let name = Box::leak(v.name.clone().into_boxed_str()) as &'static str;
                (name, v.id)
            })
            .collect();
        opts
    });

    // Derived labels from action
    let action_label = match action {
        VaultAction::Move => "Move",
        VaultAction::Clone => "Clone",
    };
    let count = selected_passwords.len();
    #[rustfmt::skip]
    let title = format!("{action_label} {count} {} to vault", if count == 1 { "password" } else { "passwords" });
    let confirm_btn_text = action_label.to_string();
    let create_and_confirm_text = format!("Create & {action_label}");
    let mut is_create_disabled: Signal<bool> = use_signal(|| false);
    let is_creating_ref = is_creating;
    let new_vault_name_ref = new_vault_name;
    use_effect(move || {
        is_create_disabled.set(is_creating_ref() || new_vault_name_ref().trim().is_empty());
    });

    // Displayed password names (max 5, then "and X more...")
    let display_names: Vec<String> = selected_passwords
        .iter()
        .take(MAX_DISPLAYED_NAMES)
        .map(|p| p.name.clone())
        .collect();
    let remaining = count.saturating_sub(MAX_DISPLAYED_NAMES);

    let on_action = move |_| {
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

    let on_create_and_action = move |_| {
        let name_val = new_vault_name();
        if name_val.trim().is_empty() {
            return;
        }
        is_creating.set(true);
        let pool = pool.clone();
        let user_id = user_id;
        let desc = new_vault_desc();
        let mut open = open_clone;
        let on_confirm = on_confirm;
        let toast = toast;
        let mut creating = is_creating;
        spawn(async move {
            match create_vault(&pool, user_id, name_val.trim().to_string(), desc).await {
                Ok(vault) => {
                    let new_vault_id = vault.id.unwrap_or(0);
                    on_confirm.call(new_vault_id);
                    open.set(false);
                }
                Err(e) => {
                    creating.set(false);
                    show_toast_error(format!("Failed to create vault: {}", e), toast);
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
            h3 { class: "font-bold text-lg mb-4", "{title}" }

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
                if !show_inline_form() {
                    // Combobox + "+ New Vault" button row
                    div { class: "flex gap-2 items-center",
                        div { class: "flex-1",
                            Combobox::<i64> {
                                options: vault_options(),
                                placeholder: "Select vault...".to_string(),
                                on_change: move |v| {
                                    target_vault_id.set(v);
                                },
                            }
                        }
                        button {
                            class: "btn btn-sm btn-outline btn-primary",
                            r#type: "button",
                            onclick: move |_| {
                                show_inline_form.set(true);
                            },
                            "+ New Vault"
                        }
                    }
                } else {
                    // Inline new vault form
                    div {
                        div { class: "flex gap-2 items-center mb-2",
                            div { class: "flex-1",
                                input {
                                    class: "input input-bordered input-sm w-full",
                                    r#type: "text",
                                    placeholder: "New vault name...",
                                    value: "{new_vault_name}",
                                    oninput: move |e| new_vault_name.set(e.value()),
                                }
                            }
                        }
                        input {
                            class: "input input-bordered input-sm w-full",
                            r#type: "text",
                            placeholder: "Description (optional)",
                            value: "{new_vault_desc().unwrap_or_default()}",
                            oninput: move |e| new_vault_desc.set(Some(e.value())),
                        }
                    }
                }
            }

            // Action buttons
            div { class: "modal-action",
                if show_inline_form() {
                    // Inline form buttons
                    ActionButton {
                        text: "\u{2190} Back".to_string(),
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            show_inline_form.set(false);
                        },
                    }
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
                        text: create_and_confirm_text.clone(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        disabled: is_create_disabled,
                        on_click: on_create_and_action,
                    }
                } else {
                    // Normal combobox buttons
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
                        text: confirm_btn_text.clone(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: on_action,
                    }
                }
            }
        }
    }
}
