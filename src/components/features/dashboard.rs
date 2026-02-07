use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    rsx! {
        div { class: "max-w-7xl mx-auto px-6 py-8 animate-fade-in",
            div { class: "mb-8",
                h1 { class: "text-3xl font-bold text-neutral-900", "Welcome, {username}!" }
                p { class: "text-neutral-600 mt-2", "Manage your passwords and secure your digital life" }
            }
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6 mb-8",
                div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 p-6 hover:shadow-md transition-shadow",
                    p { class: "text-3xl font-bold text-primary-600", "0" }
                    p { class: "text-neutral-600 mt-1", "Total Passwords" }
                }
                div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 p-6 hover:shadow-md transition-shadow",
                    p { class: "text-3xl font-bold text-success-600", "0" }
                    p { class: "text-neutral-600 mt-1", "Strong Passwords" }
                }
                div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 p-6 hover:shadow-md transition-shadow",
                    p { class: "text-3xl font-bold text-warning-600", "0" }
                    p { class: "text-neutral-600 mt-1", "Weak Passwords" }
                }
            }
            div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 p-8",
                p { class: "text-neutral-600 text-center", "Your passwords will appear here" }
            }
        }
    }
}