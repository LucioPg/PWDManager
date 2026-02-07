use dioxus::prelude::*;
use crate::{components, Route};
use crate::auth::AuthState;

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    // Determiniamo se siamo nella landing page
    // (Assumendo che Route::Landing sia la tua home)
    if auth_state.is_logged_in() {
        rsx! {
            Outlet::<Route> {}
        }
    }
    else {
        rsx! {
            components::LandingPage {}
        }
    }

}