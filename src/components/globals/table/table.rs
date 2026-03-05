use crate::components::{Spinner, SpinnerSize, StoredRawPasswordRow};
use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;

use dioxus::prelude::*;

#[component]
pub fn StoredRawPasswordsTable(
    /// Valore dei dati (già calcolato dal parent in modo reattivo)
    data: Option<Vec<StoredRawPassword>>,
) -> Element {
    match data.as_ref() {
        Some(stored_raw_passwords) => {
            rsx! {
                // Wrapper con scroll orizzontale per gestire overflow
                div { class: "pwd-table-wrapper relative",
                    table { class: "pwd-table",
                        thead {
                            tr {
                                th { class: "", "Location" }
                                th { class: "", "Password" }
                                th { class: "pwd-table__col-strength", "Strength" }
                                th { class: "pwd-table__col-info", "Info" }
                                th { class: "pwd-table__col-actions", "Edit" }
                                th { class: "pwd-table__col-actions", "Delete" }
                            }
                        }
                        tbody {
                            for (index , stored_raw_password) in stored_raw_passwords.iter().enumerate() {
                                // Key include id + len(password) + score per forzare re-render
                                // quando qualsiasi campo significativo cambia
                                StoredRawPasswordRow {
                                    key: "{stored_raw_password.id.unwrap_or(0)}-{stored_raw_password.password.expose_secret().len()}-{stored_raw_password.score.map(|s| s.value()).unwrap_or(0)}",
                                    stored_raw_password: stored_raw_password.clone(),
                                    on_edit: move |_| {},
                                    on_delete: move |_| {},
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {
            rsx! {
                Spinner { size: SpinnerSize::Large, color_class: "text-blue-500" }
            }
        }
    }
}
