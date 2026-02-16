#![allow(dead_code)]

use super::base_modal::ModalVariant;
use crate::backend::password_types_helper::{PasswordStrength, StoredPassword, StoredRawPassword};
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, FormField, FormSecret, InputType,
    PasswordHandler,
};
use dioxus::html::completions::CompleteWithBraces::label;
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;

#[cfg(feature = "ide-only")]
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
    let mut current_stored_raw_password: StoredRawPassword;
    let mut is_new = false;
    if let Some(stored_raw_password) = stored_raw_password {
        current_stored_raw_password = stored_raw_password;
    } else {
        is_new = true;
        current_stored_raw_password = StoredRawPassword::new();
    }
    let (srp_id, location, password, notes, strength) =
        current_stored_raw_password.get_form_fields();
    let (title, alert_type) = if is_new {
        ("New Stored Password", "alert-info")
    } else {
        ("Edit Stored Password", "alert-warning")
    };
    let mut location_sig = use_signal(|| location.clone());
    let mut password_sig = use_signal(|| password.expose_secret().clone());
    let mut notes_sig = use_signal(|| notes.clone());
    let mut strength_sig = use_signal(|| strength.clone());
    let mut evaluated_password = use_signal(|| Option::<FormSecret>::None);

    rsx! {
            crate::components::globals::dialogs::BaseModal {
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
                    svg {
                        class: "w-6 h-6",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
                            "stroke-linecap": "round",
                            "stroke-linejoin": "round",
                            "stroke-width": "2"
                        }
                    }
                }

                // Titolo
                h3 { class: "font-bold text-lg mb-2", "{title}" }

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
