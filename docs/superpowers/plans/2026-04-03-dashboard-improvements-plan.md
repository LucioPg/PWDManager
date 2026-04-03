# Dashboard Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify move/clone dialogs, fix vault combobox display, and handle empty-vault state on dashboard.

**Architecture:** Replace two separate MoveToVaultDialog/CloneToVaultDialog with a single VaultActionDialog using a VaultAction enum. Fix dashboard vault combobox to show active vault name and disable controls when no vault exists. Investigate and fix combobox scrollbar issue inside modals.

**Tech Stack:** Dioxus 0.7, pwd-dioxus Combobox, sqlx/sqlite, Rust

**Spec:** `docs/superpowers/specs/2026-04-03-dashboard-improvements-design.md`

---

## Implementation Notes

> These notes come from the plan review. Follow them during implementation to avoid common pitfalls.

1. **`selected_ids` prop intentionally dropped**: The old MoveToVaultDialog/CloneToVaultDialog accepted `selected_ids: Vec<i64>` as a prop but never used it internally (dead code). The unified VaultActionDialog drops it. The IDs are collected from the parent's `selected_ids` signal in the `on_confirm` callback instead. Do not re-add this prop.

2. **Remove `pool_for_clone` in dashboard.rs**: When wiring the unified dialog (Task 1, Step 3), the old `pool_for_clone` declaration (line ~66) is no longer needed since the unified `on_confirm` callback uses only `pool_for_move`. Remove it along with the old dialog signals.

3. **Verify `pwd-dioxus` Combobox props before Task 2**: The plan assumes `Combobox` accepts `selected_value` and `disabled` props. These are used by the autologout combobox in `general_settings.rs` (line 250-256), so they should exist. Run `cargo check` early to confirm. If they don't exist, the pwd-dioxus crate needs to be updated — this contradicts Decision #1 in the spec (no library changes). Flag this immediately if it happens.

4. **Task 4 (scrollbar) is deliberately investigative**: The steps are reproduce → investigate → fix → verify. The refactoring in Task 1 may already resolve the issue. If the scrollbar problem no longer reproduces, skip Task 4.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/components/globals/dialogs/vault_action_dialog.rs` | Create | Unified move/clone dialog with inline vault creation |
| `src/components/globals/dialogs/move_to_vault_dialog.rs` | Delete | Replaced by vault_action_dialog |
| `src/components/globals/dialogs/clone_to_vault_dialog.rs` | Delete | Replaced by vault_action_dialog |
| `src/components/globals/dialogs/mod.rs` | Modify | Update module exports |
| `src/components/features/dashboard.rs` | Modify | Wire VaultActionDialog, fix vault combobox, disable controls |

---

### Task 1: Create VaultActionDialog component

**Files:**
- Create: `src/components/globals/dialogs/vault_action_dialog.rs`
- Reference: `src/components/globals/dialogs/move_to_vault_dialog.rs` (structure to merge)
- Reference: `src/components/globals/dialogs/clone_to_vault_dialog.rs` (structure to merge)
- Reference: `src/components/globals/dialogs/vault_create.rs` (inline vault creation pattern)
- Reference: `.superpowers/brainstorm/502-1775153267/move-clone-v2.html` (mockup)

- [ ] **Step 1: Create the VaultAction enum and dialog component structure**

Create `src/components/globals/dialogs/vault_action_dialog.rs` with:

```rust
// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use super::base_modal::ModalVariant;
use crate::auth::AuthState;
use crate::backend::vault_utils::{create_vault, fetch_vaults_by_user};
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant, show_toast_error, use_toast};
use dioxus::prelude::*;
use pwd_dioxus::Combobox;
use pwd_types::StoredRawPassword;
use sqlx::SqlitePool;

const MAX_DISPLAYED_NAMES: usize = 5;

#[derive(Clone, Copy, PartialEq)]
pub enum VaultAction {
    Move,
    Clone,
}

