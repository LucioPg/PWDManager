# Vault System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a vault layer between users and passwords, with full CRUD, move/clone bulk operations, and scoped dashboard/management views.

**Architecture:** New `vaults` table with FK to `users`. `passwords` table gains `vault_id` FK to `vaults`. Frontend adds `/my-vaults` route, vault selector combobox in dashboard, multi-select with move/clone bulk actions. Active vault persisted in `user_settings`.

**Tech Stack:** Rust, Dioxus 0.7, SQLite (SQLCipher), sqlx-template, pwd-types/pwd-crypto/pwd-dioxus (external git crates)

**Spec:** `docs/superpowers/specs/2026-04-02-vaults-design.md`

---

## File Structure

### External crate (pwd-types) — modify in its repo, use `[patch]` in Cargo.toml

- `src/stored.rs` — Add `vault_id` to `StoredPassword`, `StoredRawPassword`; add `Vault` struct; update `StoredPassword::new()`
- `src/lib.rs` — Export `Vault`

### Backend (`src/backend/`)

- `init_queries.rs` — Add `vaults` table, `vault_id` to `passwords`, `active_vault_id` to `user_settings`
- `vault_utils.rs` — **NEW** — Vault CRUD operations
- `db_backend.rs` — Add vault-scoped query functions, modify existing queries to filter by `vault_id`
- `password_utils.rs` — Propagate `vault_id` in `create_stored_data_records` and `decrypt_bulk_stored_data`
- `export_data.rs` — Add `vault_id` to `ExportData`
- `import_data.rs` — Add `vault_id` to `ImportData`
- `export.rs` — Update pipeline to filter by `vault_id`
- `import.rs` — Update pipeline to use `vault_id`
- `settings_types.rs` — Add `active_vault_id` to `UserSettings`

### Frontend (`src/`)

- `main.rs` — Add `MyVaults` route inside `AuthWrapper`
- `components/globals/navbar.rs` — Add "My Vaults" link with separator
- `components/globals/auth_wrapper.rs` — Load active vault from settings
- `components/globals/table/component.rs` — Add checkbox column to `StoredRawPasswordsTable`
- `components/globals/table/table_row.rs` — Add checkbox to `StoredRawPasswordRow`
- `components/features/dashboard.rs` — Vault combobox, empty state, remove DashboardMenu, bulk actions
- `components/features/dashboard_menu.rs` — **DELETE** (functionality moved to My Vaults)
- `components/features/my_vaults.rs` — **NEW** — My Vaults page
- `components/features/my_vaults/vault_card.rs` — **NEW** — Vault card component
- `components/globals/dialogs/` — New dialogs: vault create/edit/delete, move, clone

---

## Phase 1: External Crate (pwd-types)

### Task 1: Add Vault struct and vault_id fields in pwd-types

**Prerequisite:** Clone pwd-types repo locally. Add `[patch]` section in main project's `Cargo.toml` pointing to local path.

- [x] **Step 1: Add `[patch]` to Cargo.toml for local development** ✅ DONE (not committed — workaround)

In `Cargo.toml`, add at the bottom:

```toml
[patch.'https://github.com/LucioPg/pwd-types']
pwd-types = { path = "../pwd-types" }
```

Adjust path to point to the local pwd-types checkout.

- [x] **Step 2: Add `Vault` struct to pwd-types `src/stored.rs`** ✅ DONE

After the `StoredRawPassword` impl block, add:

```rust
/// Vault per raggruppare password.
#[derive(FromRow, Debug, Clone, SqlxTemplate, PartialEq)]
#[table("vaults")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct Vault {
    pub id: Option<i64>,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
}
```

- [x] **Step 3: Add `vault_id` to `StoredPassword`** ✅ DONE

In `StoredPassword` struct, add field after `user_id`:

```rust
pub vault_id: i64,
```

- [x] **Step 4: Update `StoredPassword::new()` to accept `vault_id`** ✅ DONE

Add `vault_id: i64` parameter after `user_id`. Include it in the struct construction:

```rust
pub fn new(
    id: Option<i64>,
    user_id: i64,
    vault_id: i64,
    name: String,
    // ... rest unchanged
) -> Self {
    // ... existing code ...
    StoredPassword {
        id,
        user_id,
        vault_id,
        name,
        // ... rest unchanged
    }
}
```

