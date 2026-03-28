# Auto-Start at Boot — Design Spec

**Date:** 2026-03-28
**Status:** Approved

## Overview

Enable the app to start automatically at Windows boot, minimized in the system tray. The user controls this via a toggle in GeneralSettings. The Windows registry (`HKCU\...\Run`) is the single source of truth — no DB storage.

## Architecture

```
GeneralSettings (UI)
    |
    +-- Toggle "Auto Start" -- reads/writes directly --> Registry (HKCU\...\Run)
    |                                                       |
    |                                                       v
    |                                                 App starts with --minimized
    |
    +-- Save button --> UserSettings (DB) -- unchanged
```

## Backend Module: `src/backend/auto_start.rs`

New module gated with `#[cfg(target_os = "windows")]`. Three public synchronous functions:

- `is_enabled() -> bool` — reads `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` key named `PwdManager`
- `enable() -> Result<(), AutoStartError>` — writes the key with value `"\"<exe_path>\" --minimized"`
- `disable() -> Result<(), AutoStartError>` — removes the key

The executable path is obtained via `std::env::current_exe()`. All functions are synchronous — the `winreg` crate calls Windows API directly. Async wrapping (e.g. `tokio::task::spawn_blocking`) happens at the call site if needed.

### Error Type

```rust
#[derive(Debug)]
enum AutoStartError {
    RegistryError(String),  // winreg operation failed
    ExePathError(String),   // current_exe() failed
}

impl std::fmt::Display for AutoStartError { ... }
impl std::error::Error for AutoStartError { ... }
```

### Registry Key Details

- **Hive:** `HKEY_CURRENT_USER` (no admin required)
- **Path:** `Software\Microsoft\Windows\CurrentVersion\Run`
- **Value name:** `PwdManager`
- **Value data:** `"<exe_path>" --minimized` (path MUST always be quoted — the default install path `C:\Program Files\PWDManager\PWDManager.exe` contains spaces)

## UI Changes: `GeneralSettings`

- New toggle row "Auto Start" placed after "Auto Update"
- On component mount: calls `is_enabled()` via `use_resource` (wrapping the sync call) to set initial toggle state
- On toggle change: calls `enable()` or `disable()` immediately via `spawn_blocking`, shows toast on error
- Writing is immediate — independent of the Save button
- Save button continues to persist only DB-backed settings (theme, auto_update, auto_logout)

## Startup Behavior: `main.rs` + `gui_launcher`

- Parse `--minimized` CLI argument in `main()` before Dioxus launch
- Pass `start_visible: bool` to `create_desktop_config()` in `gui_launcher`
- `WindowBuilder` uses `.with_visible(start_visible)` — the window is never shown at all (no flash)
- The system tray remains active and functional — user clicks tray icon to show window

### Changes to `gui_launcher`

`create_desktop_config(app_version: &str)` becomes `create_desktop_config(app_version: &str, start_visible: bool)`. The `launch_desktop!` macro is updated accordingly. This is a minimal change — one parameter + one `.with_visible()` call on `WindowBuilder`.

### Launch Command (stored in registry)

```
"C:\path\to\PWDManager.exe" --minimized
```

## Dependencies

- Add `winreg` crate to `Cargo.toml` (standard Windows registry access crate)

## What Does NOT Change

- `UserSettings` struct and `user_settings` DB table — no new field
- Save button behavior — continues to save only theme, auto_update, auto_logout_settings
- System tray — already implemented, no modifications

## Task Manager Behavior

The registry key appears in Task Manager's Startup tab. If the user disables it there, Windows moves the key to `...\Run\Disabled`. On next app open, `is_enabled()` will return `false`, and the toggle will correctly show OFF. Re-toggling ON will re-enable it.

## Edge Cases

- **Path with spaces:** The exe path MUST always be quoted when writing the registry value. The stored format is `"<exe_path>" --minimized`. Without quotes, the `--minimized` flag is parsed as a separate executable.
- **Portable installs:** If the exe is moved, the registry key will point to the old path. This is acceptable — the user can re-toggle to update the path.
- **Multiple users:** Each Windows user has their own `HKCU`, so auto-start is per Windows user. This is correct behavior.
