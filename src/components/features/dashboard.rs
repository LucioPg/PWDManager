use crate::components::globals::{StatCard, StatVariant};
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    rsx! {
        div { class: "content-container animate-fade-in",
            div { class: "mb-8",
                h1 { class: "text-h2", "Welcome, {username}!" }
                p { class: "text-body mt-2", "Manage your passwords and secure your digital life" }
            }
            div { class: "stats-grid",
                StatCard {
                    value: "0".to_string(),
                    label: "Total Passwords".to_string(),
                    variant: StatVariant::Primary,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Strong Passwords".to_string(),
                    variant: StatVariant::Success,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Weak Passwords".to_string(),
                    variant: StatVariant::Warning,
                }
            }
            div { class: "card card-lg",
                p { class: "text-body text-center", "Your passwords will appear here" }
            }
        }
    }
}
