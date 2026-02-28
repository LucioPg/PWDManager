use super::base_modal::{BaseModal, ModalVariant};
use crate::backend::password_utils::create_stored_data_pipeline_bulk;
use crate::components::{
    ActionButton, ButtonSize, ButtonType, ButtonVariant, FormField, FormSecret, InputType,
    PasswordHandler, show_toast_error, use_toast,
};
use dioxus::prelude::*;
use pwd_types::{PasswordChangeResult, StoredRawPassword};
use secrecy::{ExposeSecret, SecretString};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct StoredPasswordUpsertDialogState {
    pub is_open: Signal<bool>,
    pub current_stored_raw_password: Signal<Option<StoredRawPassword>>,
}

#[component]
pub fn StoredPasswordUpsertDialog(
    /// Callback quando l'utente conferma
    on_confirm: EventHandler<()>,
    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();
    let mut error = use_signal(|| Option::<String>::None);
    let user_state = use_context::<crate::auth::AuthState>();
    let user = user_state.get_user();
    if user.is_none() {
        error.set(Some("User not logged in".to_string()));
    }
    #[allow(unused_mut)]
    let mut stored_password_dialog_state = use_context::<StoredPasswordUpsertDialogState>();
    let mut open_clone = stored_password_dialog_state.is_open.clone();
    let mut is_new = use_signal(|| false);
    let mut location_sig = use_signal(String::new);
    let mut notes_sig = use_signal(|| None::<String>);
    let mut evaluated_password = use_signal(|| Option::<PasswordChangeResult>::None);

    // use_effect per sincronizzare i campi quando il dialog si apre
    use_effect(move || {
        if (stored_password_dialog_state.is_open)() {
            match (stored_password_dialog_state.current_stored_raw_password)() {
                Some(data) => {
                    location_sig.set(data.location.expose_secret().to_string());
                    notes_sig.set(data.notes.as_ref().map(|n| n.expose_secret().to_string()));
                    is_new.set(false);
                }
                None => {
                    location_sig.set(String::new());
                    notes_sig.set(None);
                    is_new.set(true);
                }
            }
        }
    });
    use_effect(move || {
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error saving user: {}", msg), toast);
            this_error.set(None);
        }
    });
    // Leggi created_at direttamente dal signal per il titolo
    let created_at = (stored_password_dialog_state.current_stored_raw_password)()
        .and_then(|p| p.created_at)
        .unwrap_or_default();

    let (title, alert_type) = if is_new() {
        ("New Stored Password".to_string(), "alert-info".to_string())
    } else {
        (
            format!("Edit Stored Password: \"{}\"", location_sig()),
            "alert-warning".to_string(),
        )
    };

    let on_submit = move |_| {
        if user.is_none() {
            error.set(Some("User not logged in".to_string()));
            return;
        }
        let current = stored_password_dialog_state
            .current_stored_raw_password
            .clone();
        let user_id = user.clone().unwrap().id;
        let stored_password_id = match current() {
            Some(data) => data.id,
            None => None,
        };
        let location = location_sig.clone();
        let note = notes_sig.clone();
        let evaluated_password_reader = evaluated_password.read().clone();
        let (password_to_be_saved, score) = if let Some(ref result) = evaluated_password_reader {
            (result.password.clone(), result.score)
        } else {
            error.set(Some("Password cannot be empty".to_string()));
            return;
        };
        println!(
            "Location: {}\nNote: {}\nPassword: {}",
            location(),
            note().unwrap_or("".to_string()),
            password_to_be_saved.expose_secret().to_string()
        );
        let stored_raw_password = StoredRawPassword {
            id: stored_password_id,
            user_id,
            location: SecretString::new(location().into()),
            notes: note().map(|n| SecretString::new(n.into())),
            password: password_to_be_saved,
            created_at: None,
            score,
        };
        let stored_raw_passwords = vec![stored_raw_password];
        let pool_clone = pool.clone();
        spawn(async move {
            let result =
                create_stored_data_pipeline_bulk(&pool_clone, user_id, stored_raw_passwords).await;
            match result {
                Ok(_) => {
                    // on_confirm.call(());
                    open_clone.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
        // let result = create_stored_data_pipeline_bulk(&pool, user_id, stored_raw_passwords).await;
    };

    rsx! {
        BaseModal {
            open: stored_password_dialog_state.is_open,
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

            // Titolo del dialog
            div {
                class: "alert {alert_type} mb-4 flex items-center justify-center mx-10",
                p {
                   class: "text-center",
                {title}}
                p { class: "text-center", "{created_at}" }
            }

            form { onsubmit: on_submit, class: "flex flex-col gap-3",
                FormField {
                    label: "Location".to_string(),
                    input_type: InputType::Text,
                    placeholder: "location or url or whatever...".to_string(),
                    value: location_sig,
                    name: Some("location".to_string()),
                    required: true,
                    forbid_spaces: false,
                    alphanumeric_only: false,
                }
                PasswordHandler {
                    // Key basata sull'id - forza re-mount quando cambia la password
                    key: stored_raw_password()
                        .as_ref()
                        .and_then(|p| p.id.map(|id| id.to_string()))
                        .unwrap_or_default(),
                    on_password_change: move |pwd| {
                        evaluated_password.set(Some(pwd));
                    },
                    password_required: true,
                    // Legge direttamente dal signal originale
                    initial_password: (stored_password_dialog_state.current_stored_raw_password)().map(|p| FormSecret(p.password)),
                    initial_score: (stored_password_dialog_state.current_stored_raw_password)().and_then(|p| p.score),
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
                    button_type: ButtonType::Submit,
                    size: ButtonSize::Normal,
                    additional_class: "text-success-600 hover:bg-success-50".to_string(),
                    on_click: move |_| {
                        // definito dal on_submit
                    },
                }
            }
            }


        }
    }
}
