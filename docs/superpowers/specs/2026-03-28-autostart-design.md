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

New module with three public functions:

- `is_enabled() -> bool` — reads `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` key named `PwdManager`
- `enable() -> Result<(), AutoStartError>` — writes the key with value `"<exe_path>" --minimized`
- `disable() -> Result<(), AutoStartError>` — removes the key

The executable path is obtained via `std::env::current_exe()`.

### Error Type

```rust
enum AutoStartError {
    RegistryError(String),  // winreg operation failed
    ExePathError(String),   // current_exe() failed
}
```

### Registry Key Details

- **Hive:** `HKEY_CURRENT_USER` (no admin required)
- **Path:** `Software\Microsoft\Windows\CurrentVersion\Run`
- **Value name:** `PwdManager`
- **Value data:** Full path to executable + ` --minimized` flag

## UI Changes: `GeneralSettings`

- New toggle row "Auto Start" placed after "Auto Update"
- On component mount: calls `is_enabled()` (async via `use_resource`) to set initial toggle state
- On toggle change: calls `enable()` or `disable()` immediately, shows toast on error
- Writing is immediate — independent of the Save button
- Save button continues to persist only DB-backed settings (theme, auto_update, auto_logout)

## Startup Behavior: `main.rs`

- Parse CLI arguments before Dioxus launch
- If `--minimized` flag is present: start with window hidden (`window().set_visible(false)`)
- The system tray remains active and functional — user clicks tray icon to show window

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
- `gui_launcher` — no changes needed

## Task Manager Behavior

The registry key appears in Task Manager's Startup tab. If the user disables it there, Windows moves the key to `...\Run\Disabled`. On next app open, `is_enabled()` will return `false`, and the toggle will correctly show OFF. Re-toggling ON will re-enable it.

## Edge Cases

- **Path with spaces:** The full exe path is not quoted in the registry by default. If needed, add quotes around the path when writing the registry value.
- **Portable installs:** If the exe is moved, the registry key will point to the old path. This is acceptable — the user can re-toggle to update the path.
- **Multiple users:** Each Windows user has their own `HKCU`, so auto-start is per Windows user. This is correct behavior.
