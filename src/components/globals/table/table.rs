use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_auth_from_id;
use crate::backend::password_types_helper::StoredRawPassword;
use crate::components::{Spinner, SpinnerSize, StoredRawPasswordRow, show_toast_error, use_toast};
use custom_errors::DBError;
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::ops::Deref;

#[component]
pub fn StoredRawPasswordsTable(data: ReadSignal<Option<Vec<StoredRawPassword>>>) -> Element {
    match &*data.read() {
        Some(stored_raw_passwords) => {
            rsx! {
            table { class: "table-auto w-full",
                thead {
                    tr {
                        th { "Location" }
                        th { "Password" }
                        th { "Info" }
                        th { "Edit" }
                        th { "Delete" }
                    }
                }
                tbody {
                    for stored_raw_password in stored_raw_passwords.iter() {
                            StoredRawPasswordRow {
                                stored_raw_password: stored_raw_password.clone(),
                                on_edit: move |_| {},
                                on_delete: move |_| {},
                            }
                        }
                }
            }
            }
        }
        _ => {
            rsx! {
                Spinner { size: SpinnerSize::Large, }
            }
        }
    }
}
