#![allow(dead_code)]
#![allow(unused)]

use dioxus::prelude::*;
use sqlx::SqlitePool;
use tracing::{instrument};
use crate::Route;
#[component]
#[instrument]
pub fn Logout() -> Element {

    let user_id = use_signal(|| String::new()); // Deve essere prelevato l'id dell'utente loggato va usato lo state non il signal
    let pool = use_context::<SqlitePool>(); // questo non serve perché non ci serve il database
    let auth_state = use_context::<crate::auth::AuthState>();
    let mut auth_state_logout = auth_state.clone();
    let nav = use_navigator();
    let on_submit = move |_| {
        auth_state_logout.logout();
    };
    let cancel_logout = move |_| {
        nav.push(Route::Dashboard);
    };

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen px-6 animate-scale-in",
            div { class: "bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center",
                svg {
                    class: "w-16 h-16 text-error-600 mx-auto mb-4",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                    }
                }
                h2 { class: "text-2xl font-bold text-neutral-900 mb-2", "Confirm Logout" }
                p { class: "text-neutral-600 mb-8", "Are you sure you want to logout from your account?" }
                div { class: "flex gap-4",
                    button {
                        class: "flex-1 px-6 py-3 bg-error-600 text-white font-semibold rounded-lg hover:bg-error-700 transition-all",
                        r#type: "submit",
                        onclick: on_submit,
                        "Logout"
                    }
                    button {
                        class: "flex-1 px-6 py-3 border-2 border-neutral-300 text-neutral-700 font-semibold rounded-lg hover:bg-neutral-50 transition-all",
                        r#type: "button",
                        onclick: cancel_logout,
                        "Cancel"
                    }
                }
            }
        }
    }
}