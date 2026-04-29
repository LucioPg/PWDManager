// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Windows auto-start via registry.

use super::AutoStartError;

/// Registry value name used for auto-start.
pub const VALUE_NAME: &str = "PwdManager";

/// Registry sub-path under HKCU.
const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Registry path for Task Manager's disabled-startup tracking.
const STARTUP_APPROVED_PATH: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";

/// Windows error code for "file not found" (ERROR_FILE_NOT_FOUND).
const WIN_ERROR_FILE_NOT_FOUND: i32 = 2;

/// First byte value in StartupApproved\Run binary data meaning "disabled".
pub const STARTUP_DISABLED_FLAG: u8 = 0x03;

/// Builds the registry value string: `"<exe_path>" --minimized`.
pub fn build_registry_value(exe_path: &str) -> String {
    format!("\"{exe_path}\" --minimized")
}

/// Checks whether auto-start is currently enabled.
pub fn is_enabled() -> bool {
    let run_key = match winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY_PATH)
    {
        Ok(key) => key,
        Err(_) => return false,
    };

    if run_key.get_value::<String, _>(VALUE_NAME).is_err() {
        return false;
    }

    match winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(STARTUP_APPROVED_PATH)
        .and_then(|key| key.get_raw_value(VALUE_NAME))
    {
        Ok(rv) => rv.bytes.first() != Some(&STARTUP_DISABLED_FLAG),
        Err(_) => true,
    }
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
        .map_err(|e| AutoStartError::RegistryError(e.to_string()))?;

    if let Ok(approved_key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(STARTUP_APPROVED_PATH, winreg::enums::KEY_SET_VALUE)
    {
        let _ = approved_key.delete_value(VALUE_NAME);
    }

    Ok(())
}

/// Disables auto-start by removing the value from the Run key.
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
