use crate::components::features::dashboard::{
    DeleteStoredPasswordDialogState, StoredPasswordUpsertDialogState,
};
use crate::components::globals::form_field::FormSecret;
use crate::components::globals::password_handler::StrengthAnalyzer;
use crate::components::globals::secret_display::SecretDisplay;
use crate::components::globals::secret_notes_tooltip::SecretNotesTooltip;
use crate::components::globals::svgs::{BurgerIcon, DeleteIcon, EditIcon};
use dioxus::prelude::*;
use pwd_types::{PasswordScore, StoredRawPassword};
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
    pub unlocked_location: Signal<bool>,
    pub unlocked_password: Signal<bool>,
    // /// Callback when user clicks on burger button
    // pub on_click: EventHandler<StoredRawPassword>,
}

#[component]
pub fn StoredRawPasswordRow(props: StoredRawPasswordRowProps) -> Element {
    let mut show_info_tooltip = use_signal(|| false);
    let password_id = props.stored_raw_password.id.unwrap_or(0);
    let store_raw_password_clone = props.stored_raw_password.clone();
    let mut stored_password_dialog_state = use_context::<StoredPasswordUpsertDialogState>();
    let mut deletion_password_dialog_state = use_context::<DeleteStoredPasswordDialogState>();
    // Get strength from score for StrengthAnalyzer
    let strength =
        PasswordScore::get_strength(store_raw_password_clone.score.map(|s| s.value() as i64));
    rsx! {
        tr {
            key: "{password_id}",
            class: "stored-password-row hover:bg-base-200/50 transition-colors",

            // Column 1: Location (visualizzazione sicura con toggle)
            td { class: "pwd-table__cell-content",
                SecretDisplay {
                    secret: FormSecret(props.stored_raw_password.location.clone()),
                    max_width: "".to_string(),
                    // max_width: "120px".to_string(),
                    unlocked: props.unlocked_location,
                }
            }

            // Column 2: Password (visualizzazione sicura con toggle)
            td { class: "pwd-table__cell-content",
                SecretDisplay {
                    secret: FormSecret(store_raw_password_clone.password.clone()),
                    // max_width: "150px".to_string(),
                    max_width: "".to_string(),
                    unlocked: props.unlocked_password,
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

            // Column 4: Burger button (tooltip for notes and created_at) - nascosto su mobile
            td { class: "pwd-table__col-info",
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
                            SecretNotesTooltip {
                                notes: store_raw_password_clone.notes.as_ref().map(|n| n.expose_secret().to_string()),
                                created_at: store_raw_password_clone.created_at.clone(),
                            }
                        }
                    }
                }
            }

            // Column 5: Edit button (gear icon, yellow warning background)
            td { class: "pwd-table__col-actions",
                button {
                    class: "pwd-row-action-btn pwd-edit-btn",
                    r#type: "button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        stored_password_dialog_state
                            .current_stored_raw_password
                            .set(Some(store_raw_password_clone.clone()));
                        stored_password_dialog_state.is_open.set(true);
                    },
                    // Gear icon
                    EditIcon { size: "12".to_string() }
                }
            }

            // Column 6: Delete button (trash outline)
            td { class: "pwd-table__col-actions",
                button {
                    class: "pwd-row-action-btn pwd-delete-btn",
                    r#type: "button",
                    onclick: move |_| {
                        deletion_password_dialog_state.is_open.set(true);
                        deletion_password_dialog_state.password_id.set(Some(password_id));
                        println!("[modal] Deleting password with id: {}", password_id);
                        props.on_delete.call(password_id)
                    },
                    // Trash icon (outline)
                    DeleteIcon { size: "12".to_string() }
                }
            }
        }
    }
}