- [x] **Step 5: Add `vault_id` to `StoredRawPassword`** ✅ DONE

Add field after `user_id`:

```rust
pub vault_id: i64,
```

Update `StoredRawPassword::new()` to initialize `vault_id: 0`.

**IMPORTANT:** `StoredRawPassword` is also constructed via **struct literals** (not `::new()`) in:
- `password_utils.rs:521` (`decrypt_bulk_stored_data`)
- `export_types.rs` (`ExportablePassword::to_stored_raw`)

All struct literal sites must add `vault_id` field. Use grep `StoredRawPassword {` to find them all.

Update `Debug` impl to include `.field("vault_id", &self.vault_id)`.

- [x] **Step 6: Export `Vault` from `src/lib.rs`** ✅ DONE

Change:
```rust
pub use stored::{UserAuth, StoredPassword, StoredRawPassword};
```
To:
```rust
pub use stored::{UserAuth, StoredPassword, StoredRawPassword, Vault};
```

- [x] **Step 7: Verify compilation**

Run: `cargo check`
Expected: Compiles (some downstream errors expected due to new field)

- [x] **Step 8: Commit in pwd-types repo** ✅ DONE (commit 011f180)

```bash
git add src/stored.rs src/lib.rs
git commit -m "feat: add Vault struct and vault_id to StoredPassword/StoredRawPassword"
```

---

## Important Notes for Implementers

### Context Provider Scoping
Dioxus `use_context_provider` registers context per component tree. Dashboard and MyVaults are sibling routes under `AuthWrapper`. Each provides its own `PaginationState` independently — child components see only their parent's context. This works correctly.

### Export/Import Pipeline Context Hierarchy
`ExportProgressChn` and `ImportProgressChn` (in `src/components/features/export_progress.rs` and `import_progress.rs`) consume `Signal<ExportData>`/`Signal<ImportData>` via `use_context`. These components **must** be rendered as descendants of the `use_context_provider` call. When migrating from DashboardMenu to MyVaults, keep these component files in place and render them from MyVaults's RSX tree.

### StoredRawPassword Construction
`StoredRawPassword` is constructed via **struct literals** (not `::new()`) in `decrypt_bulk_stored_data` (password_utils.rs:521) and `ExportablePassword::to_stored_raw` (export_types.rs). All struct literal construction sites must include the new `vault_id` field.

### Migration Pipeline Functions Kept Unchanged
`fetch_all_stored_passwords_for_user` and `delete_all_user_stored_passwords` are **kept unchanged** — they operate on `user_id` and are used exclusively by the master password change migration pipeline. Raw SQL in `fetch_passwords_paginated` and `fetch_all_passwords_for_user_with_filter` must still include `vault_id` in their SELECT columns (for `StoredPassword` struct mapping via `query_as`).

### UserSettings Persistence Pattern
There is no `upsert_user_settings` function. Settings are upserted directly via `UserSettings::upsert_by_id(&settings, pool)` — see `general_settings.rs:186` for the existing pattern. Use the same pattern for `active_vault_id` persistence.

---

## Phase 2: Database Schema & Backend Foundation

### Task 2: Update database schema in init_queries.rs

**Files:**
- Modify: `src/backend/init_queries.rs:56-103`

- [ ] **Step 1: Add `vaults` table**

Before the `passwords` CREATE TABLE (before line 56), add:

```sql
CREATE TABLE IF NOT EXISTS vaults (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, name)
);
```

- [ ] **Step 2: Add `vault_id` to `passwords` table**

After `user_id INTEGER NOT NULL,` in the passwords CREATE TABLE, add:

```sql
    vault_id INTEGER NOT NULL,
    FOREIGN KEY(vault_id) REFERENCES vaults(id) ON DELETE CASCADE,
```

- [ ] **Step 3: Add `active_vault_id` to `user_settings` table**

After `auto_logout_settings TEXT DEFAULT 'TenMinutes',` in the user_settings CREATE TABLE, add:

```sql
    active_vault_id INTEGER REFERENCES vaults(id),
```

Note: No NOT NULL — nullable so existing users without vaults don't break.

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles (downstream call-site errors expected for vault_id)

- [ ] **Step 5: Commit**

