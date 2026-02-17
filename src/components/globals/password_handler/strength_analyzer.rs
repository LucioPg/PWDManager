use dioxus::prelude::*;

use crate::backend::password_types_helper::{PasswordScore, PasswordStrength};
use crate::components::globals::spinner::{Spinner, SpinnerSize};

#[derive(Props, Clone, PartialEq)]
pub struct StrengthAnalyzerProps {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
    #[props(default)]
    pub is_evaluating: bool,
    pub score: Option<PasswordScore>,
}

#[component]
pub fn StrengthAnalyzer(props: StrengthAnalyzerProps) -> Element {
    let mut show_tooltip = use_signal(|| false);

    // Color mapping
    let (text_class, strength_text) = match props.strength {
        PasswordStrength::NotEvaluated => ("text-gray-500", "Not evaluated".to_string()),
        PasswordStrength::WEAK => ("text-error-600", "Weak".to_string()),
        PasswordStrength::MEDIUM => ("text-warning-600", "Medium".to_string()),
        PasswordStrength::STRONG => ("text-success-600", "Strong".to_string()),
        PasswordStrength::EPIC => ("text-blue-600", "Epic".to_string()),
        PasswordStrength::GOD => ("text-purple-600", "God".to_string()),
    };

    // Calculate cursor position (0-100%)
    let cursor_position = props.score.unwrap_or(PasswordScore::new(0));

    rsx! {
        div { class: "strength-analyzer flex flex-col gap-2",
            // Top row: strength text and tooltip button
            div { class: "flex items-center gap-2",
                // Stato evaluating con spinner
                if props.is_evaluating {
                    Spinner {
                        size: SpinnerSize::Small,
                        color_class: "text-blue-500".to_string(),
                        duration: 0.8,
                    }
                } else {
                    // Strength text
                    span { class: "{text_class} font-medium", "{strength_text}" }

                    // Score display
                    if let Some(score) = props.score {
                        span { class: "text-gray-500 text-sm", "({score}%)" }
                    }

                    // Tooltip button con (?)
                    if !props.reasons.is_empty() {
                        div { class: "relative",
                            button {
                                class: "strength-info-btn",
                                r#type: "button",
                                onclick: move |_| show_tooltip.set(!show_tooltip()),
                                "?"
                            }

                            // Tooltip dropdown
                            if show_tooltip() {
                                // Overlay invisibile per chiudere il tooltip su click outside
                                div {
                                    class: "fixed inset-0 z-[5]",
                                    onclick: move |_| show_tooltip.set(false),
                                }

                                div { class: "strength-reasons-tooltip absolute top-full left-0 mt-2 z-10",
                                    div { class: "dropdown-content mockup-code bg-base-200 shadow-lg rounded-lg p-3 min-w-[200px]",
                                        h4 { class: "font-bold text-sm mb-2", "Why this rating?" }
                                        ul { class: "text-xs space-y-1",
                                            for reason in &props.reasons {
                                                li { class: "flex items-start gap-1",
                                                    span { class: "text-base-content/70", "•" }
                                                    span { "{reason}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Strength bar with gradient and cursor
            if !props.is_evaluating && props.score.is_some() {
                div { class: "strength-bar-container",
                    div { class: "strength-bar",
                        // Cursor indicator with tooltip
                        div {
                            class: "strength-cursor",
                            style: "left: {cursor_position.value()}%",
                            title: "Score: {cursor_position.value()}%",
                        }
                    }
                }
            }
        }
    }
}
