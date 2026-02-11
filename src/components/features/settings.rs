
use crate::components::{ TabContent, Tabs, TabList, TabTrigger, ToastsState, UpsertUser, add_toast, ToastType};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use crate::backend::db_backend::delete_user;



#[component]
pub fn SettingsTabContent() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let user = auth_state.get_user();
    let error = use_signal(|| Option::<String>::None);
    let mut toast_state = use_context::<Signal<ToastsState>>();
    let is_user_deleted = use_signal::<bool>(|| false);
    let mut auth_state_clone = auth_state.clone();
    let on_delete_user = move || {
        let mut is_user_deleted = is_user_deleted.clone();
        let pool = use_context::<SqlitePool>();
        let user = auth_state_clone.get_user();
        let mut error = error.clone();
        match user {
            Some(user) => {
                spawn(async move {
                    match delete_user(&pool, user.id).await {
                        Ok(()) => {

                            is_user_deleted.set(true);
                        },
                        Err(e) => { error.set(Some(e.to_string()));}
                    }
                });
            },
            None => println!("No user to delete"),
        }
    };

    use_effect(move || {

        let mut is_user_deleted = is_user_deleted.clone();
        let error = error.clone();
        let mut auth_state = auth_state.clone();
        let user = auth_state.get_user();
        if is_user_deleted(){
            add_toast(
                format!("User {} deleted successfully!", user.unwrap().username),
               3,
                ToastType::Success,
                toast_state

            );
            auth_state.logout();
            is_user_deleted.set(false);
        }
        if let Some(msg) = error.read().clone() {
            add_toast(msg.to_string(), 4, ToastType::Error, toast_state);
            let mut error_clone = error.clone();
            error_clone.set(None);
        }
        // error.set(None);
    });

    rsx! {
        Tabs{
            default_value: "Account".to_string(),
            horizontal: true,
            TabList {
                TabTrigger { value: "Account".to_string(), index: 0usize, "Account" }
                TabTrigger { value: "Security".to_string(), index: 1usize, "Security" }
                TabTrigger { value: "Notifications".to_string(), index: 2usize, "Notifications" }
            }
            TabContent { index: 0usize, class: "tabs-content border-none shadow-none", value: "Account".to_string(),
                UpsertUser {user_to_edit: user.clone()}
                // div {class:"flex justify-end",
                //     button {class: "btn-danger-lg" ,r#type: "button", onclick: move |_| {on_delete_user();}, "Delete Account"}
                // }
            }
            TabContent { index: 1usize, class: "tabs-content", value: "Security".to_string(),
                div {
                    width: "100%",
                    height: "5rem",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    "Security"
                }
            }
            TabContent { index: 2usize, class: "tabs-content", value: "Notifications".to_string(),
                div {
                    width: "100%",
                    height: "5rem",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    "Notifications"
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
                div { class: "settings-container",
                    SettingsTabContent {}
                }
            }
        }
    }
}
    // let mut auth_state = use_context::<crate::auth::AuthState>();
    // let original_username = auth_state.get_username();
    // let new_username = use_signal(|| original_username.clone());
    // let new_username_string = new_username.read().clone();
    // let selected_image = use_signal(|| None::<Vec<u8>>);
    // let mut toast_state = use_context::<Signal<ToastsState>>();
    // let mut error = use_signal(|| Option::<String>::None);
    // let mut is_loading = use_signal(|| false);
    // let on_submit = move || {
    //     let new_username_string_clone = new_username_string.clone();
    //     println!("Submit {new_username_string_clone}");
    // };
    // let mut new_username_clone = new_username.clone();
    // let original_username_clone = original_username.clone();
    // let mut on_abort = move || {
    //     new_username_clone.set(original_username.clone());
    //     println!("Abort ");
    // };
    //
    // let is_save_disabled_signal = use_signal(move || false);
    // let pick_image = move |_evt: MouseEvent| {
    //     let mut err_signal = error;
    //     let mut img_signal = selected_image;
    //     let mut is_loading_signal = is_loading;
    //     spawn(pick_and_process_avatar(
    //         img_signal,
    //         is_loading_signal,
    //         err_signal,
    //     ));
    // };
    //
    // use_memo(move || {
    //     // forse va rimosso altrimenti impedisce di salvare se non si cambia anche l'username
    //     let mut is_save_disabled_signal_clone = is_save_disabled_signal.clone();
    //     let dis = new_username.clone().to_string() == original_username_clone.clone();
    //     println!("is_save_disabled_signal: {dis}");
    //     is_save_disabled_signal_clone.set(dis);
    // });
    //
    // rsx! {
    //     div { class: "content-container-md animate-fade-in",
    //         h1 { class: "text-h2 mb-8", "Settings" }
    //         form { onsubmit: move |_| {on_submit();},
    //             class: "flex flex-col gap-3 w-full",
    //             div { class: "settings-card",
    //                 div { class: "card-header",
    //                     h2 { class: "card-header-title", "Profile Information" }
    //                 }
    //                 div { class: "p-6",
    //                     form { class: "flex flex-col gap-6",
    //                         AvatarSelector {
    //                             avatar_src: get_user_avatar_with_default(selected_image.read().clone()),
    //                             on_pick: pick_image,
    //                             button_text: "Select Avatar".to_string(),
    //                             size: AvatarSize::XXLarge,
    //                             shadow: true,
    //                             show_border: true,
    //                             loading: is_loading,
    //                         }
    //                         FormField {
    //                             label: "Username".to_string(),
    //                             input_type: InputType::Text,
    //                             placeholder: new_username.clone(),
    //                             value: new_username.clone(),
    //                             disabled: false,
    //                             readonly: false,
    //                         }
    //                     }
    //                 }
    //             }
    //             div { class: "settings-card",
    //                 div { class: "card-header",
    //                     h2 { class: "card-header-title", "Security" }
    //                 }
    //                 div { class: "p-6",
    //                     p { class: "text-body", "Change your password and security settings" }
    //                 }
    //             }
    //             ActionButton {
    //                 text: "Save".to_string(),
    //                 size: ButtonSize::Normal,
    //                 block: false,
    //                 button_type: ButtonType::Submit,
    //                 on_click: move |_| {},
    //                 disabled: is_save_disabled_signal.clone(),
    //
    //             }
    //             ActionButton {
    //                 variant: ButtonVariant::Ghost,
    //                 text: "Abort".to_string(),
    //                 size: ButtonSize::Normal,
    //                 block: false,
    //                 button_type: ButtonType::Button,
    //                 on_click: move |_| {on_abort();},
    //
    //
    //             }
    //             // ActionButtons {
    //             //     primary_text: "Save".to_string(),
    //             //     secondary_text: "Abort".to_string(),
    //             //     primary_on_click: move |_| {}, // Gestito dal form onsubmit
    //             //     secondary_on_click: move |_| {on_abort("test");},
    //             //     variant: ActionButtonsVariant::Auth,
    //             // }
    //         }
    //     }
    // }
//}