```bash
git add src/backend/init_queries.rs
git commit -m "feat: add vaults table, vault_id to passwords, active_vault_id to user_settings"
```

### Task 3: Add vault_id to UserSettings struct

**Files:**
- Modify: `src/backend/settings_types.rs:26-34`

- [ ] **Step 1: Add `active_vault_id` field to `UserSettings`**

After `auto_logout_settings` field:

```rust
pub active_vault_id: Option<i64>,
```

Note: `Option<i64>` because a new user has no vault yet.

- [ ] **Step 2: Verify compilation**

Run: `cargo check`

- [ ] **Step 3: Commit**

```bash
git add src/backend/settings_types.rs
git commit -m "feat: add active_vault_id to UserSettings"
```

### Task 4: Create vault_utils.rs — Vault CRUD

**Files:**
- Create: `src/backend/vault_utils.rs`
- Modify: `src/backend/mod.rs` — add `pub mod vault_utils;`

- [ ] **Step 1: Write failing tests**

Create `src/backend/vault_utils.rs`:

```rust
//! CRUD operations per i vault.
//!
//! Fornisce funzioni per creare, leggere, aggiornare e eliminare vault.

use sqlx::SqlitePool;
use pwd_types::Vault;
use custom_errors::DBError;

/// Crea un nuovo vault.
pub async fn create_vault(
    pool: &SqlitePool,
    user_id: i64,
    name: String,
    description: Option<String>,
) -> Result<Vault, DBError> {
    let vault = Vault {
        id: None,
        user_id,
        name,
        description,
        created_at: None,
    };
    Vault::upsert_by_id(&vault, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Failed to create vault: {}", e)))
}

/// Recupera tutti i vault di un utente.
pub async fn fetch_vaults_by_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<Vault>, DBError> {
    Vault::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch vaults: {}", e)))?;

    // Sort by created_at ascending for deterministic "first vault" selection
    let mut vaults = result;
    vaults.sort_by_key(|v| v.created_at.clone().unwrap_or_default());
    Ok(vaults)
}

/// Aggiorna un vault esistente (nome/descrizione).
pub async fn update_vault(pool: &SqlitePool, vault: Vault) -> Result<(), DBError> {
    Vault::upsert_by_id(&vault, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Failed to update vault: {}", e)))
}

/// Elimina un vault (cascade elimina le password associate).
pub async fn delete_vault(pool: &SqlitePool, vault_id: i64) -> Result<(), DBError> {
    sqlx::query("DELETE FROM vaults WHERE id = ?")
        .bind(vault_id)
        .execute(pool)
        .await
        .map_err(|e| DBError::new_password_delete_error(format!("Failed to delete vault: {}", e)))?;
    Ok(())
}

/// Recupera il conteggio delle password in un vault.
pub async fn fetch_password_count_by_vault(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<u64, DBError> {
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM passwords WHERE vault_id = ?"
    )
    .bind(vault_id)
    .fetch_one(pool)
    .await
    .map_err(|e| DBError::new_list_error(format!("Failed to count vault passwords: {}", e)))?;
    Ok(result.0 as u64)
}
```

- [ ] **Step 2: Add module to mod.rs**

In `src/backend/mod.rs`, add:
```rust
pub mod vault_utils;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src/backend/vault_utils.rs src/backend/mod.rs
git commit -m "feat: add vault_utils.rs with vault CRUD operations"
```

### Task 5a: Add vault-scoped query functions in db_backend.rs

**Files:**
- Modify: `src/backend/db_backend.rs`

**Note:** Existing `fetch_all_stored_passwords_for_user` and `delete_all_user_stored_passwords` are **kept unchanged** — they operate on `user_id` and are used by the master password migration pipeline.

- [ ] **Step 1: Add `fetch_all_stored_passwords_for_vault`**

New function (vault-scoped version of `fetch_all_stored_passwords_for_user`):

```rust
pub async fn fetch_all_stored_passwords_for_vault(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!("Fetching all passwords for vault_id: {}", vault_id);
    StoredPassword::builder_select()
        .vault_id(&vault_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch passwords: {}", e)))
}
```

- [ ] **Step 2: Add `fetch_passwords_paginated_for_vault`**

New function (vault-scoped version of `fetch_passwords_paginated`). Same logic but replace `WHERE user_id = ?` with `WHERE vault_id = ?` and bind `vault_id`. Include `vault_id` in SELECT column list.

