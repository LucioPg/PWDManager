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
                div { class: "stat-card",
                    p { class: "stat-value stat-value-primary", "0" }
                    p { class: "stat-label", "Total Passwords" }
                }
                div { class: "stat-card",
                    p { class: "stat-value stat-value-success", "0" }
                    p { class: "stat-label", "Strong Passwords" }
                }
                div { class: "stat-card",
                    p { class: "stat-value stat-value-warning", "0" }
                    p { class: "stat-label", "Weak Passwords" }
                }
            }
            div { class: "card card-lg",
                p { class: "text-body text-center", "Your passwords will appear here" }
            }
        }
    }
}
