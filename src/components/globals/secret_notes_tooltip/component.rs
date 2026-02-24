use crate::components::globals::svgs::EyeIcon;
use dioxus::prelude::*;

/// Componente per mostrare notes segrete nel tooltip del burger button.
/// Le notes sono nascoste di default e rivelate solo dopo click.
///
/// Nota: notes viene passata come String (già esposta dal chiamante) perché
/// Dioxus non supporta SecretString come prop serializzabile.
#[component]
pub fn SecretNotesTooltip(
    /// Le notes (già esposte dal chiamante tramite expose_secret())
    notes: Option<String>,
    /// Data di creazione
    created_at: Option<String>,
) -> Element {
    let mut notes_visible = use_signal(|| false);

    rsx! {
        div { class: "dropdown-content mockup-code bg-base-200 shadow-lg rounded-lg p-3 min-w-[200px] max-w-[280px]",
            // Notes section
            if let Some(notes) = &notes {
                if !notes.is_empty() {
                    div { class: "mb-3",
                        h4 { class: "font-bold text-xs mb-1 text-gray-600", "Notes" }

                        // Toggle reveal button
                        div { class: "flex items-center gap-2",
                            if notes_visible() {
                                p { class: "text-xs text-gray-700 break-words flex-1",
                                    "{notes}"
                                }
                            } else {
                                p { class: "text-xs text-gray-500 italic flex-1",
                                    "•••••••• (click to reveal)"
                                }
                            }

                            button {
                                class: "btn btn-ghost btn-xs",
                                r#type: "button",
                                onclick: move |_| notes_visible.set(!notes_visible()),
                                aria_label: if notes_visible() { "Nascondi notes" } else { "Mostra notes" },
                                EyeIcon { class: Some("w-4 h-4".to_string()) }
                            }
                        }
                    }
                }
            }

            // Created at section
            if let Some(created_at) = &created_at {
                div {
                    h4 { class: "font-bold text-xs mb-1 text-gray-600", "Created" }
                    p { class: "text-xs text-gray-700", "{created_at}" }
                }
            }

            // Show placeholder if no info available
            if (notes.is_none() || notes.as_ref().is_some_and(|n| n.is_empty())) && created_at.is_none() {
                p { class: "text-xs text-gray-500 italic", "No additional info" }
            }
        }
    }
}
