use crate::backend::db_backend::delete_all_user_stored_passwords;
use crate::components::AllStoredPasswordDeletionDialog;
use dioxus::prelude::*;
use pwd_dioxus::{show_toast_error, use_toast};
use sqlx::SqlitePool;

/// Dashboard menu component with import/export actions and delete all option.
/// Provides a dropdown menu with nested submenus for file operations.
#[component]
pub fn DashboardMenu(on_need_restart: Signal<bool>) -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let pool = use_context::<SqlitePool>();
    let user = auth_state.get_user();
    let toast = use_toast();
    let mut error = use_signal(|| Option::<String>::None);
    let mut warning_open = use_signal(|| false);
    let on_delete_all = move |_| {
        let user_clone = user.clone();
        if let Some(user) = user_clone {
            let pool_clone = pool.clone();
            spawn(async move {
                match delete_all_user_stored_passwords(&pool_clone, user.id).await {
                    Ok(()) => {
                        tracing::info!("All user passwords deleted successfully");
                        println!("All user passwords deleted successfully");
                        on_need_restart.set(true);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                    }
                }
            });
        };
    };

    let on_warning_open = move |_| {
        let mut warning_open = warning_open.clone();
        warning_open.set(true);
    };

    use_effect(move || {
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error saving user: {}", msg), toast);
            this_error.set(None);
        }
    });

    rsx! {
        div { class: "dropdown dropdown-end",
            // Trigger button with three dots
            div {
                tabindex: "0",
                role: "button",
                class: "btn btn-ghost btn-sm btn-circle flex items-center justify-center",
                "aria-label": "Menu azioni",
                // Three dots icon (centered)
                span { class: "text-lg font-bold leading-none", "..." }
            }

            // Dropdown content
            ul {
                tabindex: "0",
                class: "dropdown-content menu bg-base-100 rounded-box z-[100] w-52 p-2 shadow-lg border border-base-300",

                // Import submenu
                li {
                    details {
                        summary { class: "cursor-pointer", "Import" }
                        ul {
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import JSON clicked");
                                    },
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import CSV clicked");
                                    },
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Import XML clicked");
                                    },
                                    "XML"
                                }
                            }
                        }
                    }
                }

                // Export submenu
                li {
                    details {
                        summary { class: "cursor-pointer", "Export" }
                        ul {
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Export JSON clicked");
                                    },
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Export CSV clicked");
                                    },
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        tracing::info!("Export XML clicked");
                                    },
                                    "XML"
                                }
                            }
                        }
                    }
                }

                // Separator
                li { class: "my-1",
                    hr { class: "border-base-300" }
                }

                // Delete all (dangerous action)
                li {
                    button {
                        r#type: "button",
                        class: "text-error hover:bg-error hover:text-error-content",
                        onclick: on_warning_open,
                        "Delete All"
                    }
                }
            }
        }
        AllStoredPasswordDeletionDialog {
            open: warning_open,
            on_confirm: on_delete_all,
            on_cancel: move |_| {
                let mut warning_open = warning_open.clone();
                warning_open.set(false);
            },
        }
    }
}
