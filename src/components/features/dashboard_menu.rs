use dioxus::prelude::*;

/// Dashboard menu component with import/export actions and delete all option.
/// Provides a dropdown menu with nested submenus for file operations.
#[component]
pub fn DashboardMenu() -> Element {
    rsx! {
        div { class: "dropdown dropdown-end",
            // Trigger button with three dots
            div {
                tabindex: "0",
                role: "button",
                class: "btn btn-ghost btn-sm btn-circle",
                "aria-label": "Menu azioni",
                // Three dots icon (centered)
                span { class: "text-lg font-bold tracking-tighter", "..." }
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
                                        // TODO: Implement JSON import
                                        tracing::info!("Import JSON clicked");
                                    },
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        // TODO: Implement CSV import
                                        tracing::info!("Import CSV clicked");
                                    },
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        // TODO: Implement XML import
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
                                        // TODO: Implement JSON export
                                        tracing::info!("Export JSON clicked");
                                    },
                                    "JSON"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        // TODO: Implement CSV export
                                        tracing::info!("Export CSV clicked");
                                    },
                                    "CSV"
                                }
                            }
                            li {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        // TODO: Implement XML export
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
                        onclick: move |_| {
                            // TODO: Implement delete all with confirmation dialog
                            tracing::info!("Delete all clicked");
                        },
                        "Delete All"
                    }
                }
            }
        }
    }
}
