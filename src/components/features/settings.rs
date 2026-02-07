use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    rsx! {
        div { class: "max-w-2xl mx-auto px-6 py-8 animate-fade-in",
            h1 { class: "text-3xl font-bold text-neutral-900 mb-8", "Settings" }
            div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 mb-6 overflow-hidden",
                div { class: "px-6 py-4 border-b border-neutral-200 bg-neutral-50",
                    h2 { class: "text-lg font-semibold text-neutral-800", "Profile Information" }
                }
                div { class: "px-6 py-6",
                    form { class: "flex flex-col gap-6",
                        div { class: "form-group",
                            label { class: "block text-sm font-medium text-neutral-700 mb-2", "Username" }
                            input {
                                class: "w-full px-4 py-3 border border-neutral-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent transition-all",
                                r#type: "text",
                                value: "{username}",
                                disabled: true
                            }
                        }
                        div { class: "form-group",
                            label { class: "block text-sm font-medium text-neutral-700 mb-2", "Email" }
                            input {
                                class: "w-full px-4 py-3 border border-neutral-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent transition-all",
                                r#type: "email",
                                placeholder: "your.email@example.com"
                            }
                        }
                    }
                }
            }
            div { class: "bg-white rounded-xl shadow-sm border border-neutral-200 mb-6 overflow-hidden",
                div { class: "px-6 py-4 border-b border-neutral-200 bg-neutral-50",
                    h2 { class: "text-lg font-semibold text-neutral-800", "Security" }
                }
                div { class: "px-6 py-6",
                    p { class: "text-neutral-600", "Change your password and security settings" }
                }
            }
        }
    }
}