use crate::components::globals::secret_notes_tooltip::SecretNotesTooltip;
use crate::components::globals::table::TooltipState;
use crate::components::{Spinner, SpinnerSize, StoredRawPasswordRow};
use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;

use dioxus::document;
use dioxus::prelude::*;

/// Dimensioni della finestra per boundary detection
#[derive(Clone, Copy, Default, serde::Deserialize)]
struct WindowSize {
    width: f64,
    height: f64,
}

#[component]
pub fn StoredRawPasswordsTable(
    /// Valore dei dati (già calcolato dal parent in modo reattivo)
    data: Option<Vec<StoredRawPassword>>,
) -> Element {
    let mut tooltip_state = use_signal(TooltipState::default);
    let mut window_size = use_signal(WindowSize::default);

    // Aggiorna le dimensioni della finestra quando il tooltip si apre
    use_effect(move || {
        if tooltip_state.read().is_open() {
            let mut window_size_clone = window_size.clone();
            spawn(async move {
                let mut eval = document::eval(
                    r#"
                    dioxus.send({ width: window.innerWidth, height: window.innerHeight });
                    "#,
                );
                if let Ok(result) = eval.recv::<WindowSize>().await {
                    window_size_clone.set(result);
                }
            });
        }
    });

    match data.as_ref() {
        Some(stored_raw_passwords) => {
            rsx! {
                // Wrapper con scroll orizzontale per gestire overflow
                div { class: "pwd-table-wrapper relative",
                    table { class: "pwd-table",
                        thead {
                            tr {
                                th { class: "", "Name" }
                                th { class: "pwd-table__col-info", "Info" }
                                th { class: "pwd-table__col-strength", "Strength" }
                                th { class: "pwd-table__col-actions", "Edit" }
                                th { class: "pwd-table__col-actions", "Delete" }
                            }
                        }
                        tbody {
                            for (index , stored_raw_password) in stored_raw_passwords.iter().enumerate() {
                                // Key include id + len(password) + score per forzare re-render
                                // quando qualsiasi campo significativo cambia
                                StoredRawPasswordRow {
                                    key: "{stored_raw_password.id.unwrap_or(0)}-{stored_raw_password.password.expose_secret().len()}-{stored_raw_password.score.map(|s| s.value()).unwrap_or(0)}",
                                    stored_raw_password: stored_raw_password.clone(),
                                    on_edit: move |_| {},
                                    on_delete: move |_| {},
                                    on_show_tooltip: move |(password, x, y)| {
                                        tooltip_state.set(TooltipState::new(password, x, y));
                                    },
                                }
                            }
                        }
                    }
                }

                // Tooltip renderizzato UNA sola volta a livello di tabella
                if tooltip_state.read().is_open() {
                    // Overlay per chiudere il tooltip on click outside
                    div {
                        class: "fixed inset-0 z-[5]",
                        onclick: move |_| tooltip_state.write().close(),
                    }

                    {
                        let state = tooltip_state.read();
                        if let Some(password) = &state.password {
                            let tooltip_width = 280.0;
                            let tooltip_height = 150.0;
                            let margin = 16.0;
                            let win = window_size.read();
                            let left = if state.x + tooltip_width + margin > win.width {
                                (state.x - tooltip_width - margin).max(margin)
                            } else {
                                state.x + margin
                            };
                            let top = if state.y + tooltip_height + margin > win.height {
                                (state.y - tooltip_height - margin).max(margin)
                            } else {
                                state.y + margin
                            };
                            rsx! {
                                div {
                                    class: "pwd-row-tooltip fixed z-10",
                                    style: "left: {left}px; top: {top}px;",
                                    SecretNotesTooltip {
                                        notes: password.notes.as_ref().map(|n| n.expose_secret().to_string()),
                                        created_at: password.created_at.clone(),
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
        _ => {
            rsx! {
                Spinner { size: SpinnerSize::Large, color_class: "text-blue-500" }
            }
        }
    }
}
