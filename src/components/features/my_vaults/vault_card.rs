// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;
use pwd_types::Vault;

#[component]
pub fn VaultCard(
    vault: Vault,
    password_count: u64,
    #[props(default)] is_selected: bool,
    on_select: EventHandler<Vault>,
    on_edit: EventHandler<Vault>,
    on_delete: EventHandler<Vault>,
    #[props(default)] on_open: EventHandler<Vault>,
) -> Element {
    let password_word = if password_count == 1 {
        "password"
    } else {
        "passwords"
    };

    let vault_for_select = vault.clone();
    let vault_for_open = vault.clone();

    rsx! {
        div {
            class: format!(
                "card card-side bg-base-100 shadow-sm pwd-vault-card cursor-pointer{}",
                if is_selected { " pwd-vault-selected" } else { "" }
            ),
            onclick: move |_| on_select.call(vault_for_select.clone()),
            ondoubleclick: move |_| on_open.call(vault_for_open.clone()),
            div { class: "card-body p-4",
                h3 { class: "card-title text-base", "{vault.name}" }
                p { class: "text-sm text-base-content/60",
                    "{password_count} {password_word}"
                }
                if let Some(desc) = &vault.description {
                    if !desc.is_empty() {
                        p { class: "text-xs text-base-content/40 mt-1", "{desc}" }
                    }
                }
            }
            div { class: "card-actions flex-col gap-1 p-2",
                {
                    let vault_for_edit = vault.clone();
                    rsx! {
                        button {
                            class: "btn btn-ghost btn-sm",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                on_edit.call(vault_for_edit.clone());
                            },
                            ondoubleclick: move |evt| {
                                evt.stop_propagation();
                            },
                            "Edit"
                        }
                    }
                }
                {
                    let vault_for_delete = vault.clone();
                    rsx! {
                        button {
                            class: "btn btn-ghost btn-sm text-error",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                on_delete.call(vault_for_delete.clone());
                            },
                            ondoubleclick: move |evt| {
                                evt.stop_propagation();
                            },
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
