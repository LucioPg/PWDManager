#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;

use crate::auth::User;
use crate::backend::db_backend::list_users_no_avatar;
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, RouteWrapper,
    Settings, ToastContainer, ToastType, ToastsState, UpsertUser, add_toast,
};
use backend::db_backend::init_db;
use dioxus::core::Task;
use dioxus::prelude::*;
use dioxus_components::{Spinner, SpinnerSize};
use gui_launcher::launch_desktop;

// Asset CSS di Tailwind
static TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
static MAIN_CSS: &str = include_str!("../assets/main.css");
const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");
const SHOW_USERS_LIST: bool = true;
#[component]
fn App() -> Element {
    let auth_state = auth::AuthState::new();
    use_context_provider(move || auth_state);
    use_context_provider(|| Signal::new(ToastsState::default()));
    // Il resource ora conterrà un Result
    let mut db_resource = use_resource(move || async move { init_db().await });
    let db_resource_clone_drop = db_resource.clone();
    let resource_value = db_resource.read();
    #[allow(unused_mut)]
    let mut spawn_handle = use_signal(|| Option::<Task>::None);
    #[allow(unused_mut)]
    let mut toast_state = use_context::<Signal<ToastsState>>();

    // Flag per ricordare se abbiamo già notificato l'inizializzazione del DB
    let mut db_init_notified = use_signal(|| false);
    let mut users_list_printed = use_signal(|| false);

    // Cleanup del pool quando il componente viene smontato o l'app si chiude
    use_drop(move || {
        let db_resource_clone = db_resource_clone_drop.clone();
        match &*db_resource_clone.read() {
            Some(Ok(pool)) => {
                println!("Cleanup: chiudo connessioni DB prima dell'uscita");
                let pool_clone = pool.clone();
                spawn(async move {
                    let _ = pool_clone.close().await;
                });
            }
            _ => println!("Cleanup: pool non presente"), // Chiude tutte le connessioni al database
        }
    });

    use_effect(move || {
        let db_resource_clone = db_resource.clone();

        match &*db_resource_clone.read() {
            Some(Ok(pool)) => {
                // Toast: solo la prima volta che il DB è caricato con successo
                if !db_init_notified() {
                    add_toast(
                        "Caricamento database riuscito!".into(),
                        6,
                        ToastType::Success,
                        toast_state,
                    );
                    db_init_notified.set(true);
                }
                let mut spawn_handle = spawn_handle.clone();
                if let Some(new_handle) = spawn_handle.take() {
                    new_handle.cancel();
                }
                // Lista utenti: solo la prima volta (se abilitato)
                if SHOW_USERS_LIST && !users_list_printed() {
                    let pool_clone = pool.clone();
                    let handle = spawn(async move {
                        match list_users_no_avatar(&pool_clone).await {
                            Ok(users) => {
                                println!("=== LISTA UTENTI ===");
                                println!("ID  --  Username  --  Creation Date");
                                for (id, username, password) in users {
                                    println!("{}\t{}\t{}", id, username, password);
                                }
                                println!("===================");
                                users_list_printed.set(true);
                            }
                            Err(e) => {
                                println!("Errore nel recupero utenti: {:?}", e);
                            }
                        }
                    });
                    spawn_handle.set(Some(handle));
                }
            }
            Some(Err(_)) => {
                // L'errore può essere temporaneo, non usiamo flag
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
    Login {
        new_user: Option<bool>,
        user_updated: Option<bool>,
    },
    #[route("/register")]
    UpsertUser { user_to_edit: Option<User> },

    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}
