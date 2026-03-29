# Guide to sqlx-template

How to use `sqlx-template` in this project for automatic CRUD generation via derive macros.

## What it is

`sqlx-template` generates database query functions at compile time from struct definitions. Instead of writing raw SQL for every operation, you derive the macro on a struct and get upsert, select, delete, and builder methods generated automatically.

## Dependencies

```toml
[dependencies]
sqlx = { version = "0.8.6", default-features = false, features = ["runtime-tokio", "sqlite", "macros"] }
sqlx-template = "0.2.1"
futures = "0.3"  # required by SqlxTemplate for builder pattern
```

## Basic Syntax

### Struct Definition

All structs that use sqlx-template in this project use the `SqlxTemplate` derive (not `SqliteTemplate`):

```rust
use sqlx::FromRow;
use sqlx_template::SqlxTemplate;

#[derive(FromRow, Debug, SqlxTemplate)]
#[db("sqlite")]
#[table("user_settings")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct UserSettings {
    pub id: Option<i64>,
    pub user_id: i64,
    pub theme: Theme,
    pub auto_update: AutoUpdate,
    pub auto_logout_settings: Option<AutoLogoutSettings>,
}
```

### Required Attributes

| Attribute | Purpose |
|---|---|
| `#[db("sqlite")]` | Target database engine |
| `#[table("name")]` | Database table name |
| `#[tp_upsert(by = "id")]` | Generate upsert method, keyed on `id` |
| `#[tp_select_builder]` | Generate the builder pattern for dynamic queries |

## Generated Methods

### Upsert

The `#[tp_upsert(by = "id")]` attribute generates a static upsert method:

```rust
impl UserSettings {
    pub async fn upsert_by_id(
        &self,
        pool: impl SqlxExecutor<'_, Database = Sqlite>
    ) -> Result<(), sqlx::Error>;
}
```

The method is **static** (called on the type, not on an instance). The first argument is `&self` (the struct value to insert/update).

Behavior depends on the key field:
- `id: None` -- INSERT a new row
- `id: Some(n)` -- INSERT OR REPLACE the existing row

```rust
// Insert new record
let settings = UserSettings { id: None, user_id: 1, ..Default::default() };
UserSettings::upsert_by_id(&settings, pool).await?;

// Update existing record
let settings = UserSettings { id: Some(42), user_id: 1, ..Default::default() };
UserSettings::upsert_by_id(&settings, pool).await?;
```

### Builder (tp_select_builder)

The `#[tp_select_builder]` attribute generates a builder for dynamic SELECT queries:

```rust
impl UserSettings {
    pub fn builder_select() -> UserSettingsSelectBuilder;
}
```

The builder provides methods for each struct field:

| Method pattern | Description |
|---|---|
| `.field(&val)` | Exact match filter |
| `.field_like(&pattern)` | LIKE filter (strings) |
| `.field_start_with(&prefix)` | Starts with (strings) |
| `.field_end_with(&suffix)` | Ends with (strings) |
| `.field_gt(&val)` | Greater than (numbers) |
| `.field_gte(&val)` | Greater or equal |
| `.field_lt(&val)` | Less than |
| `.field_lte(&val)` | Less or equal |
| `.order_by_field_asc()` | Sort ascending |
| `.order_by_field_desc()` | Sort descending |

Execution methods:

| Method | Return |
|---|---|
| `.find_all(pool)` | `Vec<T>` |
| `.find_one(pool)` | `Option<T>` |

Each builder method returns `Result<&mut Self, sqlx::Error>`, so you chain with `?` for error propagation.

## Usage in This Project

### Example: Fetch by user_id

From `src/backend/db_backend.rs`:

```rust
pub async fn fetch_user_settings(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<UserSettings>, DBError> {
    let user_settings = UserSettings::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .find_one(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch user settings: {}", e)))?;

    Ok(user_settings)
}
```

Generated SQL: `SELECT * FROM user_settings WHERE user_id = ?`

### Example: Fetch and order

From `src/backend/db_backend.rs` (fetching all stored passwords for a user):

```rust
pub async fn fetch_all_stored_passwords_for_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<StoredPassword>, DBError> {
    let builder = StoredPassword::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .order_by_created_at_desc()
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;

    builder
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch passwords: {}", e)))
}
```

Generated SQL: `SELECT * FROM passwords WHERE user_id = ? ORDER BY created_at DESC`

### Example: Batch upsert in a transaction

From `src/backend/db_backend.rs`:

```rust
pub async fn upsert_stored_passwords_batch(
    pool: &SqlitePool,
    passwords: Vec<StoredPassword>,
) -> Result<(), DBError> {
    let mut tx = pool.begin().await?;

    for stored_password in &passwords {
        if stored_password.password.expose_secret().is_empty()
            || stored_password.url.expose_secret().is_empty()
        {
            return Err(DBError::new_password_save_error(
                "Password and url cannot be empty".into(),
            ));
        }

        StoredPassword::upsert_by_id(stored_password, &mut *tx)
            .await
            .map_err(|e| DBError::new_password_save_error(format!("Upsert failed: {}", e)))?;
    }

    tx.commit().await?;
    Ok(())
}
```

Note the `&mut *tx` pattern: `sqlx-template` accepts any `SqlxExecutor`, and `&mut SqliteTransaction` implements it.

## Structs Using sqlx-template

| Struct | Table | File |
|---|---|---|
| `UserSettings` | `user_settings` | `src/backend/settings_types.rs` |
| `DicewareGenerationSettings` | `diceware_generation_settings` | `src/backend/settings_types.rs` |
| `StoredPassword` | `passwords` | `pwd-types` crate (external) |
| `PasswordGeneratorConfig` | `passwords_generation_settings` | `pwd-types` crate (external) |

## Custom Types

Fields with custom types (e.g., `PasswordScore`, `Theme`, `AutoUpdate`) must implement `sqlx::Type`, `sqlx::Encode`, and `sqlx::Decode` for SQLite. These implementations are defined alongside the types in `src/backend/settings_types.rs`.

For example, `Theme` is stored as a TEXT column (`"Light"` or `"Dark"`) with manual encode/decode impls.

## Deprecated Syntax

Do not use these -- they belong to an older version of sqlx-template:

| Deprecated | Current |
|---|---|
| `#[sqlx(table_name = "...")]` | `#[table("...")]` |
| `#[sqlx(upsert_by = "...")]` | `#[tp_upsert(by = "...")]` |
| `SqliteTemplate` derive | `SqlxTemplate` derive |

## Troubleshooting

### "no method named `upsert`"

You are calling an instance method. All sqlx-template methods are static:

```rust
// Wrong
stored_password.upsert(&pool)

// Correct
StoredPassword::upsert_by_id(&stored_password, &pool)
```

### "cannot find attribute `db`"

The `#[db("sqlite")]` attribute is required when using `SqlxTemplate`. It is part of the correct syntax, not an error.

### Custom type not mapped

If a field uses a type that is not a primitive SQL type, you need:

```rust
impl Type<Sqlite> for MyType { ... }
impl Encode<'q, Sqlite> for MyType { ... }
impl Decode<'r, Sqlite> for MyType { ... }
```

See `src/backend/settings_types.rs` for working examples (`Theme`, `AutoUpdate`, `AutoLogoutSettings`).
