#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;

use crate::auth::User;
use crate::backend::db_backend::list_users_no_avatar;
use crate::backend::init_blacklist_from_path;
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, RouteWrapper,
    Settings, Spinner, SpinnerSize, Style, ToastContainer, ToastHubState, UpsertUser,
    show_toast_error, show_toast_success,
};
use backend::db_backend::init_db;
use backend::settings_types::Theme;
use dioxus::core::Task;
use dioxus::prelude::*;
use gui_launcher::launch_desktop;

// const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");
//
// // Asset CSS di Tailwind only in dev
// #[cfg(debug_assertions)]
// // static TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
// static TAILWIND_CSS: Asset = asset!("../assets/tailwind.css");
// #[cfg(debug_assertions)]
// // static MAIN_CSS: &str = include_str!("../assets/main.css");
// static MAIN_CSS: Asset = asset!("../assets/main.css");

// Blacklist asset - incluso nel bundle via manganis (senza hash suffix)
#[used]
static BLACKLIST_ASSET: Asset = asset!(
    "assets/blacklist.txt",
    AssetOptions::builder().with_hash_suffix(false)
);

#[component]
fn App() -> Element {
    let auth_state = auth::AuthState::new();
    use_context_provider(move || auth_state);
    let mut app_theme = use_signal(|| Theme::Light);
    use_context_provider(move || app_theme);
    use_context_provider(|| Signal::new(ToastHubState::default()));
    // Il resource ora conterrà un Result
    let mut db_resource = use_resource(move || async move { init_db().await });
    let db_resource_clone_drop = db_resource.clone();
    let resource_value = db_resource.read();
    #[allow(unused_mut)]
    let mut spawn_handle = use_signal(|| Option::<Task>::None);
    #[allow(unused_mut)]
    let mut toast_state = use_context::<Signal<ToastHubState>>();

    // Flag per ricordare se abbiamo già notificato l'inizializzazione del DB
    let mut db_init_notified = use_signal(|| false);
    let mut users_list_printed = use_signal(|| false);
    if cfg!(debug_assertions) {}
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
        // Inizializza la blacklist usando il path risolto dall'asset system
        // Rimuovi lo slash iniziale che manganis aggiunge al path
        let blacklist_path = BLACKLIST_ASSET.to_string();
        let blacklist_path = blacklist_path.trim_start_matches('/');
        if let Err(e) = init_blacklist_from_path(blacklist_path) {
            let error = format!("BLACKLIST Loading is Failed!: {}", e.to_string());

            show_toast_error(error, toast_state);
        }
    });

    use_effect(move || {
        let db_resource_clone = db_resource.clone();

        match &*db_resource_clone.read() {
            Some(Ok(pool)) => {
                // Toast: solo la prima volta che il DB è caricato con successo
                if !db_init_notified() {
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                }
                let mut spawn_handle = spawn_handle.clone();
                if let Some(new_handle) = spawn_handle.take() {
                    new_handle.cancel();
                }
                // Lista utenti: solo la prima volta (se abilitato)
                if cfg!(debug_assertions) {
                    if !users_list_printed() {
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
            }
            Some(Err(_)) => {
                // L'errore può essere temporaneo, non usiamo flag
                show_toast_error("Database Loading failed!".into(), toast_state);
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
                Style {}
                ToastContainer {}
                Router::<Route> {}
            }
        }
        Some(Err(e)) => {
            // Mostriamo l'errore all'utente in modo elegante
            rsx! {
                Style {}
                div { class: "error-container",
                    h1 { "Errore critico del Database" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                }
            }
        }
        None => {
            rsx! {
                Style {}
                div { class: "flex gap-4 justify-center items-center h-screen",
                    Spinner {
                        size: SpinnerSize::XXXXLarge,
                        color_class: "text-blue-500",
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
    #[route("/login")]
    Login,
    #[route("/register")]
    UpsertUser { user_to_edit: Option<User> },
    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}
