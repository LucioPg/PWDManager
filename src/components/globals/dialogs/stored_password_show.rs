use super::base_modal::{BaseModal, ModalVariant};
use crate::components::globals::secret_display::copy_to_clipboard;
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, StoredPasswordShowDialogState,
};
use dioxus::prelude::*;
use pwd_dioxus::InputType;
use pwd_dioxus::form::FormField;
use pwd_dioxus::icons::ClipboardIcon;
use secrecy::ExposeSecret;

#[component]
pub fn StoredPasswordShowDialog(
    /// Callback quando l'utente conferma
    on_confirm: EventHandler<()>,
    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    #[allow(unused_mut)]
    let mut stored_password_dialog_state = use_context::<StoredPasswordShowDialogState>();
    let mut open_clone = stored_password_dialog_state.is_open;
    let mut url_sig = use_signal(String::new);
    let mut notes_sig = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut name_sig = use_signal(String::new);
    let mut username_sig = use_signal(String::new);
    // use_effect per sincronizzare i campi quando il dialog si apre
    use_effect(move || {
        if (stored_password_dialog_state.is_open)() {
            match (stored_password_dialog_state.current_stored_raw_password)() {
                Some(data) => {
                    name_sig.set(data.name.clone());
                    username_sig.set(data.username.expose_secret().to_string());
                    password.set(data.password.expose_secret().to_string());
                    url_sig.set(data.url.expose_secret().to_string());
                    let notes_data = if let Some(notes) = &data.notes {
                        notes.expose_secret().to_string()
                    } else {
                        String::new()
                    };
                    notes_sig.set(notes_data);
                }
                None => {
                    name_sig.set(String::new());
                    username_sig.set(String::new());
                    password.set(String::new());
                    url_sig.set(String::new());
                    notes_sig.set(String::new());
                }
            }
        }
    });

    // Leggi created_at direttamente dal signal per il titolo
    let created_at = (stored_password_dialog_state.current_stored_raw_password)()
        .and_then(|p| p.created_at)
        .unwrap_or_default();

    rsx! {
        BaseModal {
            open: stored_password_dialog_state.is_open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Close button "X" in alto a destra
            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            // Titolo del dialog
            div { class: "alert alert-info mb-4 flex items-center justify-center mx-10",
                p { class: "text-center", "Stored Password Details" }
                p { class: "text-center", "{created_at}" }
            }

            div { class: "flex flex-col gap-3",
                label { class: "label",
                    strong { "Name: " }
                }
                p { class: "label-text-alt", "{name_sig()}" }

                label { class: "label",
                    strong { "Username: " }
                }
                div { class: "flex flex-row gap-2",
                    button {
                        class: "pwd-display-action-btn",
                        r#type: "button",
                        tabindex: "-1",
                        aria_label: "Copy into clipboard",
                        disabled: username_sig().is_empty(),
                        onclick: move |_| {
                            copy_to_clipboard(username_sig().clone());
                        },
                        ClipboardIcon { class: Some("text-current".to_string()) }
                    }
                    p { class: "label-text-alt", "{username_sig()}" }
                }
                label { class: "label",
                    strong { "url: " }
                }
                div { class: "flex flex-row gap-2",
                    button {
                        class: "pwd-display-action-btn",
                        r#type: "button",
                        tabindex: "-1",
                        aria_label: "Copy into clipboard",
                        disabled: url_sig().is_empty(),
                        onclick: move |_| {
                            copy_to_clipboard(url_sig().clone());
                        },
                        ClipboardIcon { class: Some("text-current".to_string()) }
                    }
                    p { class: "label-text-alt", "{url_sig()}" }
                }
                label { class: "label",
                    strong { "Password: " }
                }
                div { class: "flex flex-row gap-2",
                    button {
                        class: "pwd-display-action-btn",
                        r#type: "button",
                        tabindex: "-1",
                        aria_label: "Copy into clipboard",
                        disabled: password().is_empty(),
                        onclick: move |_| {
                            copy_to_clipboard(password().clone());
                        },
                        ClipboardIcon { class: Some("text-current".to_string()) }
                    }
                    p { class: "label-text-alt", "{password()}" }
                }

                label { class: "label",
                    strong { "Notes: " }
                }
                FormField {
                    label: String::new(),
                    input_type: InputType::Textarea,
                    placeholder: String::new(),
                    value: notes_sig,
                    name: Some("notes".to_string()),
                    required: false,
                    readonly: true,
                    alphanumeric_only: false,
                    class: "w-full",
                }
            }
            // Action buttons
            div { class: "modal-action",

                ActionButton {
                    text: "Ok".to_string(),
                    variant: ButtonVariant::Ghost,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    additional_class: "text-success hover:bg-success/10".to_string(),
                    on_click: move |_| { open_clone.set(false) },
                }
            }
        }
    }
}
