mod backend;
mod auth;
mod components;
use crate::components::{Dashboard, Login, NavBar, Settings, PageNotFound, Logout};
use gui_launcher::launch_desktop;
use dioxus::prelude::*;
// use backend::{list_users, init_db};
use backend::db_backend::init_db;
// use components::{login, navbar, settings, dashboard};


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
            Router::<Route> {}
            }
        }
        Some(Err(e)) => {
            // Mostriamo l'errore all'utente in modo elegante
            rsx! {
                div { class: "error-container",
                    h1 { "Errore critico del Database" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                }
            }
        }
        None => rsx! { "Inizializzazione database in corso..." }
    }
}
fn main() {
    // println!("Creating database and table...");
    // let db_creation: Result<(), Box<dyn std::error::Error>> = create_table();
    // match  db_creation {
    //     Ok(()) => println!("Database created!"),
    //     Err(e) => println!("An error occurred while creating the database: {e}")
    // };
    launch_desktop!(App);

}


#[derive(Routable, PartialEq, Clone)]
enum Route {
    #[layout(NavBar)]
    #[route("/")]
    Login,
    #[route("/login")]
    Dashboard,
    #[route("/logout")]
    Logout,
    #[route("/settings")]
    Settings,

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