- [ ] **Step 3: Add `fetch_all_passwords_for_vault_with_filter`**

New function (vault-scoped version of `fetch_all_passwords_for_user_with_filter`). Replace `WHERE user_id = ?` with `WHERE vault_id = ?`. Include `vault_id` in SELECT.

- [ ] **Step 4: Add `fetch_password_stats_for_vault`**

New function (vault-scoped version of `fetch_password_stats`). Replace `WHERE user_id = ?` with `WHERE vault_id = ?`.

- [ ] **Step 5: Add `delete_vault_passwords`**

```rust
pub async fn delete_vault_passwords(
    pool: &SqlitePool,
    vault_id: i64,
) -> Result<(), DBError> {
    debug!("Deleting all passwords in vault_id: {}", vault_id);
    sqlx::query("DELETE FROM passwords WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_password_delete_error(format!("Failed to delete vault passwords: {}", e))
        })?;
    Ok(())
}
```

- [ ] **Step 6: Add `vault_id` to existing raw SQL SELECT columns**

In `fetch_passwords_paginated` and `fetch_all_passwords_for_user_with_filter`, add `vault_id` to the SELECT column list (required for `query_as::<_, StoredPassword>()` mapping):

```sql
SELECT id, user_id, vault_id, name, username, username_nonce, ...
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check`

