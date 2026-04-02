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
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

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

### New module: `vault_utils.rs`

CRUD operations:
- `create_vault(pool, user_id, name, description)` — INSERT
- `fetch_vaults_by_user(pool, user_id)` — SELECT all vaults for user
- `update_vault(pool, vault)` — UPSERT (name/description)
- `delete_vault(pool, vault_id)` — DELETE (cascade removes passwords)

### Query changes

All password queries currently filtered by `user_id` are updated to filter by `vault_id`:
- `fetch_all_by_user_id()` becomes `fetch_all_by_vault_id()`
- Stats queries filtered by `vault_id` of the active vault
- Import imports into the active vault
- Export exports from the selected vault

### Master password change migration pipeline

Unchanged logically. The pipeline decrypts ALL passwords for the user (across all vaults) and re-encrypts. Since it already operates on `user_id`, no structural changes are needed.

### Move / Clone operations

- **Move**: updates `vault_id` on selected `StoredPassword` records
- **Clone**: creates new `StoredPassword` records (same encrypted content, new nonces) in the target vault

## Frontend

### Routing

```
RouteWrapper
  NavBar
    /           -> LandingPage
    AuthWrapper
      /dashboard  -> Dashboard     (scoped to active vault)
      /my-vaults  -> MyVaults      <- NEW
      /logout     -> Logout
      /settings   -> Settings
    /login      -> Login
    /register   -> UpsertUser
```

### Navbar (logged in)

Left side: "Dashboard | My Vaults" as text links with vertical separator `|`. Same style as current navbar. Right side unchanged (avatar, logout).

### Dashboard

**Changes:**
- Vault selector Combobox in the controls bar (next to sort)
- StatsAside filtered by active vault
- Checkbox column in password table for multi-select
- Header checkbox for select-all
- Bulk action bar appears above table when rows are selected: "Move to..." and "Clone to..."
- DashboardMenu (import/export/delete-all) **removed**
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

### Move / Clone dialogs

**Move to vault:**
- Dialog shows count of selected passwords and their names
- Combobox to select target vault (excludes current vault)
- "+ New Vault" helper button: transforms combobox into inline input (name + optional description)
- "Create & Move" creates vault and executes move in one action
- "Back" button returns to combobox view

**Clone to vault:**
- Same layout as Move
- Clone creates new password records (same encrypted fields, new nonces) in target vault
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

- `NavBar` — add "My Vaults" link
- `Dashboard` — add vault Combobox, checkbox column, bulk action bar, remove DashboardMenu, add empty state
- `StatsAside` / `StatCard` — filter by vault_id
- `StoredRawPasswordsTable` — add checkbox column
- `StoredRawPasswordRow` — add checkbox, highlight selected
- `PaginationControls` — reuse for My Vaults card grid
- `StoredPasswordUpsertDialog` — pass vault_id when creating new password
- `AllStoredPasswordDeletionDialog` — scope to vault (move to My Vaults page)
- `init_queries.rs` — add vaults table, add vault_id to passwords table

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
