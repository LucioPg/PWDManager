// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Auto-start management.
//!
//! - Windows: reads/writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! - Linux: manages `.desktop` file in `~/.config/autostart/`

use std::fmt;

/// Errors that can occur during auto-start operations.
#[derive(Debug)]
pub enum AutoStartError {
    RegistryError(String),
    DesktopEntryError(String),
    ExePathError(String),
}

impl fmt::Display for AutoStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutoStartError::RegistryError(msg) => {
                write!(f, "Failed to access registry: {msg}")
            }
            AutoStartError::DesktopEntryError(msg) => {
                write!(f, "Failed to manage desktop entry: {msg}")
            }
            AutoStartError::ExePathError(msg) => {
                write!(f, "Failed to get executable path: {msg}")
            }
        }
    }
}

impl std::error::Error for AutoStartError {}

// ── Platform dispatch ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// Checks whether auto-start is currently enabled.
#[cfg(target_os = "windows")]
pub fn is_enabled() -> bool {
    windows::is_enabled()
}

/// Enables auto-start.
#[cfg(target_os = "windows")]
pub fn enable() -> Result<(), AutoStartError> {
    windows::enable()
}

/// Disables auto-start.
#[cfg(target_os = "windows")]
pub fn disable() -> Result<(), AutoStartError> {
    windows::disable()
}

#[cfg(target_os = "linux")]
pub use linux::{disable, enable, is_enabled};

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn enable() -> Result<(), AutoStartError> {
    Err(AutoStartError::DesktopEntryError(
        "Auto-start not supported on this platform".into(),
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn disable() -> Result<(), AutoStartError> {
    Err(AutoStartError::DesktopEntryError(
        "Auto-start not supported on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_registry_value_quotes_path() {
        let value = windows::build_registry_value(r"C:\Program Files\PWDManager\PWDManager.exe");
        assert_eq!(
            value,
            r#""C:\Program Files\PWDManager\PWDManager.exe" --minimized"#
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_registry_value_simple_path() {
        let value = windows::build_registry_value(r"C:\Apps\PWDManager.exe");
        assert_eq!(value, r#""C:\Apps\PWDManager.exe" --minimized"#);
    }

    #[test]
    fn test_auto_start_error_display_registry() {
        let err = AutoStartError::RegistryError("access denied".into());
        assert_eq!(err.to_string(), "Failed to access registry: access denied");
    }

    #[test]
    fn test_auto_start_error_display_desktop() {
        let err = AutoStartError::DesktopEntryError("write failed".into());
        assert_eq!(
            err.to_string(),
            "Failed to manage desktop entry: write failed"
        );
    }

    #[test]
    fn test_auto_start_error_display_exe_path() {
        let err = AutoStartError::ExePathError("not found".into());
        assert_eq!(err.to_string(), "Failed to get executable path: not found");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_startup_disabled_flag() {
        assert_eq!(windows::STARTUP_DISABLED_FLAG, 0x03);
    }
}