- [ ] **Step 8: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: vault-scoped query functions in db_backend"
```

### Task 5b: Propagate vault_id in password_utils.rs

**Files:**
- Modify: `src/backend/password_utils.rs`

- [ ] **Step 1: Update `create_stored_data_records` (line 279)**

Pass `vault_id` from `StoredRawPassword` to `StoredPassword::new()` call (line 342).

- [ ] **Step 2: Update `decrypt_bulk_stored_data` (line 448)**

In the struct literal construction of `StoredRawPassword` (line 521), add `vault_id: sp.vault_id`.

- [ ] **Step 3: Update `StoredPassword::new()` call sites**

Search all `StoredPassword::new(` calls in the codebase. Add `vault_id` parameter after `user_id`. For migration pipeline, vault_id comes from source `StoredRawPassword`.

- [ ] **Step 4: Verify compilation**

Run: `cargo check`

- [ ] **Step 5: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat: propagate vault_id in password encrypt/decrypt pipeline"
```

### Task 5c: Update ExportData, ImportData, and pipeline components

**Files:**
- Modify: `src/backend/export_data.rs`
- Modify: `src/backend/import_data.rs`
- Modify: `src/backend/export_types.rs` — `ExportablePassword::to_stored_raw()`
- Modify: `src/backend/export.rs`
- Modify: `src/backend/import.rs`
- Modify: `src/components/features/export_progress.rs` — pass `vault_id` to pipeline
- Modify: `src/components/features/import_progress.rs` — pass `vault_id` to pipeline

- [ ] **Step 1: Add `vault_id` to ExportData and ImportData**

In `export_data.rs`:
```rust
pub struct ExportData {
    pub user_id: i64,
    pub vault_id: i64,
    pub output_path: PathBuf,
    pub format: ExportFormat,
}
```

In `import_data.rs`:
```rust
pub struct ImportData {
    pub user_id: i64,
    pub vault_id: i64,
    pub input_path: PathBuf,
    pub format: ExportFormat,
}
```

Update both constructors to accept `vault_id: i64`.

- [ ] **Step 2: Update `ExportablePassword::to_stored_raw()` in `export_types.rs`**

Add `vault_id: i64` parameter. Set `vault_id` in the returned `StoredRawPassword` struct literal.

- [ ] **Step 3: Update export pipeline in `export.rs`**

Use `fetch_all_stored_passwords_for_vault(context.vault_id)` instead of `fetch_all_stored_passwords_for_user(context.user_id)`.

- [ ] **Step 4: Update import pipeline in `import.rs`**

Use `vault_id` from `ImportData`. Update `to_stored_raw(user_id, vault_id)` calls.

- [ ] **Step 5: Update `ExportProgressChn` in `export_progress.rs`**

Read `vault_id` from `ExportData` context and pass it to the pipeline function.

- [ ] **Step 6: Update `ImportProgressChn` in `import_progress.rs`**

Read `vault_id` from `ImportData` context and pass it to the pipeline function.

- [ ] **Step 7: Update all `ExportData::new()` and `ImportData::new()` call sites**

In `dashboard_menu.rs` (6 export + 3 import calls), add `vault_id` parameter.

- [ ] **Step 8: Verify compilation**

Run: `cargo check`

- [ ] **Step 9: Commit**

```bash
git add src/backend/export_data.rs src/backend/import_data.rs src/backend/export_types.rs src/backend/export.rs src/backend/import.rs src/components/features/export_progress.rs src/components/features/import_progress.rs
git commit -m "feat: vault_id in export/import pipeline and progress components"
```

---

## Phase 3: Frontend Infrastructure

### Task 6: Add /my-vaults route

**Files:**
- Modify: `src/main.rs:511-532`

- [ ] **Step 1: Add route variant**

In the `Route` enum, add between `Dashboard` and `Logout`:

```rust
#[layout(AuthWrapper)]
#[route("/dashboard")]
Dashboard,
#[route("/my-vaults")]
MyVaults,
#[route("/logout")]
Logout,
```

- [ ] **Step 2: Add placeholder component**

Before the `Route` enum, add:

```rust
#[component]
fn MyVaults() -> Element {
    rsx! {
        h1 { "My Vaults — placeholder" }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add /my-vaults route placeholder"
```

### Task 7: Update Navbar with My Vaults link

**Files:**
- Modify: `src/components/globals/navbar.rs:17`

- [ ] **Step 1: Refactor navbar left side**

Replace the single Dashboard brand link with two links separated by `|`:

```rust
div { class: "navbar-left",
    Link { to: Route::Dashboard, class: "navbar-brand",
        h3 { class: "navbar-brand-text", "Dashboard" }
    }
    div { class: "pwd-navbar-separator", "|" }
    Link { to: Route::MyVaults, class: "navbar-brand",
        h3 { class: "navbar-brand-text", "My Vaults" }
    }
}
```

- [ ] **Step 2: Add CSS for separator**

In the global styles, add:
```css
.pwd-navbar-separator {
    margin: 0 8px;
    color: var(--color-base-300);
    font-weight: 300;
}
```

- [ ] **Step 3: Verify in browser**

Run app, login, verify both links visible and separator renders correctly.

- [ ] **Step 4: Commit**

```bash
git add src/components/globals/navbar.rs assets/
git commit -m "feat: add My Vaults link to navbar with separator"
```

### Task 8: Active vault state and persistence

**Files:**
- Modify: `src/components/globals/auth_wrapper.rs:106-130`
- Modify: `src/components/features/dashboard.rs` (consumer)

- [ ] **Step 1: Add active vault signal in auth_wrapper or a shared context**

In `auth_wrapper.rs`, after loading settings, if `active_vault_id` is set, provide it as a Dioxus context. If None, fetch first vault and set it.

Use `use_signal` or `use_context_provider` for a shared `ActiveVaultState`:

```rust
#[derive(Clone, Copy, Default)]
pub struct ActiveVaultState(pub Signal<Option<i64>>);
```

- [ ] **Step 2: On login, load active vault from settings**

In the settings loading `use_resource`, after `fetch_user_settings`, if `active_vault_id` is Some, set the signal. If None, fetch vaults and default to the first one.

- [ ] **Step 3: On vault change, persist to user_settings**

When the vault combobox changes, persist `active_vault_id` using `UserSettings::upsert_by_id(&settings, pool)` — same pattern as theme/auto_update in `general_settings.rs:186`.

- [ ] **Step 4: Verify**

Run app, login, verify active vault is loaded and persisted.

- [ ] **Step 5: Commit**

```bash
git add src/components/globals/auth_wrapper.rs
git commit -m "feat: active vault state with persistence in user_settings"
```

---

## Phase 4: Dashboard Modifications

### Task 9: Dashboard vault combobox and empty state

**Files:**
- Modify: `src/components/features/dashboard.rs`

- [ ] **Step 1: Add vault combobox to controls bar**

Use the existing `Combobox` component. Fetch vaults via `use_resource` calling `fetch_vaults_by_user`. Render a `Combobox::<Option<i64>>` with vault options. On change, update the `ActiveVaultState` signal and re-trigger password fetch.

- [ ] **Step 2: Add empty state when no vaults**

When vault list is empty (length == 0), render the centered empty state instead of the normal dashboard:

```rust
if vaults.is_empty() {
    rsx! {
        div { class: "pwd-empty-state",
            div { class: "pwd-empty-state-icon",
                // lock SVG icon
            }
            h3 { "Create your first Vault" }
            p { class: "pwd-empty-state-subtitle", "A vault is where your passwords live. Create one to get started." }
            VaultCreateDialog { on_created: move |_| /* navigate to dashboard */ }
        }
    }
}
```

- [ ] **Step 3: Update password fetch to use vault_id**

Change `use_resource` for passwords to use `fetch_all_stored_passwords_for_vault` with the active vault_id instead of user_id.

- [ ] **Step 4: Update stats fetch to use vault_id**

Change `use_resource` for stats to use `fetch_password_stats_for_vault` with active vault_id.

- [ ] **Step 5: Remove DashboardMenu import and rendering**

Remove `DashboardMenu` component import and its `rsx!` usage from dashboard.

- [ ] **Step 6: Verify**

Run app, login with no vaults → see empty state. Create vault → dashboard shows with vault selected. Switch vault → passwords and stats update.

- [ ] **Step 7: Commit**

```bash
git add src/components/features/dashboard.rs
git commit -m "feat: dashboard vault combobox, empty state, remove DashboardMenu"
```

### Task 10: Multi-select checkboxes in password table

**Files:**
- Modify: `src/components/globals/table/component.rs`
- Modify: `src/components/globals/table/table_row.rs`
- Modify: `src/components/features/dashboard.rs`

- [ ] **Step 1: Add checkbox props to StoredRawPasswordsTable**

Update props to accept selection state:

```rust
#[component]
pub fn StoredRawPasswordsTable(
    data: Option<Vec<StoredRawPassword>>,
    selected_ids: Signal<HashSet<i64>>,
    on_select: EventHandler<(i64, bool)>,
    on_select_all: EventHandler<bool>,
) -> Element
```

- [ ] **Step 2: Add header checkbox**

In the table header row, add a checkbox that toggles all visible rows:

```rust
th {
    input {
        r#type: "checkbox",
        checked: all_visible_selected,
        onchange: move |_| on_select_all.call(!all_visible_selected),
    }
}
```

- [ ] **Step 3: Add checkbox to each row**

Pass `selected_ids` and `on_select` to each `StoredRawPasswordRow`. Add a checkbox cell as the first `<td>`.

- [ ] **Step 4: Add selection state in dashboard**

In dashboard.rs, add:

```rust
let mut selected_ids: Signal<HashSet<i64>> = use_signal(HashSet::new);
```

Reset on vault change and page change.

- [ ] **Step 5: Commit**

```bash
git add src/components/globals/table/component.rs src/components/globals/table/table_row.rs src/components/features/dashboard.rs
git commit -m "feat: multi-select checkboxes in password table"
```

### Task 11: Bulk action bar (Move/Clone)

**Files:**
- Modify: `src/components/features/dashboard.rs`
- Create: `src/components/features/bulk_action_bar.rs`

- [ ] **Step 1: Create BulkActionBar component**

```rust
#[component]
pub fn BulkActionBar(
    count: usize,
    on_move: EventHandler<()>,
    on_clone: EventHandler<()>,
    on_clear: EventHandler<()>,
) -> Element
```

Renders the blue bar with "N selected", "Move to...", "Clone to...", "Clear selection".

- [ ] **Step 2: Add to dashboard**

In dashboard.rs, show `BulkActionBar` when `selected_ids.read().len() > 0`, positioned above the table.

- [ ] **Step 3: Wire up Move and Clone to open respective dialogs**

`on_move` opens `MoveToVaultDialog`, `on_clone` opens `CloneToVaultDialog`.

- [ ] **Step 4: Commit**

```bash
git add src/components/features/bulk_action_bar.rs src/components/features/dashboard.rs
git commit -m "feat: bulk action bar with Move and Clone triggers"
```

---

## Phase 5: My Vaults Page

### Task 12: My Vaults page structure with card grid

**Files:**
- Modify: `src/main.rs` (update MyVaults component)
- Create: `src/components/features/my_vaults.rs`
- Create: `src/components/features/my_vaults/mod.rs`
- Create: `src/components/features/my_vaults/vault_card.rs`

- [ ] **Step 1: Create VaultCard component**

```rust
#[component]
pub fn VaultCard(
    vault: Vault,
    password_count: u64,
    on_edit: EventHandler<Vault>,
    on_delete: EventHandler<Vault>,
) -> Element
```

Renders a card with vault name, password count, Edit and Delete buttons.

- [ ] **Step 2: Create MyVaults page component**

Structure:
- Header with "My Vaults" title and "+ New Vault" button
- Vault selector Combobox (for actions target)
- Action buttons: Import, Export, Delete All
- Card grid (3 columns responsive) using `VaultCard`
- `PaginationControls` for vault pagination
- Dialog states for vault create/edit/delete

- [ ] **Step 3: Migrate import/export/delete-all from DashboardMenu**

Copy the relevant logic from `dashboard_menu.rs`:
- Export handlers (JSON/CSV/XML) — scoped to selected vault
- Import handlers (JSON/CSV/XML) — scoped to selected vault
- Delete All handler — uses `delete_vault_passwords` instead of `delete_all_user_stored_passwords`
- Progress dialog rendering

- [ ] **Step 4: Wire up context providers**

Move `ExportData` and `ImportData` context providers from DashboardMenu to MyVaults. The `ExportProgressChn` and `ImportProgressChn` components (in `src/components/features/export_progress.rs` and `import_progress.rs`) must be rendered inside MyVaults's RSX tree as descendants of the context provider.

- [ ] **Step 5: Delete dashboard_menu.rs**

Remove the file. In `src/components/features/mod.rs`, remove both `mod dashboard_menu;` (line 6) and `pub use dashboard_menu::*;` (line 22).

- [ ] **Step 6: Verify**

Run app, navigate to My Vaults, create/edit/delete vaults, test import/export/delete-all scoped to a vault.

- [ ] **Step 7: Commit**

```bash
git add src/components/features/my_vaults/ src/components/features/mod.rs src/components/features/dashboard_menu.rs src/main.rs
git commit -m "feat: My Vaults page with card grid, CRUD, and scoped import/export/delete"
```

### Task 13: Vault CRUD dialogs

**Files:**
- Create: `src/components/globals/dialogs/vault_create_dialog.rs`
- Create: `src/components/globals/dialogs/vault_edit_dialog.rs`
- Create: `src/components/globals/dialogs/vault_delete_dialog.rs`
- Modify: `src/components/globals/dialogs/mod.rs`

- [ ] **Step 1: Create VaultCreateDialog**

Using `BaseModal` with `ModalVariant::Small`:
- Name input (required)
- Description input (optional)
- Cancel / Create buttons
- On create: calls `create_vault`, then emits `on_created` event

- [ ] **Step 2: Create VaultEditDialog**

Same layout as create, pre-filled with current values. On save: calls `update_vault`.

- [ ] **Step 3: Create VaultDeleteDialog**

Using `BaseModal` with `ModalVariant::Middle`:
- Warning icon
- "Delete vault 'X'?" with password count
- "This will permanently delete the vault and all N passwords."
- Cancel / Delete buttons
- On confirm: calls `delete_vault`

- [ ] **Step 4: Register dialogs in mod.rs**

Add `pub mod` for all three dialog files.

- [ ] **Step 5: Commit**

```bash
git add src/components/globals/dialogs/
git commit -m "feat: vault create, edit, and delete dialogs"
```

---

## Phase 6: Move/Clone Operations

### Task 14: Move to Vault dialog

**Files:**
- Create: `src/components/globals/dialogs/move_to_vault_dialog.rs`
- Modify: `src/backend/db_backend.rs` — add `move_passwords_to_vault` function

- [ ] **Step 1: Add `move_passwords_to_vault` backend function**

```rust
pub async fn move_passwords_to_vault(
    pool: &SqlitePool,
    password_ids: Vec<i64>,
    target_vault_id: i64,
) -> Result<(), DBError> {
    for id in password_ids {
        sqlx::query("UPDATE passwords SET vault_id = ? WHERE id = ?")
            .bind(target_vault_id)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| DBError::new_password_save_error(format!("Failed to move password: {}", e)))?;
    }
    Ok(())
}
```

- [ ] **Step 2: Create MoveToVaultDialog component**

Props:
```rust
pub fn MoveToVaultDialog(
    open: Signal<bool>,
    selected_passwords: Vec<StoredRawPassword>,
    current_vault_id: i64,
    vaults: Vec<Vault>,
    on_confirm: EventHandler<i64>,  // target vault_id
)
```

Renders:
- Header: "Move N passwords to vault"
- Subtitle: comma-separated list of password names
- Combobox for target vault (exclude current vault)
- "+ New Vault" helper button → transforms to inline name/description inputs
- "Create & Move" / "Back" / "Cancel" / "Move" buttons

- [ ] **Step 3: Wire up in dashboard**

On "Move to..." click in BulkActionBar, open MoveToVaultDialog. On confirm, call `move_passwords_to_vault`, refresh passwords, clear selection.

- [ ] **Step 4: Commit**

```bash
git add src/components/globals/dialogs/move_to_vault_dialog.rs src/backend/db_backend.rs src/components/features/dashboard.rs
git commit -m "feat: Move to Vault dialog with bulk password transfer"
```

### Task 15: Clone to Vault dialog

**Files:**
- Create: `src/components/globals/dialogs/clone_to_vault_dialog.rs`
- Modify: `src/backend/password_utils.rs` — add clone function

- [ ] **Step 1: Add `clone_passwords_to_vault` function**

This function must **decrypt** source passwords and **re-encrypt** with new nonces:

```rust
pub async fn clone_passwords_to_vault(
    pool: &SqlitePool,
    user_auth: UserAuth,
    password_ids: Vec<i64>,
    target_vault_id: i64,
) -> Result<(), DBError> {
    // 1. Fetch source passwords by IDs
    // 2. Decrypt using decrypt_bulk_stored_data
    // 3. Update vault_id on each StoredRawPassword
    // 4. Re-encrypt using create_stored_data_records (generates new nonces)
    // 5. Upsert cloned records (with id: None so they get new IDs)
    Ok(())
}
```

- [ ] **Step 2: Create CloneToVaultDialog component**

Same layout as MoveToVaultDialog but:
- Target can be the same vault (duplicates allowed)
- Button text: "Clone" / "Create & Clone"

- [ ] **Step 3: Wire up in dashboard**

On "Clone to..." click in BulkActionBar, open CloneToVaultDialog. On confirm, call `clone_passwords_to_vault`, refresh passwords, clear selection.

- [ ] **Step 4: Commit**

```bash
git add src/components/globals/dialogs/clone_to_vault_dialog.rs src/backend/password_utils.rs src/components/features/dashboard.rs
git commit -m "feat: Clone to Vault dialog with re-encryption"
```

---

## Phase 7: Responsive & Polish

### Task 16: Responsive CSS for new components

**Files:**
- Modify: `assets/input_main.css` or appropriate stylesheet

- [ ] **Step 1: My Vaults card grid responsive**

```css
.pwd-vault-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
}

@media (max-width: 1024px) {
    .pwd-vault-grid { grid-template-columns: repeat(2, 1fr); }
}

@media (max-width: 640px) {
    .pwd-vault-grid { grid-template-columns: 1fr; }
}
```

- [ ] **Step 2: Dashboard controls bar responsive**

```css
.pwd-dashboard-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
}
```

- [ ] **Step 3: Bulk action bar responsive**

```css
.pwd-bulk-action-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
}
```

- [ ] **Step 4: Test responsive behavior**

Resize browser window to verify all breakpoints work correctly.

- [ ] **Step 5: Commit**

```bash
git add assets/
git commit -m "feat: responsive CSS for vault components"
```

### Task 17: Registration flow — require vault creation

**Files:**
- Modify: `src/components/features/upsert_user.rs` (registration UI success handler)

- [ ] **Step 1: After successful registration, auto-create a "Default" vault**

In the registration success handler in `upsert_user.rs`, after `register_user_with_settings` returns successfully, call `create_vault(pool, user_id, "Default".to_string(), None)` and then update `user_settings.active_vault_id` via `UserSettings::upsert_by_id(&settings, pool)`.

- [ ] **Step 2: Verify**

Register a new user, verify "Default" vault is created and active.

- [ ] **Step 3: Commit**

```bash
git add src/
git commit -m "feat: auto-create Default vault on user registration"
```
