# Auto-Start at Boot — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable the app to auto-start at Windows boot, minimized in the system tray, controlled via a toggle in GeneralSettings.

**Architecture:** A backend module (`auto_start.rs`) reads/writes the Windows registry `HKCU\...\Run` key. The `gui_launcher` accepts a `start_visible` flag to create the window hidden when launched with `--minimized`. The GeneralSettings toggle calls `is_enabled()`/`enable()`/`disable()` directly.

**Tech Stack:** `winreg` crate, Dioxus 0.7 signals, `tao::WindowBuilder::with_visible()`

**Spec:** `docs/superpowers/specs/2026-03-28-autostart-design.md`

---

### Task 1: Add `winreg` dependency

**Files:**
- Modify: `Cargo.toml:28-53` (dependencies section)

- [ ] **Step 1: Add winreg to Cargo.toml**

Add after the `rand` dependency (line 53):

```toml
winreg = "0.55"
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully (new dep fetched)

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add winreg dependency for auto-start registry access"
```

---

### Task 2: Create `auto_start.rs` backend module

**Files:**
- Create: `src/backend/auto_start.rs`
- Modify: `src/backend/mod.rs:1` (add module declaration)

- [ ] **Step 1: Write tests first**

Create `src/backend/auto_start.rs` with tests for value formatting and error display:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test auto_start -- --nocapture`
Expected: FAIL (functions not defined)

- [ ] **Step 3: Write implementation**

Create `src/backend/auto_start.rs`:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test auto_start -- --nocapture`
Expected: All 4 tests PASS

- [ ] **Step 5: Register module in `backend/mod.rs`**

Add at line 1 (before existing modules):

```rust
#[cfg(target_os = "windows")]
pub mod auto_start;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 7: Commit**

```bash
git add src/backend/auto_start.rs src/backend/mod.rs
git commit -m "feat: add auto_start module for registry-based boot launch"
```

---

### Task 3: Update `gui_launcher` and `main.rs` for `--minimized` support

**Files:**
- Modify: `gui_launcher/src/lib.rs:109-167`
- Modify: `src/main.rs:454-483`

- [ ] **Step 1: Modify `create_desktop_config` signature and body**

In `gui_launcher/src/lib.rs`, change the function signature at line 109:

```rust
pub fn create_desktop_config(app_version: &str, start_visible: bool) -> Config {
```

Add `.with_visible(start_visible)` to the `WindowBuilder` chain:

```rust
let mut config = Config::new().with_window(
    WindowBuilder::new()
        .with_title(format!("PWDManager v{}", app_version))
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_resizable(true)
        .with_visible(start_visible)
        .with_window_icon(window_icon),
);
```

- [ ] **Step 2: Update the `launch_desktop!` macro**

Change the macro at line 156:

```rust
#[macro_export]
macro_rules! launch_desktop {
    ($app:expr, $version:expr, $visible:expr) => {{
        $crate::init_logging();
        tracing::info!("Using custom desktop launcher configuration");
        let config = $crate::create_desktop_config($version, $visible);
        dioxus::LaunchBuilder::new().with_cfg(config).launch($app);
    }};
}
```

- [ ] **Step 3: Add `--minimized` parsing in `main.rs` and pass to launcher**

In `src/main.rs` `main()`, after the `--setup` block (line 478), add:

```rust
let start_visible = !args.contains(&"--minimized".to_string());
```

Change line 483 from:

```rust
launch_desktop!(App, APP_VERSION);
```

to:

```rust
launch_desktop!(App, APP_VERSION, start_visible);
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Manual test — verify `--minimized` hides window**

Run: `dx serve --desktop` and verify the app starts normally (visible). Then test with `--minimized` (add it to args in your IDE run config or build and run from terminal) — window should not appear, only tray icon.

- [ ] **Step 6: Commit**

```bash
git add gui_launcher/src/lib.rs src/main.rs
git commit -m "feat: add --minimized flag and start_visible to gui_launcher"
```

---

### Task 4: Add "Auto Start" toggle to GeneralSettings

**Files:**
- Modify: `src/components/features/general_settings.rs`

- [ ] **Step 1: Add the toggle to the component**

In `general_settings.rs`:

1. Add gated import at the top (after line 2):

```rust
#[cfg(target_os = "windows")]
use crate::backend::auto_start::{self, AutoStartError};
```

2. Add signal after the existing `auto_update_sig` signal (around line 33):

```rust
#[cfg(target_os = "windows")]
let mut auto_start_enabled = use_signal(|| false);
```

3. Add a `use_resource` to read the initial auto-start state. Place it after the existing `_settings_resource` (after line 61):

```rust
#[cfg(target_os = "windows")]
let _autostart_resource = use_resource(move || {
    let mut auto_start_enabled = auto_start_enabled;
    async move {
        let enabled = tokio::task::spawn_blocking(|| auto_start::is_enabled())
            .await
            .unwrap_or(false);
        auto_start_enabled.set(enabled);
    }
});
```

4. Add the toggle handler. Place it after `on_toggle_auto_logout` (around line 101):

```rust
#[cfg(target_os = "windows")]
let on_toggle_auto_start = move |_| {
    let toast = toast;
    spawn(async move {
        let result = match tokio::task::spawn_blocking(|| {
            if auto_start_enabled() {
                auto_start::disable()
            } else {
                auto_start::enable()
            }
        })
        .await
        {
            Ok(inner) => inner,
            Err(e) => Err(AutoStartError::RegistryError(format!("Task failed: {}", e))),
        };

        match result {
            Ok(()) => {
                auto_start_enabled.set(!auto_start_enabled());
            }
            Err(e) => {
                show_toast_error(format!("Auto-start error: {}", e), toast);
            }
        }
    });
};
```

5. Add the toggle UI in the RSX. Place it after the "Auto Update" toggle block (after line 161), before the "Auto Logout" toggle. Gate the entire block:

```rust
#[cfg(target_os = "windows")]
div { class: "flex flex-row justify-between mb-2",
    label { class: "label cursor-pointer",
        strong {
            span { class: "label-text", "Auto Start" }
        }
    }
    Toggle {
        checked: auto_start_enabled(),
        onchange: on_toggle_auto_start,
        size: ToggleSize::Large,
        color: ToggleColor::Success,
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Manual test — verify toggle works**

Run: `dx serve --desktop`. Navigate to Settings → General. Verify:
- Toggle reads initial state from registry
- Toggling ON writes to registry (check with `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v PwdManager`)
- Toggling OFF removes the value
- Error toast appears if something goes wrong

- [ ] **Step 4: Commit**

```bash
git add src/components/features/general_settings.rs
git commit -m "feat: add auto-start toggle to GeneralSettings"
```

---

### Task 5: Final verification

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: All tests PASS (including the 4 new auto_start tests)

- [ ] **Step 2: Run full manual test**

1. `dx serve --desktop`
2. Navigate to Settings → General
3. Enable "Auto Start" toggle
4. Verify registry key: `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v PwdManager`
5. Disable "Auto Start" toggle
6. Verify registry key removed
7. Enable again, close app, reopen — toggle should show ON
8. Disable from Task Manager → Startup tab, reopen app → toggle should show OFF
9. Re-enable from toggle → verify it re-enables in Task Manager

- [ ] **Step 3: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "fix: address review feedback from final verification"
```
