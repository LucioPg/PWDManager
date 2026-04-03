# Dashboard Improvements Design

**Date:** 2026-04-03
**Status:** Approved

## Overview

Improve the dashboard by unifying the move/clone dialogs, fixing the vault combobox display, and handling the empty-vault state.

## Changes

### 1. VaultActionDialog — Unified Move/Clone Dialog

**File:** `src/components/globals/dialogs/vault_action_dialog.rs` (new)
**Delete:** `src/components/globals/dialogs/move_to_vault_dialog.rs`, `src/components/globals/dialogs/clone_to_vault_dialog.rs`

**Enum:**
```rust
pub enum VaultAction {
    Move,
    Clone,
}
```

**Props:**
```rust
pub struct VaultActionDialogProps {
    pub open: Signal<bool>,
    pub action: VaultAction,
    pub selected_passwords: Vec<StoredRawPassword>,
    pub current_vault_id: i64,
    pub on_confirm: EventHandler<i64>,
    pub on_cancel: EventHandler<()>,
}
```

**Internal state:**
- `target_vault_id: Signal<Option<i64>>` — selected vault from combobox
- `show_inline_form: Signal<bool>` — toggle combobox <-> inline new vault form
- `new_vault_name: Signal<String>` — new vault name input
- `new_vault_desc: Signal<String>` — new vault description input
- `is_creating: Signal<bool>` — loading state during vault creation

**Behavior:**
- Vault options fetched via `fetch_vaults_by_user`
- Move filters out current vault from options; Clone includes all vaults
- Title and button labels derived from `action`:
  - Move: "Move N passwords to vault" / "Move" / "Create & Move"
  - Clone: "Clone N passwords to vault" / "Clone" / "Create & Clone"
- "+ New Vault" button toggles `show_inline_form`, replacing combobox with text inputs
- "Back" button returns to combobox view
- "Create & Move/Clone" calls `create_vault()` then `on_confirm(new_vault_id)`
- Dropdown must not trigger scrollbar in modal (same behavior as autologout combobox in settings)

**Mockup reference:** `.superpowers/brainstorm/502-1775153267/move-clone-v2.html`

### 2. Dashboard Vault Combobox + Empty State

**File:** `src/components/features/dashboard.rs` (modify)

**Vault combobox:**
- Add `selected_value` prop with current `active_vault_id` value
- Use dynamic `key` attribute on the Combobox to force re-mount on vault change (workaround for pwd-dioxus Combobox non-reactive `selected_value`)
- Set `disabled` when vault list is empty

**Empty vault state:**
- Combobox disabled with placeholder "Create a vault first"
- "Create New Password" button disabled when `active_vault_id` is None
- No password table shown when no vault is selected

## Decisions

1. **Combobox reactivity:** Workaround with dynamic `key` on the component, not modifying the pwd-dioxus library
2. **Dialog unification:** Single component with `VaultAction` enum (type-safe, no string-based dispatch)
3. **Inline vault creation:** Combobox replaced by form inline within the same dialog, matching mockup v2
4. **Scrollbar fix:** Investigate modal CSS interference, match autologout combobox behavior
