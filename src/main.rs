#![cfg_attr(not(debug_assertions), windows_subsystem = "console")]
mod auth;
mod backend;
mod components;

use crate::auth::User;
use crate::backend::db_backend::InitResult;
use crate::backend::init_blacklist_from_path;
use crate::backend::settings_types::AutoUpdate;
use crate::backend::updater_types::{UpdateManifest, UpdateState};
use crate::components::{
    AuthWrapper, Dashboard, LandingPage, Login, Logout, NavBar, PageNotFound, RouteWrapper,
    Settings, Spinner, SpinnerSize, Style, ToastContainer, ToastHubState, UpdateNotification,
    UpsertUser, show_toast_error, show_toast_success,
};
use crate::components::{DatabaseResetDialog, RecoveryKeyInputDialog, RecoveryKeySetupDialog};
use backend::db_backend::init_db;
use backend::db_backend::{build_sqlcipher_options, get_db_path};
use backend::settings_types::Theme;
use dioxus::core::Task;
use dioxus::prelude::*;
use gui_launcher::launch_desktop;
use secrecy::ExposeSecret;

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
#[allow(clippy::redundant_closure)]
fn App() -> Element {
    let auth_state = auth::AuthState::new();
    use_context_provider(move || auth_state);
    let app_theme = use_signal(|| Theme::Light);
    let auto_update = use_signal(|| AutoUpdate::default());
    use_context_provider(move || app_theme);
    use_context_provider(|| auto_update);
    let update_state = use_signal(|| UpdateState::Idle);
    use_context_provider(|| update_state);
    let update_manifest = use_signal(|| None::<UpdateManifest>);
    use_context_provider(|| update_manifest);
    use_context_provider(|| Signal::new(ToastHubState::default()));

    let mut db_resource = use_resource(move || async move { init_db().await });
    let db_resource_clone_drop = db_resource;
    let spawn_handle = use_signal(|| Option::<Task>::None);
    let toast_state = use_context::<Signal<ToastHubState>>();
    let mut db_init_notified = use_signal(|| false);

    // Recovery dialog state
    let mut show_recovery_dialog = use_signal(|| false);
    let recovery_error = use_signal(|| None::<String>);
    let show_reset_dialog = use_signal(|| false);
    #[cfg(debug_assertions)]
    let mut show_setup_dialog = use_signal(|| false);
    #[cfg(debug_assertions)]
    let mut setup_passphrase = use_signal(|| String::new());
    #[cfg(debug_assertions)]
    let mut has_shown_setup = use_signal(|| false);

    // Cleanup del pool quando il componente viene smontato o l'app si chiude
    use_drop(move || {
        let db_resource_clone = db_resource_clone_drop;
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
            let error = format!("BLACKLIST Loading is Failed!: {}", e);
            show_toast_error(error, toast_state);
        }
    });

    // Effect: handle DB resource changes
    use_effect(move || {
        let db_resource_clone = db_resource;
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
                if !db_init_notified() {
                    show_toast_error("Recovery key required".into(), toast_state);
                    db_init_notified.set(true);
                }
                show_recovery_dialog.set(true);
            }
            Some(Err(_)) => {
                show_toast_error("Database Loading failed!".into(), toast_state);
            }
            None => {}
        }
    });

    // Effect: debug user list
    use_effect(move || {
        let db_resource_clone = db_resource;
        #[allow(unused_variables)]
        match &*db_resource_clone.read() {
            Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) => {
                let mut spawn_handle = spawn_handle;
                if let Some(new_handle) = spawn_handle.take() {
                    new_handle.cancel();
                }
            }
            _ => {}
        }
    });

    // Effect: detect FirstSetup and show dialog (dev only — release uses --setup via NSIS)
    #[cfg(debug_assertions)]
    use_effect(move || {
        let resource = db_resource.read();
        if let Some(Ok(InitResult::FirstSetup {
            recovery_phrase, ..
        })) = &*resource
            && !has_shown_setup()
        {
            setup_passphrase.set(recovery_phrase.expose_secret().to_string());
            has_shown_setup.set(true);
            show_setup_dialog.set(true);
        }
    });

    // Provide pool context before rendering (must happen within the component)
    if let Some(Ok(InitResult::Ready(pool))) | Some(Ok(InitResult::FirstSetup { pool, .. })) =
        &*db_resource.read()
    {
        use_context_provider(|| pool.clone());
    }

    let content: Element = match &*db_resource.read() {
        Some(Ok(InitResult::Ready(_))) | Some(Ok(InitResult::FirstSetup { .. })) => {
            #[cfg(debug_assertions)]
            {
                render_app_with_setup(show_setup_dialog, setup_passphrase, update_state)
            }
            #[cfg(not(debug_assertions))]
            {
                render_app(update_state)
            }
        }
        Some(Err(custom_errors::DBError::DBKeyMissingWithDb)) => render_recovery_ui(
            db_resource,
            show_recovery_dialog,
            recovery_error,
            show_reset_dialog,
            db_init_notified,
            toast_state,
        ),
        Some(Err(custom_errors::DBError::DBSaltFileError(msg))) => render_salt_error_ui(
            db_resource,
            show_reset_dialog,
            msg.clone(),
            db_init_notified,
            toast_state,
        ),
        Some(Err(e)) => rsx! {
            div { class: "error-container",
                h1 { "Critical Database Error" }
                p { "{e}" }
                button { onclick: move |_| db_resource.restart(), "Retry" }
            }
        },
        None => rsx! {
            div { class: "flex gap-4 justify-center items-center h-screen",
                Spinner {
                    size: SpinnerSize::XXXXLarge,
                    color_class: "text-blue-500",
                }
            }
        },
    };

    rsx! {
        Style {}
        {content}
    }
}

