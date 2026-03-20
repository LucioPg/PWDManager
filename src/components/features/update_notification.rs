use crate::backend::updater_types::UpdateState;
use crate::components::{Spinner, SpinnerSize};
use dioxus::prelude::*;

#[component]
pub fn UpdateNotification(update_state: Signal<UpdateState>) -> Element {
    let state = update_state.read();

    match &*state {
        UpdateState::Idle | UpdateState::UpToDate => rsx! {},
        UpdateState::Checking => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner { size: SpinnerSize::Medium, color_class: "text-primary" }
                        span { class: "pwd-update-version", "Verifica aggiornamenti..." }
                    }
                }
            }
        },
        UpdateState::Available { version, notes } => {
            let version = version.clone();
            let notes = notes.clone();
            let mut update_state_avail = update_state.clone();
            let mut update_state_dismiss = update_state.clone();
            let manifest = notes.clone();
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        // Icona aggiornamento (freccia circolare)
                        svg {
                            class: "w-10 h-10 text-primary shrink-0",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path { d: "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" }
                            path { d: "M21 3v5h-5" }
                        }
                        div { class: "flex-1 min-w-0",
                            h3 { class: "pwd-update-title", "Aggiornamento disponibile!" }
                            p { class: "pwd-update-version", "Versione {version}" }
                            if !manifest.is_empty() {
                                p { class: "pwd-update-changelog",
                                    dangerous_inner_html: "{manifest}"
                                }
                            }
                        }
                        div { class: "pwd-update-actions",
                            button {
                                class: "btn btn-primary btn-sm",
                                onclick: move |_| {
                                    // TODO(human): il manifest deve essere passato a download_and_install
                                    // per ora salviamo lo stato come placeholder
                                    update_state_avail.set(UpdateState::Downloading { progress: 0 });
                                },
                                "Aggiorna ora"
                            }
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| update_state_dismiss.set(UpdateState::Idle),
                                "Più tardi"
                            }
                        }
                    }
                }
            }
        },
        UpdateState::Downloading { progress } => {
            let progress_val = *progress;
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-title", "Download aggiornamento..." }
                        div { class: "pwd-update-progress-bar",
                            div {
                                class: "pwd-update-progress-fill",
                                style: "width: {progress_val}%",
                            }
                        }
                        p { class: "pwd-update-version mt-2", "{progress_val}%" }
                    }
                }
            }
        },
        UpdateState::Installing => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner { size: SpinnerSize::Medium, color_class: "text-primary" }
                        span { class: "pwd-update-version", "Installazione in corso, l'app si riavviera..." }
                    }
                }
            }
        },
        UpdateState::Error(e) => {
            let error_msg = e.clone();
            let mut update_state_err = update_state.clone();
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-error-text", "Errore aggiornamento: {error_msg}" }
                        div { class: "pwd-update-actions",
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| update_state_err.set(UpdateState::Idle),
                                "Chiudi"
                            }
                        }
                    }
                }
            }
        },
    }
}
