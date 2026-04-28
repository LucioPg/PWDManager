// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::backend::updater::download_and_install;
use crate::backend::updater_types::{UpdateManifest, UpdateState};
use crate::components::globals::BreakingChangeDialog;
use crate::components::{Spinner, SpinnerSize};
use dioxus::prelude::*;

#[component]
pub fn UpdateNotification(update_state: Signal<UpdateState>) -> Element {
    let state = update_state.read();
    let update_manifest = use_context::<Signal<Option<UpdateManifest>>>();
    let mut breaking_dialog_open = use_signal(|| true);

    match &*state {
        UpdateState::Idle | UpdateState::UpToDate => rsx! {},
        UpdateState::Checking => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner {
                            size: SpinnerSize::Medium,
                            color_class: "text-primary",
                        }
                        span { class: "pwd-update-version", "Check for updates..." }
                    }
                }
            }
        },
        UpdateState::Available { .. } => {
            let manifest_read = update_manifest.read();
            if let Some(manifest) = manifest_read.as_ref() {
                let manifest_clone = manifest.clone();
                let update_manifest_click = update_manifest;
                let mut update_state_avail = update_state;

                return rsx! {
                    BreakingChangeDialog {
                        open: breaking_dialog_open,
                        manifest: manifest_clone,
                        on_update_now: move |_| {
                            let manifest = update_manifest_click.read().clone();
                            if let Some(manifest) = manifest {
                                let mut update_state = update_state_avail;
                                spawn(async move {
                                    if let Err(e) = download_and_install(&manifest, update_state).await {
                                        update_state.set(UpdateState::Error(e));
                                    }
                                });
                            }
                        },
                        on_dismiss: move |_| {
                            breaking_dialog_open.set(false);
                            update_state_avail.set(UpdateState::Idle);
                        },
                    }
                };
            }

            rsx! {}
        }
        UpdateState::Downloading { progress } => {
            let progress_val = *progress;
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-title", "Download update..." }
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
        }
        UpdateState::Installing => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner {
                            size: SpinnerSize::Medium,
                            color_class: "text-primary",
                        }
                        span { class: "pwd-update-version",
                            "Installation in progress, PWDManager will be restarted..."
                        }
                    }
                }
            }
        },
        UpdateState::Error(e) => {
            let error_msg = e.clone();
            let mut update_state_err = update_state;
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-error-text", "Update Error: {error_msg}" }
                        div { class: "pwd-update-actions",
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| update_state_err.set(UpdateState::Idle),
                                "Close"
                            }
                        }
                    }
                }
            }
        }
    }
}
