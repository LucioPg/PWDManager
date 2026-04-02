// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;
use pwd_types::Vault;

#[component]
pub fn VaultCard(
    vault: Vault,
    password_count: u64,
    on_edit: EventHandler<Vault>,
    on_delete: EventHandler<Vault>,
) -> Element {
    let password_word = if password_count == 1 {
        "password"
    } else {
        "passwords"
    };

    rsx! {
        div { class: "card card-side bg-base-100 shadow-sm pwd-vault-card",
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
                            onclick: move |_| on_edit.call(vault_for_edit.clone()),
                            "Edit"
                        }
                    }
                }
                {
                    let vault_for_delete = vault.clone();
                    rsx! {
                        button {
                            class: "btn btn-ghost btn-sm text-error",
                            onclick: move |_| on_delete.call(vault_for_delete.clone()),
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
