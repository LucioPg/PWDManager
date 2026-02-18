use crate::backend::password_types_helper::{PasswordScore, StoredRawPassword};
use crate::components::globals::password_handler::StrengthAnalyzer;
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

            // Column 2: Password (with ellipsis)
            td { class: "px-4 py-3",
                div {
                    class: "truncate max-w-[200px] font-mono",
                    title: "{props.stored_raw_password.password.expose_secret()}",
                    "{props.stored_raw_password.password.expose_secret()}"
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
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            width: "18",
                            height: "18",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            line { x1: "3", y1: "6", x2: "21", y2: "6" }
                            line { x1: "3", y1: "12", x2: "21", y2: "12" }
                            line { x1: "3", y1: "18", x2: "21", y2: "18" }
                        }
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
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "18",
                        height: "18",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path {
                            d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                        }
                        circle { cx: "12", cy: "12", r: "3" }
                    }
                }
            }

            // Column 6: Delete button (trash outline)
            td { class: "px-2 py-3",
                button {
                    class: "pwd-row-action-btn pwd-delete-btn",
                    r#type: "button",
                    onclick: move |_| props.on_delete.call(password_id),
                    // Trash icon (outline)
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "18",
                        height: "18",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M3 6h18" }
                        path { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" }
                        path { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" }
                        line { x1: "10", y1: "11", x2: "10", y2: "17" }
                        line { x1: "14", y1: "11", x2: "14", y2: "17" }
                    }
                }
            }
        }
    }
}
