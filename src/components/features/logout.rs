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
        nav.push(Route::LandingPage);
    };
    let cancel_logout = move |_| {
        nav.push(Route::Dashboard);
    };

    rsx! {
        div { class: "page-centered animate-scale-in",
            div { class: "card card-md form-container text-center max-h-[90vh] overflow-y-auto",
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
                h2 { class: "text-h2", "Confirm Logout" }
                p { class: "text-body mb-8", "Are you sure you want to logout from your account?" }
                div { class: "flex gap-4",
                    button {
                        class: "btn-danger btn-block",
                        r#type: "submit",
                        onclick: on_submit,
                        "Logout"
                    }
                    button {
                        class: "btn-secondary btn-block",
                        r#type: "button",
                        onclick: cancel_logout,
                        "Cancel"
                    }
                }
            }
        }
    }
}