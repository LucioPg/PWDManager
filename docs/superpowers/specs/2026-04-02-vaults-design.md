# Vault System Design

**Date:** 2026-04-02
**Status:** Approved
**Approach:** Vault as separate entity (Approach A)

## Summary

Introduce a vault layer between users and passwords. Each user can have multiple vaults, and every password must belong to a vault. The dashboard becomes a view scoped to the active vault. A dedicated "My Vaults" page manages vault-level operations (create, edit, delete, import, export, delete-all). Passwords can be moved or cloned between vaults via multi-select bulk actions.

## Database Schema

### New table: `vaults`

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

Vault names are unique per user to avoid confusing duplicates.

### Modified table: `passwords`

Added column:

```sql
vault_id INTEGER NOT NULL,
FOREIGN KEY(vault_id) REFERENCES vaults(id) ON DELETE CASCADE
```

### Relationships

```
users 1:N vaults      (user_id -> users.id, CASCADE)
vaults 1:N passwords   (vault_id -> vaults.id, CASCADE)
users 1:N passwords    (user_id -> users.id, CASCADE)  -- unchanged
```

### Migration

No legacy migration code. The project is in beta — the database is dropped and recreated with the new schema.

## Backend

### New struct: `Vault` (in `pwd-types`)

```rust
#[derive(FromRow, Debug, Clone, SqlxTemplate)]
#[table("vaults")]
#[tp_upsert(by = "id")]
pub struct Vault {
    pub id: Option<i64>,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
}
```

### Modified structs

- `StoredPassword`: add `pub vault_id: i64`
- `StoredRawPassword`: add `pub vault_id: i64`
- `ExportData`: add `pub vault_id: i64`
- `ImportData`: add `pub vault_id: i64`

### Modified functions

The following functions must propagate `vault_id`:

- `decrypt_bulk_stored_data` — construct `StoredRawPassword` with `vault_id` from source `StoredPassword`
- `create_stored_data_records` — construct `StoredPassword` with `vault_id` from source `StoredRawPassword`
- `StoredPassword::new()` constructor — accept `vault_id` parameter
- `delete_all_user_stored_passwords` — remains unchanged (operates on `user_id`, used by migration pipeline)

### New module: `vault_utils.rs`

CRUD operations:
- `create_vault(pool, user_id, name, description)` — INSERT
- `fetch_vaults_by_user(pool, user_id)` — SELECT all vaults for user
- `update_vault(pool, vault)` — UPSERT (name/description)
- `delete_vault(pool, vault_id)` — DELETE (cascade removes passwords)

### New functions in `db_backend.rs`

- `delete_vault_passwords(pool, vault_id)` — vault-scoped delete-all (used by My Vaults page)
- `fetch_password_count_by_vault(pool, vault_id)` — returns count of passwords in a vault (for UI display)

### Query changes

All password queries currently filtered by `user_id` are updated to filter by `vault_id`:
- `fetch_all_by_user_id()` becomes `fetch_all_by_vault_id()`
- Stats queries filtered by `vault_id` of the active vault
- Import imports into the active vault
- Export exports from the selected vault

### Edge case: no vaults

When a user has no vaults:
- Import/Export actions are disabled (no vault to scope to)
- Dashboard shows the empty state with "Create your first Vault"
- New Password button is not available until a vault exists

### Master password change migration pipeline

Unchanged logically. The pipeline decrypts ALL passwords for the user (across all vaults) and re-encrypts. Since it operates on `user_id`, no structural changes are needed. The existing `delete_all_user_stored_passwords(pool, user_id)` remains as-is for the migration pipeline.

### Move / Clone operations

- **Move**: updates `vault_id` on selected `StoredPassword` records (no re-encryption needed)
- **Clone**: decrypts passwords from source vault, then **re-encrypts with new nonces** into the target vault. Copying ciphertext with different nonces would produce garbage on decryption — re-encryption is required.

## Frontend

### Routing

`MyVaults` is added inside the `AuthWrapper` layout, between `#[layout(AuthWrapper)]` and `#[end_layout(AuthWrapper)]`, after Dashboard and before Logout:

```
RouteWrapper
  NavBar
    /           -> LandingPage
    /login      -> Login
    /register   -> UpsertUser
    #[layout(AuthWrapper)]
      /dashboard  -> Dashboard     (scoped to active vault)
      /my-vaults  -> MyVaults      <- NEW
      /logout     -> Logout
      /settings   -> Settings
    #[end_layout(AuthWrapper)]
    /:..segments -> PageNotFound { segments }
```

### Navbar (logged in)

The current navbar renders "Dashboard" as a single `Link` with `navbar-brand` class. The structure is refactored to render two links side-by-side separated by a vertical divider `|`, both in the same visual style:

```
Dashboard | My Vaults          [avatar] Logout
```

"Dashboard" remains the primary brand link. "My Vaults" is added as a secondary link of equal visual weight. The separator is a styled `|` character.

### Active vault persistence

The active vault selection is stored in `user_settings` table:

```sql
ALTER TABLE user_settings ADD COLUMN active_vault_id INTEGER REFERENCES vaults(id);
```

