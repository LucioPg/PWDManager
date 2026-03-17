use crate::components::{
    StoredPasswordSettings, TabContent, TabList, TabTrigger, Tabs, UpsertUser,
};
use dioxus::prelude::*;

#[component]
pub fn SettingsTabContent() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let user = auth_state.get_user();

    rsx! {
        Tabs { default_value: "Account".to_string(), horizontal: true,
            TabList {
                TabTrigger { value: "Account".to_string(), index: 0usize, "Account" }
                TabTrigger { value: "Security".to_string(), index: 1usize, "Security" }
                TabTrigger { value: "Notifications".to_string(), index: 2usize, "Notifications" }
            }
            TabContent {
                index: 0usize,
                class: "tabs-content border-none shadow-none",
                value: "Account".to_string(),
                UpsertUser { user_to_edit: user.clone() }
                        // div {class:"flex justify-end",
            //     button {class: "btn-danger-lg" ,r#type: "button", onclick: move |_| {on_delete_user();}, "Delete Account"}
            // }
            }
            TabContent {
                index: 1usize,
                class: "tabs-content",
                value: "Security".to_string(),
                // div {
                //     width: "100%",
                //     height: "5rem",
                //     display: "flex",
                //     align_items: "center",
                //     justify_content: "center",
                //     "Security"
                // }
                StoredPasswordSettings { user_to_edit: user.clone() }
            }
            TabContent {
                index: 2usize,
                class: "tabs-content",
                value: "Aspect".to_string(),
                div {
                    width: "100%",
                    height: "5rem",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    "Aspect"
                }
            }
        }
    }
}

#[component]
pub fn Settings() -> Element {
    rsx! {
        div { class: "settings-page-body",
            div { class: "settings-page-header",
                div { class: "settings-page-header-content",
                    h1 { class: "text-h2 mt-4 mb-3 text-center", "Settings" }
                    p { class: "text-body", "Manage your account settings and preferences." }
                }
            }
            div { class: "",
                div { class: "settings-container", SettingsTabContent {} }
            }
        }
    }
}
