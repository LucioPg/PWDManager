//! Auto-start management via Windows registry.
//!
//! Reads/writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` to enable
//! the app to start automatically at Windows boot.

use std::fmt;

/// Registry value name used for auto-start.
const VALUE_NAME: &str = "PwdManager";

/// Registry sub-path under HKCU.
const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Windows error code for "file not found" (ERROR_FILE_NOT_FOUND).
/// Used to gracefully handle deleting a non-existent registry value.
const WIN_ERROR_FILE_NOT_FOUND: i32 = 2;

/// Errors that can occur during auto-start operations.
#[derive(Debug)]
pub enum AutoStartError {
    RegistryError(String),
    ExePathError(String),
}

impl fmt::Display for AutoStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutoStartError::RegistryError(msg) => {
                write!(f, "Failed to access registry: {msg}")
            }
            AutoStartError::ExePathError(msg) => {
                write!(f, "Failed to get executable path: {msg}")
            }
        }
    }
}

impl std::error::Error for AutoStartError {}

/// Builds the registry value string: `"<exe_path>" --minimized`.
///
/// The path is always quoted to handle spaces (e.g. `C:\Program Files\...`).
fn build_registry_value(exe_path: &str) -> String {
    format!("\"{exe_path}\" --minimized")
}

/// Checks whether auto-start is currently enabled.
///
/// Returns `true` if ANY value named `PwdManager` exists in the Run key,
/// regardless of its content. This intentionally covers cases where the
/// user or Task Manager has modified the value.
pub fn is_enabled() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY_PATH)
        .and_then(|key| key.get_value::<String, _>(VALUE_NAME))
        .is_ok()
}

/// Enables auto-start by writing the current executable path to the Run key.
pub fn enable() -> Result<(), AutoStartError> {
    let exe_path =
        std::env::current_exe().map_err(|e| AutoStartError::ExePathError(e.to_string()))?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| AutoStartError::ExePathError("path is not valid UTF-8".into()))?;

    let value = build_registry_value(exe_str);

    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY_PATH, winreg::enums::KEY_SET_VALUE)
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))?
        .set_value(VALUE_NAME, &value)
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))
}

/// Disables auto-start by removing the value from the Run key.
///
/// Returns `Ok(())` even if the value didn't exist.
pub fn disable() -> Result<(), AutoStartError> {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY_PATH, winreg::enums::KEY_SET_VALUE)
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))?;

    match key.delete_value(VALUE_NAME) {
        Ok(()) => Ok(()),
        Err(ref e) if e.raw_os_error() == Some(WIN_ERROR_FILE_NOT_FOUND) => Ok(()),
        Err(e) => Err(AutoStartError::RegistryError(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_registry_value_quotes_path() {
        let value = build_registry_value(r"C:\Program Files\PWDManager\PWDManager.exe");
        assert_eq!(
            value,
            r#""C:\Program Files\PWDManager\PWDManager.exe" --minimized"#
        );
    }

    #[test]
    fn test_build_registry_value_simple_path() {
        let value = build_registry_value(r"C:\Apps\PWDManager.exe");
        assert_eq!(value, r#""C:\Apps\PWDManager.exe" --minimized"#);
    }

    #[test]
    fn test_auto_start_error_display_registry() {
        let err = AutoStartError::RegistryError("access denied".into());
        assert_eq!(err.to_string(), "Failed to access registry: access denied");
    }

    #[test]
    fn test_auto_start_error_display_exe_path() {
        let err = AutoStartError::ExePathError("not found".into());
        assert_eq!(err.to_string(), "Failed to get executable path: not found");
    }
}