- On login: `active_vault_id` is loaded from `user_settings`. If NULL, defaults to the first vault (by `created_at`).
- On vault selection change: `active_vault_id` is updated in `user_settings`.
- When the active vault is deleted: `active_vault_id` is reset to the first remaining vault, or NULL if no vaults left.
- When a new vault is created and no active vault exists: the new vault becomes active.

The `UserSettings` struct (and corresponding init/queries) is updated to include `active_vault_id: Option<i64>`.

### Dashboard

**Changes:**
- Vault selector Combobox in the controls bar (next to sort)
- StatsAside filtered by active vault
- Checkbox column in password table for multi-select
- Header checkbox for select-all
- Bulk action bar appears above table when rows are selected: "Move to..." and "Clone to..."
- DashboardMenu (import/export/delete-all) **removed** — moved to My Vaults
- "New Password" creates password in the active vault
- All layouts responsive

**Empty state (no vaults):**
- Centered display with lock icon, h3 "Create your first Vault", description text, and "+ New Vault" button
- On vault creation: redirect to dashboard with new vault selected

### My Vaults page

**Layout:**
- Header: "My Vaults" title with "+ New Vault" button
- Vault selector Combobox for actions target
- Action buttons: Import, Export, Delete All (scoped to selected vault)
- Card grid (3 columns, responsive): each card shows vault name, password count, Edit and Delete buttons
- PaginationControls (same component as dashboard)
- Card simplified: name + count only (description visible in edit dialog)

**Operations:**
- **Create vault**: dialog with name (required) + description (optional) inputs
- **Edit vault**: dialog pre-filled with current name and description
- **Delete vault**: confirmation dialog showing password count, "Delete vault and all X passwords?" — cascade delete

**Import/Export/Delete All — migration from DashboardMenu:**

The current `DashboardMenu` component provides import/export/delete-all functionality. This is migrated to the My Vaults page:

1. `DashboardMenu` component is removed from `Dashboard`
2. The import/export dialog state (`ExportData`, `ImportData`) is provided via `use_context_provider` in the `MyVaults` page instead of Dashboard
3. Progress dialogs (`ExportProgressDialog`, `ImportProgressDialog`) are rendered inside the My Vaults page RSX
4. `AllStoredPasswordDeletionDialog` is moved to My Vaults and uses the new `delete_vault_passwords(pool, vault_id)` function instead of `delete_all_user_stored_passwords`
5. Import/Export use `ExportData { vault_id, .. }` / `ImportData { vault_id, .. }` scoped to the selected vault

### Move / Clone dialogs

**Move to vault:**
- Dialog shows count of selected passwords and their names
- Combobox to select target vault (excludes current vault)
- "+ New Vault" helper button: transforms combobox into inline input (name + optional description)
- "Create & Move" creates vault and executes move in one action
- "Back" button returns to combobox view

**Clone to vault:**
- Same layout as Move
- Clone creates new password records by **decrypting from source and re-encrypting with new nonces** into target vault
- Target can be the same vault (duplicates within vault allowed)
- "Create & Clone" creates vault and executes clone in one action

### Multi-select state

- Selection state resets on vault change
- Selection state resets on pagination change
- "Clear selection" link in bulk action bar
- Individual row checkboxes toggle selection
- Header checkbox toggles all visible rows

## Components affected

### Existing components to modify

- `NavBar` — refactor to multi-link layout with separator, add "My Vaults" link
- `Dashboard` — add vault Combobox, checkbox column, bulk action bar, remove DashboardMenu, add empty state
- `StatsAside` / `StatCard` — filter by vault_id
- `StoredRawPasswordsTable` — add checkbox column
- `StoredRawPasswordRow` — add checkbox, highlight selected
- `PaginationControls` — reuse for My Vaults card grid
- `StoredPasswordUpsertDialog` — pass vault_id when creating new password
- `AllStoredPasswordDeletionDialog` — scope to vault via `delete_vault_passwords` (moved to My Vaults)
- `init_queries.rs` — add vaults table, add vault_id to passwords table, add active_vault_id to user_settings
- `DashboardMenu` — **removed** (functionality migrated to My Vaults)
- `password_utils.rs` — update `decrypt_bulk_stored_data` and `create_stored_data_records` to propagate vault_id
- `db_backend.rs` — add `delete_vault_passwords`, `fetch_password_count_by_vault`; update `StoredPassword::new()` to accept vault_id

### New components

- `MyVaults` page component
- `VaultCard` component
- `VaultCreateDialog` (reused in My Vaults and Move/Clone helper)
- `VaultEditDialog`
- `VaultDeleteDialog`
- `MoveToVaultDialog`
- `CloneToVaultDialog`
- `BulkActionBar` (shared between Move and Clone triggers)

### New dialogs needed

- `BaseModal` variants can be reused for all new dialogs
- Import/Export dialogs moved to My Vaults context but same components

## Responsive design

All layouts must be responsive:
- Dashboard: single column on mobile, StatsAside collapsible (already implemented)
- My Vaults card grid: 3 cols -> 2 cols -> 1 col on smaller screens
- Controls bar: wrap on smaller screens
- Bulk action bar: wrap on smaller screens
- Combobox and action buttons stack vertically on mobile