fn render_app_with_setup(
    show_setup_dialog: Signal<bool>,
    setup_passphrase: Signal<String>,
    update_state: Signal<UpdateState>,
) -> Element {
    rsx! {
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

#[cfg(not(debug_assertions))]
fn render_app(update_state: Signal<UpdateState>) -> Element {
    rsx! {
        ToastContainer {}
        UpdateNotification { update_state }
        Router::<Route> {}
    }
}

fn render_recovery_ui(
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_recovery_dialog: Signal<bool>,
    mut recovery_error: Signal<Option<String>>,
    mut show_reset_dialog: Signal<bool>,
    mut db_init_notified: Signal<bool>,
    toast_state: Signal<ToastHubState>,
) -> Element {
    let handle_recover = move |passphrase: String| {
        let passphrase = passphrase.clone();
        spawn(async move {
            let db_path = match get_db_path() {
                Ok(p) => p,
                Err(e) => {
                    recovery_error.set(Some(e.to_string()));
                    return;
                }
            };

            let derive_result = tokio::task::spawn_blocking({
                let p = passphrase.clone();
                let path = db_path.clone();
                move || crate::backend::db_key::derive_key_from_passphrase(&p, &path)
            })
            .await;

            let key = match derive_result {
                Ok(Ok(key)) => key,
                Ok(Err(e)) => {
                    tracing::warn!("Recovery key derivation failed: {}", e);
                    recovery_error.set(Some(e.to_string()));
                    return;
                }
                Err(join_err) => {
                    tracing::error!("Recovery derivation panicked: {}", join_err);
                    recovery_error.set(Some("Key derivation failed unexpectedly".into()));
                    return;
                }
            };

            // Try to open DB with derived key
            let opts = match build_sqlcipher_options(&db_path, &key) {
                Ok(o) => o,
                Err(e) => {
                    recovery_error.set(Some(e.to_string()));
                    return;
                }
            };

            match sqlx::SqlitePool::connect_with(opts).await {
                Ok(_pool) => {
                    // Store key in keyring
                    let _ = crate::backend::db_key::store_db_key(
                        crate::backend::db_key::keyring_service_name(),
                        crate::backend::db_key::KEY_USERNAME,
                        &key,
                    );
                    recovery_error.set(None);
                    show_recovery_dialog.set(false);
                    show_toast_success("Database online!".into(), toast_state);
                    db_init_notified.set(true);
                    db_resource.restart();
                }
                Err(_) => {
                    let err = crate::backend::db_key::DBKeyError::RecoveryKeyInvalid.to_string();
                    tracing::warn!("Recovery key invalid: {}", err);
                    recovery_error.set(Some(err));
                }
            }
        });
    };

    let handle_reset =
        move |_: ()| handle_reset_callback(db_init_notified, db_resource, toast_state);

    rsx! {
        ToastContainer {}
        div { class: "flex gap-4 justify-center items-center h-screen",
            Spinner { size: SpinnerSize::XXXXLarge, color_class: "text-blue-500" }
        }

        RecoveryKeyInputDialog {
            open: show_recovery_dialog,
            error: recovery_error,
            on_recover: move |p: String| handle_recover(p),
            on_reset: move |_| show_reset_dialog.set(true),
        }

        DatabaseResetDialog { open: show_reset_dialog, on_confirm: handle_reset }
    }
}

fn render_salt_error_ui(
    db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    mut show_reset_dialog: Signal<bool>,
    error_msg: String,
    db_init_notified: Signal<bool>,
    toast_state: Signal<ToastHubState>,
) -> Element {
    let handle_reset =
        move |_: ()| handle_reset_callback(db_init_notified, db_resource, toast_state);

    rsx! {
        ToastContainer {}
        div { class: "error-container",
            h1 { "Critical Database Error" }
            p { "{error_msg}" }
            button { onclick: move |_| show_reset_dialog.set(true), "Reset database" }
        }

        DatabaseResetDialog { open: show_reset_dialog, on_confirm: handle_reset }
    }
}

fn handle_reset_callback(
    mut db_init_notified: Signal<bool>,
    mut db_resource: Resource<Result<InitResult, custom_errors::DBError>>,
    toast_state: Signal<ToastHubState>,
) {
    let db_path = match get_db_path() {
        Ok(p) => p,
        Err(_) => return,
    };

    match crate::backend::db_key::reset_database(&db_path) {
        Ok(()) => {
            db_init_notified.set(false);
            db_resource.restart();
        }
        Err(e) => {
            show_toast_error(format!("Failed to reset database: {}", e), toast_state);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--setup".to_string()) {
        if cfg!(debug_assertions) {
            eprintln!("Error: --setup is not available in debug builds");
            std::process::exit(1);
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        match rt.block_on(crate::backend::setup::run_setup()) {
            Ok(passphrase) => {
                println!("{}", passphrase.expose_secret());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Setup failed: {}", e);
                std::process::exit(1);
            }
        }
    }

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
