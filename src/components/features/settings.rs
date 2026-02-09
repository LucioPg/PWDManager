use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    rsx! {
        div { class: "content-container-md animate-fade-in",
            h1 { class: "text-h2 mb-8", "Settings" }
            div { class: "settings-card",
                div { class: "card-header",
                    h2 { class: "card-header-title", "Profile Information" }
                }
                div { class: "p-6",
                    form { class: "flex flex-col gap-6",
                        div { class: "form-group",
                            label { class: "form-label", "Username" }
                            input {
                                class: "input-base",
                                r#type: "text",
                                value: "{username}",
                                disabled: true
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Email" }
                            input {
                                class: "input-base",
                                r#type: "email",
                                placeholder: "your.email@example.com"
                            }
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
