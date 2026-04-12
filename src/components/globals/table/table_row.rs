// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::components::StoredPasswordShowDialogState;
use crate::components::features::dashboard::{
    DeleteStoredPasswordDialogState, StoredPasswordUpsertDialogState,
};
use crate::components::globals::password_handler::StrengthAnalyzer;
use crate::components::globals::svgs::{BurgerIcon, DeleteIcon, EditIcon};
use dioxus::prelude::*;
use pwd_types::{PasswordScore, StoredRawPassword};
use std::collections::HashSet;

/// Props for the StoredRawPasswordRow component
#[derive(Props, Clone, PartialEq)]
pub struct StoredRawPasswordRowProps {
    /// The password data to display
    pub stored_raw_password: StoredRawPassword,
    /// Set of selected password IDs
    pub selected_ids: Signal<HashSet<i64>>,
    /// Callback when checkbox is toggled - passes (id, is_checked)
    pub on_select: EventHandler<(i64, bool)>,
    /// Callback when edit button is clicked
    pub on_edit: EventHandler<StoredRawPassword>,
    /// Callback when delete button is clicked
    pub on_delete: EventHandler<i64>,
    /// Callback when user clicks on burger button - passa (password, x, y)
    pub on_show_tooltip: EventHandler<(StoredRawPassword, f64, f64)>,
}

#[component]
pub fn StoredRawPasswordRow(props: StoredRawPasswordRowProps) -> Element {
    let password_id = props.stored_raw_password.id.unwrap_or(0);
    let store_raw_password_clone = props.stored_raw_password.clone();
    let password_for_edit = props.stored_raw_password.clone();
    let password_for_show = props.stored_raw_password.clone();
    let mut stored_password_dialog_state = use_context::<StoredPasswordUpsertDialogState>();
    let mut stored_password_show_dialog_state = use_context::<StoredPasswordShowDialogState>();
    let mut deletion_password_dialog_state = use_context::<DeleteStoredPasswordDialogState>();
    // Get strength from score for StrengthAnalyzer
    let strength =
        PasswordScore::get_strength(store_raw_password_clone.score.map(|s| s.value() as i64));
    rsx! {
        tr {
            key: "{password_id}",
            class: "stored-password-row hover:bg-base-200/50 transition-colors",

            // Column 0: Checkbox for multi-select
            td { class: "pwd-table__col-checkbox",
                input {
                    r#type: "checkbox",
                    checked: props
                        .stored_raw_password
                        .id
                        .is_some_and(|id| props.selected_ids.read().contains(&id)),
                    onchange: move |_| {
                        let is_checked = !props.selected_ids.read().contains(&password_id);
                        props.on_select.call((password_id, is_checked));
                    },
                }
            }

            // Column 1: url (visualizzazione sicura con toggle)
            td { class: "pwd-table__cell-content",
                p { class: "pwd-table__cell-content-label", "{props.stored_raw_password.name}" }
            }

            // Column 2: Burger button (tooltip for notes and created_at) - nascosto su mobile
            td { class: "pwd-table__col-info",
                div { class: "relative",
                    button {
                        class: "pwd-row-action-btn pwd-burger-btn",
                        r#type: "button",
                        onclick: move |_| {
                            stored_password_show_dialog_state
                                .current_stored_raw_password
                                .set(Some(password_for_show.clone()));
                            stored_password_show_dialog_state.is_open.set(true);
                        },
                        BurgerIcon {}
                    }
                }
            }
            // Column 3: Score (using StrengthAnalyzer without bar) - nascosto su mobile
            td { class: "pwd-table__col-strength",
                StrengthAnalyzer {
                    strength,
                    reasons: vec![], // No reasons tooltip in table view
                    is_evaluating: false,
                    score: store_raw_password_clone.score,
                    show_bar: false,
                }
            }




            // Column 4: Edit button (gear icon, yellow warning background)
            td { class: "pwd-table__col-actions",
                button {
                    class: "pwd-row-action-btn pwd-edit-btn",
                    r#type: "button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        stored_password_dialog_state
                            .current_stored_raw_password
                            .set(Some(password_for_edit.clone()));
                        stored_password_dialog_state.is_open.set(true);
                    },
                    EditIcon { size: "12".to_string() }
                }
            }

            // Column 5: Delete button (trash outline)
            td { class: "pwd-table__col-actions",
                button {
                    class: "pwd-row-action-btn pwd-delete-btn",
                    r#type: "button",
                    onclick: move |_| {
                        deletion_password_dialog_state.is_open.set(true);
                        deletion_password_dialog_state.password_id.set(Some(password_id));
                        props.on_delete.call(password_id)
                    },
                    DeleteIcon { size: "12".to_string() }
                }
            }
        }
    }
}
