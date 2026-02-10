#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, UpsertUser,
    RouteWrapper, Settings, ToastContainer, ToastType, ToastsState, add_toast,
};
use dioxus::prelude::*;
use dioxus_components::{Spinner, SpinnerSize};
use gui_launcher::launch_desktop;
// use backend::{list_users, init_db};
use crate::auth::User;
use backend::db_backend::init_db;
// use components::{login, navbar, settings, dashboard};

// Asset CSS di Tailwind
// static TAILWIND_CSS: Asset = asset!("../assets/tailwind.css");
static TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
// static MAIN_CSS: Asset = asset!("../assets/main.css");
static MAIN_CSS: &str = include_str!("../assets/main.css");
// const FAVICON: Asset = asset!("../assets/favicon.ico", AssetOptions::builder().with_hash_suffix(false));

const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

#[component]
fn App() -> Element {
    let auth_state = auth::AuthState::new();
    use_context_provider(move || auth_state);
    use_context_provider(|| Signal::new(ToastsState::default()));
    // Il resource ora conterrà un Result
    let mut db_resource = use_resource(move || async move { init_db().await });

    let resource_value = db_resource.read();
    let mut toast_state = use_context::<Signal<ToastsState>>();
    use_effect(move || {
        let db_resource_clone = db_resource.clone();

        match &*db_resource_clone.read() {
            Some(Ok(pool)) => {
                add_toast(
                    "Caricamento database riuscito!".into(),
                    6,
                    ToastType::Success,
                    toast_state,
                );
            }
            Some(Err(e)) => {
                // Mostriamo l'errore all'utente in modo elegante
                add_toast(
                    "Caricamento database Fallito!".into(),
                    6,
                    ToastType::Error,
                    toast_state,
                );
            }
            None => {}
        }
    });

    match &*resource_value {
        Some(Ok(pool)) => {
            // Se il pool è pronto, lo forniamo al resto dell'app
            use_context_provider(|| pool.clone());
            rsx! {
                // Carica il CSS di Tailwind globalmente
                document::Style {"{TAILWIND_CSS}"}
                document::Style {"{MAIN_CSS}"}
                ToastContainer {}
                Router::<Route> {}
            }
        }
        Some(Err(e)) => {
            // Mostriamo l'errore all'utente in modo elegante
            rsx! {
                document::Style {"{TAILWIND_CSS}"}
                document::Style {"{MAIN_CSS}"}
                div { class: "error-container",
                    h1 { "Errore critico del Database" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                }
            }
        }
        None => {
            rsx! {
                document::Style {"{TAILWIND_CSS}"}
                document::Style {"{MAIN_CSS}"}
                div {
                    class: "flex gap-4 justify-center items-center h-screen",
                    Spinner {
                    size: SpinnerSize::Small,
                    color: "text-success"
                }
                }

            }
        }
    }
}
fn main() {
    // Nota: il logging viene inizializzato automaticamente nel launcher
    launch_desktop!(App);
}

#[derive(Routable, PartialEq, Clone)]
enum Route {
    #[layout(RouteWrapper)]
    #[layout(NavBar)]
    #[route("/")]
    LandingPage,
    #[layout(AuthWrapper)]
    #[route("/dashboard")]
    Dashboard,

    #[route("/logout")]
    Logout,
    #[route("/settings")]
    Settings,
    #[end_layout(AuthWrapper)]
    #[route("/login?:new_user")]
    Login { new_user: Option<bool> },
    #[route("/register")]
    UpsertUser { user_to_edit: Option<User> },

    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}

// #[derive(Routable, Clone, PartialEq, Debug)]
// #[rustfmt::skip]
// enum Route {
//     #[route("/")]
//     Login {},
//     #[route("/dashboard")]
//     Dashboard {},
// }
