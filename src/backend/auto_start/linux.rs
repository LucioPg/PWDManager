// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

//! Linux auto-start via XDG `.desktop` file in `~/.config/autostart/`.

use super::AutoStartError;

const DESKTOP_FILE_NAME: &str = "PWDManager.desktop";

/// Returns `~/.config/autostart/PWDManager.desktop`.
fn autostart_path() -> Result<std::path::PathBuf, AutoStartError> {
    dirs::config_dir()
        .map(|p| p.join("autostart").join(DESKTOP_FILE_NAME))
        .ok_or_else(|| AutoStartError::DesktopEntryError("Cannot determine config dir".into()))
}

/// Builds the `.desktop` file content.
fn desktop_entry_content(exe_path: &str, hidden: bool) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=PWDManager\n\
         Exec=\"{exe_path}\" --minimized\n\
         Hidden={}\n",
        if hidden { "true" } else { "false" }
    )
}

/// Checks whether auto-start is currently enabled.
///
/// Enabled when the file exists and `Hidden` is not `true`.
pub fn is_enabled() -> bool {
    let path = match autostart_path() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if !path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // If Hidden=true, the entry is disabled (XDG spec).
    !content.lines().any(|line| line.trim() == "Hidden=true")
}

/// Enables auto-start by creating/overwriting the `.desktop` file.
pub fn enable() -> Result<(), AutoStartError> {
    let exe_path =
        std::env::current_exe().map_err(|e| AutoStartError::ExePathError(e.to_string()))?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| AutoStartError::ExePathError("path is not valid UTF-8".into()))?;

    let path = autostart_path()?;

    // Ensure ~/.config/autostart/ exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AutoStartError::DesktopEntryError(format!("Cannot create autostart dir: {e}")))?;
    }

    let content = desktop_entry_content(exe_str, false);
    std::fs::write(&path, content)
        .map_err(|e| AutoStartError::DesktopEntryError(format!("Cannot write desktop entry: {e}")))?;

    Ok(())
}

/// Disables auto-start by removing the `.desktop` file.
///
/// Returns `Ok(())` if the file doesn't exist.
pub fn disable() -> Result<(), AutoStartError> {
    let path = autostart_path()?;

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| AutoStartError::DesktopEntryError(format!("Cannot remove desktop entry: {e}")))?;
    }

    Ok(())
}
