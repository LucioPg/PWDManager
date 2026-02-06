#![allow(dead_code)]
#![allow(unused)]

use dioxus::prelude::*;
use sqlx::SqlitePool;
use tracing::{instrument};
#[component]
#[instrument]
pub fn Logout() -> Element {

    let user_id = use_signal(|| String::new()); // Deve essere prelevato l'id dell'utente loggato va usato lo state non il signal
    let pool = use_context::<SqlitePool>(); // questo non serve perché non ci serve il database
    let on_submit = move |_| {
    };

    rsx! {
        form { onsubmit: on_submit,
            // va creato un pulsante per chiedere di procedere con il logout e uno per abortire il processo e tornare alla dashboard

            button { r#type: "submit", "Logout" }
        }
    }
}