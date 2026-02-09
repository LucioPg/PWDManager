#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, RegisterUser,
    RouteWrapper, Settings,
};
use dioxus::prelude::*;
use gui_launcher::launch_desktop;
// use backend::{list_users, init_db};
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
    // Il resource ora conterrà un Result
    let mut db_resource = use_resource(move || async move { init_db().await });
    let resource_value = db_resource.read();
    match &*resource_value {
        Some(Ok(pool)) => {
            // Se il pool è pronto, lo forniamo al resto dell'app
            use_context_provider(|| pool.clone());
            rsx! {
                // Carica il CSS di Tailwind globalmente
                // document::Stylesheet { href: TAILWIND_CSS }
                // document::Stylesheet { href: MAIN_CSS }
                document::Style {"{TAILWIND_CSS}"}
                document::Style {"{MAIN_CSS}"}
                Router::<Route> {}
            }
        }
        Some(Err(e)) => {
            // Mostriamo l'errore all'utente in modo elegante
            rsx! {
                    // document::Link {
                    // rel: "icon",
                    // href: FAVICON
                    // // In Dioxus 0.7, il CLI gestisce il routing di /assets/ correttamente
                    // }
                // document::Stylesheet { href: TAILWIND_CSS }
                document::Style {"{TAILWIND_CSS}"}
                document::Style {"{MAIN_CSS}"}
                div { class: "error-container",
                    h1 { "Errore critico del Database" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                }
            }
        }
        None => rsx! {
                    // document::Link {
                    // rel: "icon",
                    // href: FAVICON
                    // // In Dioxus 0.7, il CLI gestisce il routing di /assets/ correttamente
                    // }
            document::Stylesheet { href: TAILWIND_CSS }
            "Inizializzazione database in corso..."
        },
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
    #[route("/login")]
    Login,
    #[route("/register")]
    RegisterUser,

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
