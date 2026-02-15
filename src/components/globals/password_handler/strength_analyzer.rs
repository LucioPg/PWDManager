use dioxus::prelude::*;

use super::PasswordStrength;

#[derive(Props, Clone, PartialEq)]
pub struct StrengthAnalyzerProps {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
    #[props(default)]
    pub is_evaluating: bool,
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
    };

    rsx! {
        div { class: "strength-analyzer flex items-center gap-2",
            // Stato evaluating
            if props.is_evaluating {
                span { class: "text-gray-500 italic", "Evaluating..." }
            } else {
                // Strength text
                span { class: "{text_class} font-medium", "{strength_text}" }

                // Tooltip button con (?)
                if !props.reasons.is_empty() {
                    div { class: "relative",
                        button {
                            class: "strength-info-btn btn btn-circle btn-ghost btn-xs",
                            r#type: "button",
                            onclick: move |_| show_tooltip.set(!show_tooltip()),
                            "?"
                        }

                        // Tooltip dropdown
                        if show_tooltip() {
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
    }
}
