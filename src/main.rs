#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;

use crate::auth::User;
use crate::backend::db_backend::list_users_no_avatar;
use crate::backend::db_backend::InitResult;
use crate::backend::init_blacklist_from_path;
use crate::backend::settings_types::AutoUpdate;
use crate::backend::updater_types::{UpdateManifest, UpdateState};
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, RouteWrapper,
    Settings, Spinner, SpinnerSize, Style, ToastContainer, ToastHubState, UpdateNotification,
    UpsertUser, show_toast_error, show_toast_success,
};
use crate::components::{
    DatabaseResetDialog, RecoveryKeyInputDialog, RecoveryKeySetupDialog,
};
use backend::db_backend::init_db;
use backend::settings_types::Theme;
use dioxus::core::Task;
use dioxus::prelude::*;
use gui_launcher::launch_desktop;
use secrecy::ExposeSecret;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use std::str::FromStr;
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
    let mut auto_update = use_signal(|| AutoUpdate::default());
    use_context_provider(move || app_theme);
    use_context_provider(|| auto_update);
    let mut update_state = use_signal(|| UpdateState::Idle);
    use_context_provider(|| update_state);
    let mut update_manifest = use_signal(|| None::<UpdateManifest>);
    use_context_provider(|| update_manifest);
    use_context_provider(|| Signal::new(ToastHubState::default()));

    let mut db_resource = use_resource(move || async move { init_db().await });
    let db_resource_clone_drop = db_resource.clone();
    #[allow(unused_mut)]
    let mut spawn_handle = use_signal(|| Option::<Task>::None);
    let mut toast_state = use_context::<Signal<ToastHubState>>();
    let mut db_init_notified = use_signal(|| false);
    let mut users_list_printed = use_signal(|| false);

    // Recovery dialog state
    let mut show_recovery_dialog = use_signal(|| false);
    let mut recovery_error = use_signal(|| false);
    let mut show_reset_dialog = use_signal(|| false);
    let mut show_setup_dialog = use_signal(|| false);
    let mut setup_passphrase = use_signal(|| String::new());

    // Cleanup del pool quando il componente viene smontato o l'app si chiude
    use_drop(move || {
        let db_resource_clone = db_resource_clone_drop.clone();
        match &*db_resource_clone.read() {
            Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
                println!("Cleanup: chiudo connessioni DB prima dell'uscita");
                let pool_clone = pool.clone();
                spawn(async move {
                    let _ = pool_clone.close().await;
                });
            }
            _ => println!("Cleanup: pool non presente"),
        }
    });

    use_effect(move || {
        let blacklist_path = BLACKLIST_ASSET.to_string();
        let blacklist_path = blacklist_path.trim_start_matches('/');
        if let Err(e) = init_blacklist_from_path(blacklist_path) {
            let error = format!("BLACKLIST Loading is Failed!: {}", e.to_string());
            show_toast_error(error, toast_state);
        }
    });

    // Effect: handle DB resource changes
    use_effect(move || {
        let db_resource_clone = db_resource.clone();
        let resource = db_resource_clone.read();

        match &*resource {
            Some(Ok(InitResult::Ready(_))) => {
                if !db_init_notified() {
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                }
            }
            Some(Ok(InitResult::FirstSetup { .. })) => {
                if !db_init_notified() {
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                }
            }
            Some(Err(custom_errors::DBError::DBKeyMissingWithDb)) => {
                show_recovery_dialog.set(true);
                show_toast_error("Recovery key required".into(), toast_state);
            }
            Some(Err(_)) => {
                show_toast_error("Database Loading failed!".into(), toast_state);
            }
            None => {}
        }
    });

    // Effect: debug user list
    use_effect(move || {
        let db_resource_clone = db_resource.clone();

        match &*db_resource_clone.read() {
            Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
                let mut spawn_handle = spawn_handle.clone();
                if let Some(new_handle) = spawn_handle.take() {
                    new_handle.cancel();
                }
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
            _ => {}
        }
    });

    // Effect: detect FirstSetup and show dialog
    use_effect(move || {
        let resource = db_resource.read();
        if let Some(Ok(InitResult::FirstSetup { recovery_phrase, .. })) = &*resource {
            setup_passphrase.set(recovery_phrase.expose_secret().to_string());
            show_setup_dialog.set(true);
        }
    });

    match &*db_resource.read() {
        Some(Ok(InitResult::Ready(pool))) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
        Some(Ok(InitResult::FirstSetup { pool, .. })) => {
            use_context_provider(|| pool.clone());
            render_app_with_setup(pool, show_setup_dialog, setup_passphrase, update_state)
        }
        Some(Err(custom_errors::DBError::DBKeyMissingWithDb)) => {
            render_recovery_ui(
                db_resource,
                show_recovery_dialog,
                recovery_error,
                show_reset_dialog,
                db_init_notified,
            )
        }
        Some(Err(custom_errors::DBError::DBSaltFileError(msg))) => {
            render_salt_error_ui(db_resource, show_reset_dialog, msg.clone(), db_init_notified)
        }
        Some(Err(e)) => {
            rsx! {
                Style {}
                div { class: "error-container",
                    h1 { "Critical Database Error" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Retry" }
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

fn render_app_with_setup(
    pool: &sqlx::SqlitePool,
    show_setup_dialog: Signal<bool>,
    setup_passphrase: Signal<String>,
    update_state: Signal<UpdateState>,
) -> Element {
    rsx! {
        Style {}
        ToastContainer {}
        UpdateNotification { update_state }
        Router::<Route> {}

        RecoveryKeySetupDialog {
            open: show_setup_dialog,
            passphrase: setup_passphrase.read().clone(),
            on_confirm: move |_| {},
        }
    }
}

fn render_recovery_ui(
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_recovery_dialog: Signal<bool>,
    mut recovery_error: Signal<bool>,
    mut show_reset_dialog: Signal<bool>,
    mut db_init_notified: Signal<bool>,
) -> Element {
    let handle_recover = move |passphrase: String| {
        let passphrase = passphrase.clone();
        spawn(async move {
            let db_path = std::env::current_dir()
                .unwrap_or_default()
                .join("database.db")
                .to_str()
                .unwrap()
                .to_string();

            let derive_result = tokio::task::spawn_blocking({
                let p = passphrase.clone();
                let path = db_path.clone();
                move || {
                    let salt = crate::backend::db_key::read_salt(&path)?;
                    crate::backend::db_key::derive_key(&p, &salt)
                }
            })
            .await;

            let key = match derive_result {
                Ok(Ok(key)) => key,
                _ => {
                    recovery_error.set(true);
                    return;
                }
            };

            // Try to open DB with derived key
            let pragma = format!("\"x'{}'\"", key);
            let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))
                .unwrap()
                .pragma("key", pragma)
                .pragma("foreign_keys", "ON")
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);

            match sqlx::SqlitePool::connect_with(opts).await {
                Ok(_pool) => {
                    // Store key in keyring
                    let _ = crate::backend::db_key::store_db_key(
                        crate::backend::db_key::SERVICE_NAME,
                        crate::backend::db_key::KEY_USERNAME,
                        &key,
                    );
                    recovery_error.set(false);
                    show_recovery_dialog.set(false);
                    db_init_notified.set(false);
                    // Restart will re-init normally with the key now in keyring
                    db_resource.restart();
                }
                Err(_) => {
                    recovery_error.set(true);
                }
            }
        });
    };

    let handle_reset = move |_: ()| {
        let db_path = std::env::current_dir()
            .unwrap_or_default()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        let _ = crate::backend::db_key::reset_database(&db_path);
        db_init_notified.set(false);
        db_resource.restart();
    };

    rsx! {
        Style {}
        div { class: "flex gap-4 justify-center items-center h-screen",
            Spinner {
                size: SpinnerSize::XXXXLarge,
                color_class: "text-blue-500",
            }
        }

        RecoveryKeyInputDialog {
            open: show_recovery_dialog,
            error: recovery_error,
            on_recover: move |p: String| handle_recover(p),
            on_reset: move |_| show_reset_dialog.set(true),
        }

        DatabaseResetDialog {
            open: show_reset_dialog,
            on_confirm: handle_reset,
        }
    }
}

fn render_salt_error_ui(
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_reset_dialog: Signal<bool>,
    error_msg: String,
    mut db_init_notified: Signal<bool>,
) -> Element {
    let handle_reset = move |_: ()| {
        let db_path = std::env::current_dir()
            .unwrap_or_default()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        let _ = crate::backend::db_key::reset_database(&db_path);
        db_init_notified.set(false);
        db_resource.restart();
    };

    rsx! {
        Style {}
        div { class: "error-container",
            h1 { "Critical Database Error" }
            p { "{error_msg}" }
            button { onclick: move |_| show_reset_dialog.set(true), "Reset database" }
        }

        DatabaseResetDialog {
            open: show_reset_dialog,
            on_confirm: handle_reset,
        }
    }
}

fn main() {
    // Nota: il logging viene inizializzato automaticamente nel launcher
    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("PWDManager v{}", APP_VERSION);
    launch_desktop!(App, APP_VERSION);
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
