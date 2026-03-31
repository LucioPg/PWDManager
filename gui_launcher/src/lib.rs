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

/// Impostare a `true` per forzare l'abilitazione del devtools, anche in release.
/// Da impostare a `false` per la release finale.
const FORCE_ENABLE_DEVTOOLS: bool = false;

/// Singleton instance management via Windows named mutex.
///
/// Prevents multiple instances of the application from running simultaneously.
/// If a second instance is detected, the first instance's window is brought
/// to the foreground and the second instance exits.
#[cfg(target_os = "windows")]
pub mod singleton {
    use std::ffi::c_void;
    use std::ptr;

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            lpMutexAttributes: *const c_void,
            bInitialOwner: i32,
            lpName: *const u16,
        ) -> *mut c_void;
        fn ReleaseMutex(hMutex: *mut c_void) -> i32;
        fn CloseHandle(hObject: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> isize;
        fn SetForegroundWindow(hWnd: isize) -> i32;
        fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
        fn IsIconic(hWnd: isize) -> i32;
    }

    /// RAII guard that holds the named mutex handle.
    /// The main instance uses `std::mem::forget` to keep the mutex alive
    /// for the entire process lifetime (Windows cleans up on exit).
    pub struct SingleInstanceGuard(*mut c_void);

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0);
                CloseHandle(self.0);
            }
        }
    }

    /// Attempts to acquire a named mutex to enforce single-instance behavior.
    ///
    /// The `Local\` prefix makes it session-scoped (per logged-in user).
    pub fn try_acquire(app_name: &str) -> Result<SingleInstanceGuard, ()> {
        let mutex_name: Vec<u16> = format!("Local\\{}", app_name)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateMutexW(ptr::null(), 1, mutex_name.as_ptr());
            if handle.is_null() {
                tracing::error!("Failed to create singleton mutex for {}", app_name);
                return Err(());
            }

            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                tracing::info!("Another instance of {} is already running", app_name);
                Err(())
            } else {
                tracing::info!("Singleton lock acquired for {}", app_name);
                Ok(SingleInstanceGuard(handle))
            }
        }
    }

    /// Brings a window with the given title to the foreground.
    /// Handles both minimized and hidden windows.
    pub fn bring_window_to_foreground(window_title: &str) {
        let title: Vec<u16> = window_title
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let hwnd = FindWindowW(ptr::null(), title.as_ptr());
            if hwnd == 0 {
                tracing::warn!(
                    "Singleton: could not find window '{}'",
                    window_title
                );
                return;
            }

            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else {
                ShowWindow(hwnd, SW_SHOW);
            }
            SetForegroundWindow(hwnd);
        }
    }
}

/// Non-Windows stub: always allows running (no singleton enforcement).
#[cfg(not(target_os = "windows"))]
pub mod singleton {
    pub struct SingleInstanceGuard;

    pub fn try_acquire(_app_name: &str) -> Result<SingleInstanceGuard, ()> {
        Ok(SingleInstanceGuard)
    }

    pub fn bring_window_to_foreground(_window_title: &str) {}
}

/// Creates a desktop configuration with custom window settings
pub fn create_desktop_config(app_version: &str, start_visible: bool) -> Config {
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
            .with_title(format!("PWDManager v{}", app_version))
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_resizable(true)
            .with_visible(start_visible)
            .with_window_icon(window_icon),
    );

    // System tray: X hides window instead of closing, app stays alive in background
    config = config
        .with_close_behaviour(dioxus::desktop::WindowCloseBehaviour::WindowHides)
        .with_exits_when_last_window_closes(false);

    // Set data directory to avoid creating .exe.WebView2 folder next to exe
    if let Some(dir) = data_dir {
        tracing::info!("Setting WebView2 data directory to: {:?}", dir);
        config = config.with_data_directory(dir);
    }

    // In release mode (a meno che FORCE_ENABLE_DEVTOOLS non sia true):
    // - disabilita devtools e context menu (click destro)
    // - rimuove il menu bar (Window, Edit, Help)
    if !cfg!(debug_assertions) && !FORCE_ENABLE_DEVTOOLS {
        config = config.with_disable_context_menu(true);
        config = config.with_menu(None);
    }

    config
}

/// Macro to launch the application with custom desktop configuration
/// and singleton instance enforcement.
#[macro_export]
macro_rules! launch_desktop {
    ($app:expr, $version:expr, $visible:expr) => {{
        $crate::init_logging();

        match $crate::singleton::try_acquire("PWDManager") {
            Ok(_guard) => {
                // First instance — keep mutex alive for the entire process lifetime.
                // std::mem::forget prevents Drop; Windows cleans up the handle on exit.
                std::mem::forget(_guard);
                let config = $crate::create_desktop_config($version, $visible);
                dioxus::LaunchBuilder::new().with_cfg(config).launch($app);
            }
            Err(()) => {
                // Another instance is running — activate its window and exit.
                let title = format!("PWDManager v{}", $version);
                $crate::singleton::bring_window_to_foreground(&title);
            }
        }
    }};
}
