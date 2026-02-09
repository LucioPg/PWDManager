//! PWDManager Custom Launcher
//!
//! This crate provides a custom launcher for the PWDManager desktop application
//! that handles window configuration and icon loading without affecting
//! the main application behavior.

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use std::path::PathBuf;
use std::sync::OnceLock;

// Global guard to keep logging alive
static LOGGING_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Initialize logging to file in AppData
pub fn init_logging() {
    // Only initialize once
    if LOGGING_GUARD.get().is_some() {
        return;
    }

    let app_data_dir = if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("PWDManager"))
    } else {
        dirs::data_local_dir().map(|p| p.join("PWDManager"))
    };

    if let Some(dir) = app_data_dir {
        // Crea la cartella se non esiste
        std::fs::create_dir_all(&dir).ok();

        let log_path = dir.join("pwdmanager.log");

        let file_appender = tracing_appender::rolling::never(&dir, "pwdmanager.log");

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        // Store the guard globally to keep it alive for the entire program duration
        let _ = LOGGING_GUARD.set(guard);

        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_max_level(tracing::Level::INFO)
            .try_init()
            .ok(); // Ignore if already initialized

        tracing::info!("Logging initialized - log file: {:?}", log_path);
    }

    // Log working directory like 'pwd'
    if let Ok(cwd) = std::env::current_dir() {
        tracing::info!("Working directory (PWD): {:?}", cwd);
    }

    // Log executable path
    if let Ok(exe) = std::env::current_exe() {
        tracing::info!("Executable path: {:?}", exe);
        if let Some(exe_dir) = exe.parent() {
            tracing::info!("Executable directory: {:?}", exe_dir);
        }
    }
}

/// Loads window icon from embedded bytes (included in executable at compile time)
pub fn load_window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    // Icon is embedded directly in the executable at compile time
    // No need to search for external files!
    const ICON_BYTES: &[u8] = include_bytes!("../../icons/icon.png");

    tracing::info!(
        "Loading embedded window icon from executable ({} bytes)",
        ICON_BYTES.len()
    );

    match image::load_from_memory(ICON_BYTES) {
        Ok(img) => {
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let rgba_data = rgba_img.into_raw();

            match dioxus::desktop::tao::window::Icon::from_rgba(rgba_data, width, height) {
                Ok(icon) => {
                    tracing::info!(
                        "Window icon loaded successfully: {}x{} (embedded)",
                        width,
                        height
                    );
                    Some(icon)
                }
                Err(e) => {
                    tracing::warn!("Failed to create window icon from embedded data: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("Could not decode embedded icon image: {}", e);
            None
        }
    }
}

/// Creates a desktop configuration with custom window settings
pub fn create_desktop_config() -> Config {
    let window_icon = load_window_icon();

    // Calculate AppData path for WebView2 data folder to avoid security issues
    // This prevents creating .exe.WebView2 folder next to the executable
    let data_dir = if cfg!(windows) {
        // Use %LOCALAPPDATA% for Windows (e.g., C:\Users\<User>\AppData\Local)
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("PWDManager"))
    } else {
        // For other platforms, use a standard application data directory
        dirs::data_local_dir().map(|p| p.join("PWDManager"))
    };

    let mut config = Config::new().with_window(
        WindowBuilder::new()
            .with_title("PWDManager")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_resizable(true)
            .with_window_icon(window_icon),
    );

    // Set data directory to avoid creating .exe.WebView2 folder next to exe
    if let Some(dir) = data_dir {
        tracing::info!("Setting WebView2 data directory to: {:?}", dir);
        config = config.with_data_directory(dir);
    }

    config
}

/// Macro to launch the application with custom desktop configuration
#[macro_export]
macro_rules! launch_desktop {
    ($app:expr) => {{
        // Initialize logging first (idempotent, safe to call multiple times)
        $crate::init_logging();

        tracing::info!("Using custom desktop launcher configuration");

        let config = $crate::create_desktop_config();

        dioxus::LaunchBuilder::new().with_cfg(config).launch($app);
    }};
}