#[component]
pub fn VaultActionDialog(
    open: Signal<bool>,
    action: VaultAction,
    selected_passwords: Vec<StoredRawPassword>,
    current_vault_id: i64,
    on_confirm: EventHandler<i64>,
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let pool = use_context::<SqlitePool>();
    let user_state = use_context::<AuthState>();
    let user_id = user_state.get_user_id();
    let toast = use_toast();
    let mut open_clone = open;
    let mut target_vault_id: Signal<Option<i64>> = use_signal(|| None);
    let mut show_inline_form: Signal<bool> = use_signal(|| false);
    let mut new_vault_name: Signal<String> = use_signal(String::new);
    let mut new_vault_desc: Signal<Option<String>> = use_signal(|| None);
    let mut is_creating: Signal<bool> = use_signal(|| false);

    // Reset all state when dialog opens
    use_effect(move || {
        if open() {
            target_vault_id.set(None);
            show_inline_form.set(false);
            new_vault_name.set(String::new());
            new_vault_desc.set(None);
            is_creating.set(false);
        }
    });

    // Fetch vault list
    let vaults_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id;
        async move {
            if user_id == -1 {
                return Vec::new();
            }
            fetch_vaults_by_user(&pool, user_id)
                .await
                .unwrap_or_default()
        }
    });

    // Build combobox options
    // Move: filter out current vault. Clone: include all vaults.
    let vault_options = use_memo(move || {
        let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
        let opts: Vec<(&'static str, Option<i64>)> = vaults
            .iter()
            .filter(|v| {
                if action == VaultAction::Move {
                    v.id.is_some_and(|id| id != current_vault_id)
                } else {
                    true
                }
            })
            .map(|v| {
                let name = Box::leak(v.name.clone().into_boxed_str()) as &'static str;
                (name, v.id)
            })
            .collect();
        opts
    });

    // Derived labels from action
    let action_label = match action {
        VaultAction::Move => "Move",
        VaultAction::Clone => "Clone",
    };
    let count = selected_passwords.len();
    #[rustfmt::skip]
    let title = format!("{action_label} {count} {} to vault", if count == 1 { "password" } else { "passwords" });
    let confirm_btn_text = action_label.to_string();
    let create_and_confirm_text = format!("Create & {action_label}");

    // Displayed password names (max 5, then "and X more...")
    let display_names: Vec<String> = selected_passwords
        .iter()
        .take(MAX_DISPLAYED_NAMES)
        .map(|p| p.name.clone())
        .collect();
    let remaining = count.saturating_sub(MAX_DISPLAYED_NAMES);

    let on_action = move |_| {
        let Some(target_id) = target_vault_id() else {
            return;
        };
        let mut open = open_clone;
        let on_confirm = on_confirm;
        spawn(async move {
            on_confirm.call(target_id);
            open.set(false);
        });
    };

    let on_create_and_action = move |_| {
        let name_val = new_vault_name();
        if name_val.trim().is_empty() {
            return;
        }
        is_creating.set(true);
        let pool = pool.clone();
        let user_id = user_id;
        let desc = new_vault_desc();
        let mut open = open_clone;
        let on_confirm = on_confirm;
        let toast = toast;
        spawn(async move {
            match create_vault(&pool, user_id, name_val.trim().to_string(), desc).await {
                Ok(vault) => {
                    let new_vault_id = vault.id.unwrap_or(0);
                    on_confirm.call(new_vault_id);
                    open.set(false);
                }
                Err(e) => {
                    show_toast_error(format!("Failed to create vault: {}", e), toast);
                }
            }
        });
    };

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,
            class: "futuristic",

            // Close button "X"
            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "\u{2715}"
            }

            // Title
            h3 { class: "font-bold text-lg mb-4", "{title}" }

            // Selected passwords list
            if !display_names.is_empty() {
                div { class: "mb-4",
                    p { class: "text-sm opacity-70 mb-1", "Selected passwords:" }
                    ul { class: "list-disc list-inside text-sm space-y-1",
                        for name in display_names.iter() {
                            li { "{name}" }
                        }
                        if remaining > 0 {
                            li { class: "opacity-60", "and {remaining} more..." }
                        }
                    }
                }
            }

            // Target vault selector
            div { class: "form-control w-full mb-6",
                label { class: "label",
                    span { class: "label-text", "Target vault" }
                }
                if !show_inline_form() {
                    // Combobox + "+ New Vault" button row
                    div { class: "flex gap-2 items-center",
                        div { class: "flex-1",
                            Combobox::<i64> {
                                options: vault_options(),
                                placeholder: "Select vault...".to_string(),
                                on_change: move |v| {
                                    target_vault_id.set(v);
                                },
                            }
                        }
                        button {
                            class: "btn btn-sm btn-outline btn-primary",
                            r#type: "button",
                            onclick: move |_| {
                                show_inline_form.set(true);
                            },
                            "+ New Vault"
                        }
                    }
                } else {
                    // Inline new vault form
                    div {
                        div { class: "flex gap-2 items-center mb-2",
                            div { class: "flex-1",
                                input {
                                    class: "input input-bordered input-sm w-full",
                                    r#type: "text",
                                    placeholder: "New vault name...",
                                    value: "{new_vault_name}",
                                    oninput: move |e| new_vault_name.set(e.value()),
                                }
                            }
                        }
                        input {
                            class: "input input-bordered input-sm w-full",
                            r#type: "text",
                            placeholder: "Description (optional)",
                            value: "{new_vault_desc().unwrap_or_default()}",
                            oninput: move |e| new_vault_desc.set(Some(e.value())),
                        }
                    }
                }
            }

            // Action buttons
            div { class: "modal-action",
                if show_inline_form() {
                    // Inline form buttons
                    ActionButton {
                        text: "\u{2190} Back".to_string(),
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            show_inline_form.set(false);
                        },
                    }
                    ActionButton {
                        text: "Cancel".to_string(),
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            on_cancel.call(());
                            open_clone.set(false);
                        },
                    }
                    ActionButton {
                        text: create_and_confirm_text.clone(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        disabled: is_creating() || new_vault_name().trim().is_empty(),
                        on_click: on_create_and_action,
                    }
                } else {
                    // Normal combobox buttons
                    ActionButton {
                        text: "Cancel".to_string(),
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            on_cancel.call(());
                            open_clone.set(false);
                        },
                    }
                    ActionButton {
                        text: confirm_btn_text.clone(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: on_action,
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register the new module in mod.rs and update exports**

In `src/components/globals/dialogs/mod.rs`:

1. Add `mod vault_action_dialog;`
2. Add `pub use vault_action_dialog::*;`
3. Remove `mod move_to_vault_dialog;` and `mod clone_to_vault_dialog;`
4. Remove `pub use move_to_vault_dialog::*;` and `pub use clone_to_vault_dialog::*;`

Result should look like:
```rust
pub mod base_modal;
mod database_reset;
mod export_progress;
mod export_warning;
mod import_progress;
mod import_warning;
mod migration_progress;
mod migration_warning;
mod recovery_key_input;
mod recovery_key_regenerate;
mod recovery_key_setup;
mod stored_all_passwords_deletion;
mod stored_password_deletion;
mod stored_password_show;
mod stored_password_upsert;
mod vault_action_dialog;
mod vault_create;
mod vault_delete;
mod vault_edit;
pub mod user_deletion;

pub use base_modal::*;
pub use database_reset::*;
pub use recovery_key_input::*;
pub use recovery_key_regenerate::*;
pub use recovery_key_setup::*;
pub use stored_password_deletion::*;
pub use user_deletion::*;

// ide-only serve per avere highlight mentre si lavora su elementi non ancora completati.
pub use export_progress::*;
pub use export_warning::*;
pub use import_progress::*;
pub use import_warning::*;
pub use migration_progress::*;
pub use migration_warning::*;
pub use stored_all_passwords_deletion::*;
pub use stored_password_show::*;
pub use stored_password_upsert::*;
pub use vault_action_dialog::*;
pub use vault_create::*;
pub use vault_delete::*;
pub use vault_edit::*;
```

- [ ] **Step 3: Update dashboard.rs to use VaultActionDialog**

In `src/components/features/dashboard.rs`:

1. **Imports**: Replace `CloneToVaultDialog, MoveToVaultDialog` with `VaultActionDialog` and add `VaultAction`:
```rust
use crate::components::{
    BulkActionBar, StoredPasswordDeletionDialog,
    StoredPasswordShowDialog, StoredPasswordUpsertDialog, StoredRawPasswordsTable,
    VaultAction, VaultActionDialog,
    show_toast_error, use_toast,
};
```

2. **Dialog states**: Merge `move_dialog_open` and `clone_dialog_open` into a single signal with a vault action type. Add two new signals:
```rust
let mut vault_action_dialog_open = use_signal(|| false);
let mut current_vault_action = use_signal(|| VaultAction::Move);
```
Remove the old `move_dialog_open` and `clone_dialog_open` signals.

3. **BulkActionBar callbacks**: Update to set the action and open the unified dialog:
```rust
BulkActionBar {
    count,
    on_move: move |_| {
        current_vault_action.set(VaultAction::Move);
        vault_action_dialog_open.set(true);
    },
    on_clone: move |_| {
        current_vault_action.set(VaultAction::Clone);
        vault_action_dialog_open.set(true);
    },
    on_clear: move |_| {
        selected_ids.set(HashSet::new());
    },
}
```

4. **Replace both dialog usages** (lines ~499-565) with a single VaultActionDialog:
```rust
VaultActionDialog {
    open: vault_action_dialog_open,
    action: current_vault_action(),
    selected_passwords: all_passwords()
        .into_iter()
        .filter(|p| p.id.is_some_and(|id| selected_ids.read().contains(&id)))
        .collect(),
    current_vault_id: active_vault_id().unwrap_or(0),
    on_confirm: move |target_vault_id| {
        let pool = pool_for_move.clone();
        let user_id = user_id;
        let ids: Vec<i64> = selected_ids.read().iter().cloned().collect();
        let mut dialog_open = vault_action_dialog_open;
        let mut sorted_resource = sorted_passwords_resource;
        let mut stats_res = stats_data;
        let action = current_vault_action();
        spawn(async move {
            let result = if action == VaultAction::Move {
                move_passwords_to_vault(&pool, ids, target_vault_id).await
            } else {
                clone_passwords_to_vault(&pool, user_id, ids, target_vault_id).await
            };
            match result {
                Ok(()) => {
                    selected_ids.set(HashSet::new());
                    dialog_open.set(false);
                    sorted_resource.restart();
                    stats_res.restart();
                }
                Err(e) => {
                    show_toast_error(format!("Failed: {}", e), toast);
                }
            }
        });
    },
    on_cancel: move |_| {
        vault_action_dialog_open.set(false);
    },
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors (unused pool clones warnings are acceptable)

- [ ] **Step 5: Commit**

```bash
git add src/components/globals/dialogs/vault_action_dialog.rs src/components/globals/dialogs/mod.rs src/components/features/dashboard.rs
git rm src/components/globals/dialogs/move_to_vault_dialog.rs src/components/globals/dialogs/clone_to_vault_dialog.rs
git commit -m "refactor: unify move/clone dialogs into VaultActionDialog"
```

---

### Task 2: Fix dashboard vault combobox display

**Files:**
- Modify: `src/components/features/dashboard.rs:365-372`

- [ ] **Step 1: Add selected_value and key to vault combobox**

In `src/components/features/dashboard.rs`, replace the vault combobox (lines ~365-372):

```rust
// Vault selector Combobox
Combobox::<i64> {
    options: vault_options(),
    placeholder: "Select Vault".to_string(),
    size: ComboboxSize::Medium,
    on_change: move |v| {
        active_vault_id.set(v);
    },
}
```

With:
```rust
// Vault selector Combobox
// key forces re-mount when vault changes (workaround for non-reactive selected_value)
{
    let vault_key = active_vault_id().unwrap_or(-1);
    let selected = active_vault_id();
    rsx! {
        Combobox::<i64> {
            key: "{vault_key}",
            options: vault_options(),
            placeholder: "Select Vault".to_string(),
            size: ComboboxSize::Medium,
            selected_value: selected,
            on_change: move |v| {
                active_vault_id.set(v);
            },
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/components/features/dashboard.rs
git commit -m "fix: show active vault name in dashboard combobox"
```

---

### Task 3: Handle empty vault state on dashboard

**Files:**
- Modify: `src/components/features/dashboard.rs:365-382`

- [ ] **Step 1: Disable vault combobox and "New Password" button when no vaults**

The empty vault state already exists (lines ~409-436) with a "Create your first Vault" message. Now add:

1. Disable vault combobox when `vaults.is_empty()`:

In the vault combobox block, wrap with a vault availability check:
```rust
// Vault selector Combobox
{
    let vaults = vaults_resource.read().as_ref().cloned().unwrap_or_default();
    let is_empty = vaults.is_empty();
    let vault_key = active_vault_id().unwrap_or(-1);
    let selected = active_vault_id();
    rsx! {
        Combobox::<i64> {
            key: "{vault_key}",
            options: vault_options(),
            placeholder: if is_empty { "Create a vault first".to_string() } else { "Select Vault".to_string() },
            size: ComboboxSize::Medium,
            selected_value: selected,
            disabled: is_empty,
            on_change: move |v| {
                active_vault_id.set(v);
            },
        }
    }
}
```

2. Disable "New Password" button when `active_vault_id` is None:

Replace the "New Password" button (lines ~374-383):
```rust
button {
    class: if active_vault_id().is_none() { "btn btn-success btn-disabled" } else { "btn btn-success" },
    r#type: "button",
    disabled: active_vault_id().is_none(),
    onclick: move |_| {
        stored_password_dialog_state.current_stored_raw_password.set(None);
        stored_password_dialog_state.is_open.set(true);
    },
    "New Password"
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/components/features/dashboard.rs
git commit -m "feat: disable controls when no vaults exist on dashboard"
```

---

### Task 4: Fix combobox scrollbar issue inside modal

**Files:**
- Modify: `src/components/globals/dialogs/vault_action_dialog.rs` (if needed)
- Investigate: CSS in `assets/input_main.css`

- [ ] **Step 1: Reproduce the scrollbar issue**

Open the app, navigate to dashboard, select some passwords, click "Move to..." to open the dialog. Click the vault combobox dropdown. Observe if the modal scrolls.

If the issue no longer reproduces (it may have been fixed by the BaseModal refactor), skip to Step 4.

- [ ] **Step 2: Investigate the root cause**

The Combobox uses `position: fixed` with `getBoundingClientRect()` for its dropdown. The BaseModal likely has `overflow: hidden` or `overflow: auto` on its content area which may conflict.

Check:
1. `assets/input_main.css` for modal overflow rules
2. The BaseModal component for its CSS classes
3. The invisible overlay (z-index 9998) that the Combobox adds — it may be causing scroll in the modal

- [ ] **Step 3: Apply fix**

Likely fixes (apply the minimal one that works):
- If the modal content has `overflow: auto`, the full-screen overlay may trigger a scroll event. Ensure the overlay has `overflow: hidden` and `pointer-events: none` except on the dropdown.
- If the BaseModal uses `overflow: hidden`, ensure the dropdown is not a DOM child of the modal content (portal pattern), or add CSS `overflow: visible` on the dropdown container.

- [ ] **Step 4: Verify the fix**

Open the dialog, click the combobox — the dropdown should appear without scrolling the modal.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "fix: prevent scrollbar when opening combobox inside modal"
```
