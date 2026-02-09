use crate::components::{FormField, InputType};
use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    let email_value = use_signal(|| String::new());
    rsx! {
        div { class: "content-container-md animate-fade-in",
            h1 { class: "text-h2 mb-8", "Settings" }
            div { class: "settings-card",
                div { class: "card-header",
                    h2 { class: "card-header-title", "Profile Information" }
                }
                div { class: "p-6",
                    form { class: "flex flex-col gap-6",
                        FormField {
                            label: "Username".to_string(),
                            input_type: InputType::Text,
                            placeholder: username.clone(),
                            value: use_signal(|| username.clone()),
                            disabled: true,
                            readonly: true,
                        }
                        FormField {
                            label: "Email".to_string(),
                            input_type: InputType::Email,
                            placeholder: "your.email@example.com".to_string(),
                            value: email_value,
                        }
                    }
                }
            }
            div { class: "settings-card",
                div { class: "card-header",
                    h2 { class: "card-header-title", "Security" }
                }
                div { class: "p-6",
                    p { class: "text-body", "Change your password and security settings" }
                }
            }
        }
    }
}
