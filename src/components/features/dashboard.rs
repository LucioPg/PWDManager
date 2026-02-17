use crate::backend::password_types_helper::{PasswordScore, PasswordStrength, StoredRawPassword};
use crate::components::StoredPasswordUpsertDialog;
use crate::components::globals::{StatCard, StatVariant};
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let username = auth_state.get_username();
    let nav = use_navigator();
    let mut stored_password_dialog_open = use_signal(|| false);
    #[allow(unused_mut)]
    let mut current_stored_raw_password = use_signal(|| Option::<StoredRawPassword>::None);

    let temp = StoredRawPassword {
        id: Some(55),
        user_id: 1,
        location: "Google".to_string(),
        password: "amazingPassword1256@ds!!".to_string().into(),
        notes: Some("test".to_string()),
        score: Some(PasswordScore::new(60)),
        created_at: Some("2025-01-15".to_string()),
    };
    current_stored_raw_password.set(Some(temp));

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
                    label: "God Passwords".to_string(),
                    variant: StatVariant::Success,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Epic Passwords".to_string(),
                    variant: StatVariant::Success,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Strong Passwords".to_string(),
                    variant: StatVariant::Success,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Medium Passwords".to_string(),
                    variant: StatVariant::Warning,
                }
                StatCard {
                    value: "0".to_string(),
                    label: "Weak Passwords".to_string(),
                    variant: StatVariant::Warning,
                }
            }
                        div { class: "card-no-border items-end",
                button { class: "btn btn-success",
                    r#type: "button",
                    onclick: move |_| {stored_password_dialog_open.set(true);},
                    "New Password" }
            }
            div { class: "card card-lg",
                p { class: "text-body text-center", "Your passwords will appear here" }
            }


        }
        StoredPasswordUpsertDialog {
            open: stored_password_dialog_open,
            on_confirm: move |_| {stored_password_dialog_open.set(false);},
            on_cancel: move |_| {stored_password_dialog_open.set(false);},
            stored_raw_password: current_stored_raw_password(),
        }
    }
}
