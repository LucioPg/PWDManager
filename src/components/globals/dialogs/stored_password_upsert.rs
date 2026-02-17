#![allow(dead_code)]

use super::base_modal::{BaseModal, ModalVariant};
use crate::backend::password_types_helper::StoredRawPassword;
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, FormField, FormSecret, InputType,
    PasswordHandler,
};
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;

// #[cfg(feature = "ide-only")]
#[component]
pub fn StoredPasswordUpsertDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,
    /// Callback quando l'utente conferma la cancellazione
    on_confirm: EventHandler<()>,
    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
    /// Username da mostrare nel messaggio di warning
    stored_raw_password: Option<StoredRawPassword>,
) -> Element {
    // Cloni per le closure
    let mut open_clone = open.clone();
    let pool = use_context::<SqlitePool>();
    let empty_password = "".to_string();
    let secret_password = SecretString::new(empty_password.into());
    let mut is_new = false;
    #[allow(unused_mut)]
    let mut current_stored_raw_password: StoredRawPassword =
        stored_raw_password.unwrap_or_else(|| {
            is_new = true;
            StoredRawPassword::new()
        });

    #[allow(unused_mut)]
    let mut location_sig = use_signal(|| current_stored_raw_password.location.clone());
    #[allow(unused_mut)]
    let mut password_sig = use_signal(|| current_stored_raw_password.password.clone());
    #[allow(unused_mut)]
    let mut notes_sig = use_signal(|| current_stored_raw_password.notes.clone());
    #[allow(unused_mut)]
    let mut score_sig = use_signal(|| current_stored_raw_password.score.clone());
    let mut evaluated_password = use_signal(|| Option::<FormSecret>::None);

    let (title, alert_type, created_at) = if is_new {
        (
            "New Stored Password".to_string(),
            "alert-info".to_string(),
            "".to_string(),
        )
    } else {
        let l = location_sig.clone();
        (
            format!("Edit Stored Password: \"{}\"", l()),
            "alert-warning".to_string(),
            current_stored_raw_password
                .created_at
                .clone()
                .unwrap_or_default(),
        )
    };

    rsx! {
            BaseModal {
                open: open,
                on_close: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                variant: ModalVariant::Middle,

                // Close button "X" in alto a destra
                button {
                    class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                    onclick: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                    "✕"
                }

                // Icona di warning
                div {
                    class: "alert {alert_type} mb-4 flex items-center justify-center mx-10",
                    p {
                       class: "text-center",
                    {title}}
                    p { class: "text-center", "{created_at}" }
                }

                // Titolo
                // h3 { class: "font-bold text-lg mb-2", "{title}" }

                // Messaggio d

                form { class: "flex flex-col gap-3",
                    FormField {
                            label: "Location".to_string(),
                            input_type: InputType::Text,
                            placeholder: "location or url or whatever...".to_string(),
                            value: location_sig,
                            name: Some("location".to_string()),
                            required: true,
                            alphanumeric_only: false,
                        }
                PasswordHandler {
                        on_password_change: move |pwd| {
                            evaluated_password.set(Some(pwd));
                        },
                        password_required: true,
                        initial_password: if !is_new && !password_sig().expose_secret().is_empty() {
                            Some(FormSecret(password_sig().clone()))
                        } else {
                            None
                        },
                        initial_score: if !is_new { score_sig().clone() } else { None },
                    }
                FormField {
                    label: "Notes".to_string(),
                    input_type: InputType::Textarea,
                    placeholder: "Optional notes".to_string(),
                    value: notes_sig,
                    name: Some("notes".to_string()),
                    required: false,
                    alphanumeric_only: false,
                }
                }


                // Action buttons
                div {
                    class: "modal-action",

                    ActionButton {
                        text: "Annulla".to_string(),
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            on_cancel.call(());
                            open_clone.set(false);
                        },
                    }

                    ActionButton {
                        text: "Save".to_string(),
                        variant: ButtonVariant::Ghost,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        additional_class: "text-success-600 hover:bg-success-50".to_string(),
                        on_click: move |_| {
                            on_confirm.call(());
                            open_clone.set(false);
                        },
                    }
                }
            }
    }
}
