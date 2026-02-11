use crate::auth::AuthState;
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    let nav = use_navigator();
    // Determiniamo se siamo nella landing page
    // (Assumendo che Route::Landing sia la tua home)
    if !auth_state.is_logged_in() {
        nav.push(Route::LandingPage);
    }
    rsx! {
        Outlet::<Route> {}
    }
}
