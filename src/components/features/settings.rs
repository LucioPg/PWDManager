// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::components::features::diceware_settings::DicewareSettings;
use crate::components::features::general_settings::GeneralSettings;
use crate::components::features::recovery_key_settings::RecoveryKeySettings;
use crate::components::{
    StoredPasswordSettings, TabContent, TabList, TabTrigger, Tabs, UpsertUser,
};
use dioxus::prelude::*;
use pwd_dioxus::{show_toast_error, use_toast};

#[component]
pub fn SettingsTabContent() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let toast = use_toast();
    let user = auth_state.get_user();
    let error = use_signal(|| None::<String>);

    use_effect(move || {
        let mut this_error = error;
        let toast = toast;
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching password settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    rsx! {
        Tabs { default_value: "Account".to_string(), horizontal: true,
            TabList {
                TabTrigger { value: "Account".to_string(), index: 0usize, "Account" }
                TabTrigger { value: "Security".to_string(), index: 1usize, "Security" }
                TabTrigger { value: "General".to_string(), index: 2usize, "General" }
            }
            TabContent {
                index: 0usize,
                class: "pwd-tabs-content border-none shadow-none",
                value: "Account".to_string(),
                UpsertUser { user_to_edit: user.clone() }
                        // div {class:"flex justify-end",
            //     button {class: "btn-danger-lg" ,r#type: "button", onclick: move |_| {on_delete_user();}, "Delete Account"}
            // }
            }
            TabContent {
                index: 1usize,
                class: "pwd-tabs-content",
                value: "Security".to_string(),
                Tabs {
                    class: "pwd-tabs-inner".to_string(),
                    default_value: "Random Password".to_string(),
                    horizontal: true,
                    TabList {
                        TabTrigger {
                            value: "Random Password".to_string(),
                            index: 0usize,
                            "Random Password"
                        }
                        TabTrigger { value: "Diceware".to_string(), index: 1usize, "Diceware" }
                        TabTrigger { value: "Recovery Key".to_string(), index: 2usize, "Recovery Key" }
                    }
                    TabContent {
                        index: 0usize,
                        class: "pwd-tabs-content",
                        value: "Random Password".to_string(),
                        StoredPasswordSettings { user_to_edit: user.clone() }
                    }
                    TabContent {
                        index: 1usize,
                        class: "pwd-tabs-content",
                        value: "Diceware".to_string(),
                        DicewareSettings {}
                    }
                    TabContent {
                        index: 2usize,
                        class: "pwd-tabs-content",
                        value: "Recovery Key".to_string(),
                        RecoveryKeySettings {}
                    }
                }
            }
            TabContent {
                index: 2usize,
                class: "pwd-tabs-content",
                value: "General".to_string(),
                GeneralSettings {}
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
            div { class: "w-full",
                div { class: "settings-container", SettingsTabContent {} }
            }
        }
    }
}
