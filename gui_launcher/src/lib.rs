//! HotDog Custom Launcher
//!
//! This crate provides a custom launcher for the HotDog desktop application
//! that handles window configuration and icon loading without affecting
//! the main application behavior.

use dioxus::desktop::{Config, WindowBuilder, LogicalSize};
// per testare i percorsi delle icone
use std::path::Path;

// funzione di check per il path
pub fn check_paths() -> String {
    let icon_paths = [
        "../icons/icon.png",      // Sibling directory structure
        "icons/icon.png",         // Current directory
        "../../icons/icon.png",   // Different relative path
    ];
    let mut results: Vec<String> = Vec::new();
    for path in &icon_paths {
        if Path::new(&path).exists() {
            results.push(path.to_string());
        }
    }
    if results.is_empty() {
        "None of the expected paths exist".to_string()
    }
    else {
        results.join(", ")
    }
}

/// Loads window icon from file system
pub fn load_window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    // Try to load icon from parent directory (hot_dog/icons/)
    let icon_paths = [
        "../icons/icon.png",      // Sibling directory structure
        "icons/icon.png",         // Current directory
        "../../icons/icon.png",   // Different relative path
    ];

    for icon_path in &icon_paths {
        if let Ok(icon_bytes) = std::fs::read(icon_path) {
            tracing::info!("Attempting to load icon from: {}", icon_path);

            match image::load_from_memory(&icon_bytes) {
                Ok(img) => {
                    let rgba_img = img.to_rgba8();
                    let (width, height) = rgba_img.dimensions();
                    let rgba_data = rgba_img.into_raw();

                    match dioxus::desktop::tao::window::Icon::from_rgba(rgba_data, width, height) {
                        Ok(icon) => {
                            tracing::info!("Window icon loaded successfully: {}x{} from {}", width, height, icon_path);
                            return Some(icon);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to create window icon from {}: {}", icon_path, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Could not decode image from {}: {}", icon_path, e);
                }
            }
        }
    }

    tracing::warn!("No suitable icon found in any of the expected paths");
    None
}

/// Creates a desktop configuration with custom window settings
pub fn create_desktop_config() -> Config {
    let window_icon = load_window_icon();

    Config::new().with_window(
        WindowBuilder::new()
            .with_title("HotDog")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_resizable(true)
            .with_window_icon(window_icon)
    )
}

/// Macro to launch the application with custom desktop configuration
#[macro_export]
macro_rules! launch_desktop {
    ($app:expr) => {
        {
            tracing::info!("Using custom desktop launcher configuration");

            let config = $crate::create_desktop_config();

            dioxus::LaunchBuilder::new()
                .with_cfg(config)
                .launch($app);
        }
    };
}