use crate::backend::password_types_helper::{PasswordScore, StoredRawPassword};
use crate::components::globals::form_field::FormSecret;
use crate::components::globals::password_display::PasswordDisplay;
use crate::components::globals::password_handler::StrengthAnalyzer;
use crate::components::globals::svgs::{BurgerIcon, DeleteIcon, EditIcon};
use dioxus::prelude::*;
use secrecy::ExposeSecret;

/// Props for the StoredRawPasswordRow component
#[derive(Props, Clone, PartialEq)]
pub struct StoredRawPasswordRowProps {
    /// The password data to display
    pub stored_raw_password: StoredRawPassword,
    /// Callback when edit button is clicked
    pub on_edit: EventHandler<StoredRawPassword>,
    /// Callback when delete button is clicked
    pub on_delete: EventHandler<i64>,
    // /// Callback when user clicks on burger button
    // pub on_click: EventHandler<StoredRawPassword>,
}

#[component]
pub fn StoredRawPasswordRow(props: StoredRawPasswordRowProps) -> Element {
    let mut show_info_tooltip = use_signal(|| false);
    let password_id = props.stored_raw_password.id.unwrap_or(0);
    let store_raw_password_clone = props.stored_raw_password.clone();
    // Get strength from score for StrengthAnalyzer
    let strength =
        PasswordScore::get_strength(props.stored_raw_password.score.map(|s| s.value() as i64));
    let mut current = use_context::<Signal<Option<StoredRawPassword>>>();
    rsx! {
        tr {
            key: "{password_id}",
            onclick: move |_| current.set(Some(store_raw_password_clone.clone())),
            class: "stored-password-row hover:bg-base-200/50 transition-colors",

            // Column 1: Location (with ellipsis)
            td { class: "px-4 py-3",
                div { class: "truncate max-w-[150px]", title: "{props.stored_raw_password.location}",
                    "{props.stored_raw_password.location}"
                }
            }

            // Column 2: Password (visualizzazione sicura con toggle)
            td { class: "px-4 py-3",
                PasswordDisplay {
                    password: FormSecret(props.stored_raw_password.password.clone()),
                    max_width: "200px".to_string(),
                }
            }

            // Column 3: Score (using StrengthAnalyzer without bar)
            td { class: "px-4 py-3",
                StrengthAnalyzer {
                    strength: strength,
                    reasons: vec![], // No reasons tooltip in table view
                    is_evaluating: false,
                    score: props.stored_raw_password.score,
                    show_bar: false,
                }
            }

            // Column 4: Burger button (tooltip for notes and created_at)
            td { class: "px-2 py-3",
                div { class: "relative",
                    button {
                        class: "pwd-row-action-btn pwd-burger-btn",
                        r#type: "button",
                        onclick: move |_| show_info_tooltip.set(!show_info_tooltip()),
                        // Burger icon (three horizontal lines)
                        BurgerIcon {}
                    }

                    // Tooltip dropdown
                    if show_info_tooltip() {
                        // Overlay to close tooltip on click outside
                        div {
                            class: "fixed inset-0 z-[5]",
                            onclick: move |_| show_info_tooltip.set(false),
                        }

                        div { class: "pwd-row-tooltip absolute right-0 top-full mt-2 z-10",
                            div { class: "dropdown-content mockup-code bg-base-200 shadow-lg rounded-lg p-3 min-w-[200px] max-w-[280px]",
                                // Notes section
                                if let Some(notes) = &props.stored_raw_password.notes {
                                    if !notes.is_empty() {
                                        div { class: "mb-3",
                                            h4 { class: "font-bold text-xs mb-1 text-gray-600", "Notes" }
                                            p { class: "text-xs text-gray-700 break-words", "{notes}" }
                                        }
                                    }
                                }

                                // Created at section
                                if let Some(created_at) = &props.stored_raw_password.created_at {
                                    div {
                                        h4 { class: "font-bold text-xs mb-1 text-gray-600", "Created" }
                                        p { class: "text-xs text-gray-700", "{created_at}" }
                                    }
                                }

                                // Show placeholder if no info available
                                if props.stored_raw_password.notes.as_ref().is_none_or(|n| n.is_empty())
                                    && props.stored_raw_password.created_at.is_none() {
                                    p { class: "text-xs text-gray-500 italic", "No additional info" }
                                }
                            }
                        }
                    }
                }
            }

            // Column 5: Edit button (gear icon, yellow warning background)
            td { class: "px-2 py-3",
                button {
                    class: "pwd-row-action-btn pwd-edit-btn",
                    r#type: "button",
                    onclick: {
                        let password = props.stored_raw_password.clone();
                        move |_| props.on_edit.call(password.clone())
                    },
                    // Gear icon
                    EditIcon {}
                }
            }

            // Column 6: Delete button (trash outline)
            td { class: "px-2 py-3",
                button {
                    class: "pwd-row-action-btn pwd-delete-btn",
                    r#type: "button",
                    onclick: move |_| props.on_delete.call(password_id),
                    // Trash icon (outline)
                    DeleteIcon {}
                }
            }
        }
    }
}
