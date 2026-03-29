//! Auto-start management via Windows registry.
//!
//! Reads/writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` to enable
//! the app to start automatically at Windows boot.

use std::fmt;

/// Registry value name used for auto-start.
const VALUE_NAME: &str = "PwdManager";

/// Registry sub-path under HKCU.
const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Registry path for Task Manager's disabled-startup tracking.
/// Windows stores a binary value per entry: first byte 0x02 = enabled, 0x03 = disabled.
const STARTUP_APPROVED_PATH: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";

/// Windows error code for "file not found" (ERROR_FILE_NOT_FOUND).
/// Used to gracefully handle deleting a non-existent registry value.
const WIN_ERROR_FILE_NOT_FOUND: i32 = 2;

/// First byte value in StartupApproved\Run binary data meaning "disabled".
const STARTUP_DISABLED_FLAG: u8 = 0x03;

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
/// Two conditions must be true:
/// 1. A value named `PwdManager` exists in the `Run` key
/// 2. The Task Manager has NOT disabled it (checked via `StartupApproved\Run`,
///    where first byte `0x03` means disabled)
pub fn is_enabled() -> bool {
    // Step 1: check if the Run key value exists
    let run_key = match winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY_PATH)
    {
        Ok(key) => key,
        Err(_) => return false,
    };

    if run_key.get_value::<String, _>(VALUE_NAME).is_err() {
        return false;
    }

    // Step 2: check StartupApproved — Task Manager marks disabled entries here
    // If the value doesn't exist in StartupApproved, the entry is enabled (default).
    // If it exists and first byte is 0x03, Task Manager disabled it.
    match winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(STARTUP_APPROVED_PATH)
        .and_then(|key| key.get_raw_value(VALUE_NAME))
    {
        Ok(rv) => rv.bytes.first() != Some(&STARTUP_DISABLED_FLAG),
        Err(_) => true, // No StartupApproved entry → enabled
    }
}

/// Enables auto-start by writing the current executable path to the Run key.
/// Also removes any `StartupApproved\Run` disabled flag so Task Manager
/// sees the entry as enabled again.
pub fn enable() -> Result<(), AutoStartError> {
    let exe_path =
        std::env::current_exe().map_err(|e| AutoStartError::ExePathError(e.to_string()))?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| AutoStartError::ExePathError("path is not valid UTF-8".into()))?;

    let value = build_registry_value(exe_str);

    // Write the Run key value
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY_PATH, winreg::enums::KEY_SET_VALUE)
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))?
        .set_value(VALUE_NAME, &value)
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))?;

    // Remove any StartupApproved disabled flag so Task Manager sees it as enabled.
    // Ignore errors — the key may not exist (user never disabled via Task Manager).
    if let Ok(approved_key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(STARTUP_APPROVED_PATH, winreg::enums::KEY_SET_VALUE)
    {
        let _ = approved_key.delete_value(VALUE_NAME);
    }

    Ok(())
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

    #[test]
    fn test_startup_disabled_flag() {
        // Verify the constant matches Windows Task Manager's disabled flag
        assert_eq!(STARTUP_DISABLED_FLAG, 0x03);
    }
}
