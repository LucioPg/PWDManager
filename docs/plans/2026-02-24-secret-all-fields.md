# Secret Fields Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Tutti i campi sensibili (`location`, `password`, `notes`) diventano `SecretString` per protezione end-to-end. UI rivelata solo tramite interazione utente.

**Architecture:**
- `StoredRawPassword`: `location` e `notes` diventano `SecretString` (come `password`)
- `TableRow`: `location` e `password` usano `SecretDisplay` con toggle eye
- `Burger Tooltip`: `notes` rivelate solo dopo click su "reveal" button

**Tech Stack:** Rust, Dioxus 0.7, secrecy crate, SQLite

---

## Task 1: Update StoredRawPassword Struct

**Files:**
- Modify: `src/backend/password_types_helper.rs:251-291`

**Step 1: Update struct definition**

⚠️ **IMPORTANTE**: SecretString NON implementa `Debug`. Rimuovere il derive o implementarlo manualmente.

```rust
// Rimuovere #[derive(Debug, Clone)] e implementare Debug manualmente
#[derive(Clone)]
pub struct StoredRawPassword {
    pub id: Option<i64>,
    #[allow(unused)]
    pub user_id: i64,
    pub location: SecretString,           // String → SecretString
    pub password: SecretString,           // already SecretString
    pub notes: Option<SecretString>,      // Option<String> → Option<SecretString>
    pub score: Option<PasswordScore>,
    pub created_at: Option<String>,
}

// Implementare Debug manualmente per NON esporre i segreti
impl std::fmt::Debug for StoredRawPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredRawPassword")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("location", &"***SECRET***")
            .field("password", &"***SECRET***")
            .field("notes", &self.notes.as_ref().map(|_| "***SECRET***"))
            .field("score", &self.score)
            .field("created_at", &self.created_at)
            .finish()
    }
}
```

**Step 2: Update constructor**

```rust
impl StoredRawPassword {
    pub fn new() -> Self {
        StoredRawPassword {
            id: None,
            user_id: 0,
            location: SecretString::new("".into()),
            password: "".to_string().into(),
            notes: None,
            score: None,
            created_at: None,
        }
    }
```

**Step 3: Update get_form_fields**

```rust
    #[allow(dead_code)]
    pub fn get_form_fields(
        &self,
    ) -> (
        i64,
        SecretString,        // location
        SecretString,        // password
        Option<SecretString>, // notes
        Option<PasswordScore>,
    ) {
        (
            self.id.unwrap(),
            self.location.clone(),
            self.password.clone(),
            self.notes.clone(),
            self.score.clone(),
        )
    }
}
```

**Step 4: Update PartialEq implementation**

```rust
impl PartialEq for StoredRawPassword {
    fn eq(&self, other: &Self) -> bool {
        match (&self.id, &other.id) {
            (Some(id1), Some(id2)) => {
                id1 == id2
                    && self.location.expose_secret() == other.location.expose_secret()
            },
            (None, None) => true,
            _ => false,
        }
    }
}
```

**Step 5: Verify imports**

Ensure `ExposeSecret` and `SecretString` are imported:
```rust
use secrecy::{ExposeSecret, SecretBox, SecretString};
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Errors about type mismatches (will fix in next tasks)

**Step 7: Commit**

```bash
git add src/backend/password_types_helper.rs
git commit -m "feat(types): change location and notes to SecretString in StoredRawPassword"
```

---

## Task 2: Update decrypt_bulk_stored_data

**Files:**
- Modify: `src/backend/password_utils.rs:405-465`

**Step 1: Wrap location and notes in SecretString**

```rust
Ok(StoredRawPassword {
    id: sp.id,
    user_id: user_auth.id,
    location: SecretString::new(location.into()),      // wrap in SecretString
    password: SecretString::new(password.into()),      // already done
    notes: notes.map(|n| SecretString::new(n.into())), // wrap in SecretString
    score: Some(sp.score),
    created_at: sp.created_at,
})
```

**Step 2: Verify imports**

Ensure `SecretString` is imported:
```rust
use secrecy::{ExposeSecret, SecretBox, SecretString};
```

**Step 3: Verify compilation**

Run: `cargo check`

**Step 4: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): wrap location and notes in SecretString during decryption"
```

---

## Task 3: Update Tests

**Files:**
- Modify: `src/backend/password_utils_tests.rs`

**Step 1: Update test_decrypt_location_and_notes_roundtrip**

```rust
#[tokio::test]
async fn test_decrypt_location_and_notes_roundtrip() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_rt", "MasterPass123!").await;

    let raw_password = SecretString::new("MyPassword".into());
    let location = "MySecretService".to_string();
    let notes = Some("My secret notes".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt");

    let decrypted = crate::backend::password_utils::get_stored_raw_passwords(&pool, user_id)
        .await
        .expect("Failed to decrypt");

    assert_eq!(decrypted.len(), 1);
    assert_eq!(decrypted[0].location.expose_secret(), location);
    assert_eq!(decrypted[0].notes.as_ref().map(|n| n.expose_secret().as_str()), notes.as_deref());
    assert_eq!(decrypted[0].password.expose_secret(), raw_password.expose_secret());
}
```

**Step 2: Update test_location_and_notes_are_encrypted**

No changes needed - tests StoredPassword (encrypted), not StoredRawPassword.

**Step 3: Update any other test using StoredRawPassword.location directly**

Search and update:
```bash
grep -n "\.location" src/backend/password_utils_tests.rs
```

Use `.location.expose_secret()` for comparisons.

**Step 4: Run tests**

Run: `cargo test password_utils_tests`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/backend/password_utils_tests.rs
git commit -m "test: update tests for SecretString location and notes"
```

---

## Task 4: Rename PasswordDisplay to SecretDisplay

**Files:**
- Create: `src/components/globals/secret_display/mod.rs`
- Delete: `src/components/globals/password_display/` (entire directory)
- Modify: `src/components/globals/mod.rs`
- Modify: `assets/input_main.css` (rename CSS classes)

**Step 1: Create new directory structure**

Create `src/components/globals/secret_display/mod.rs`:

```rust
use crate::components::globals::svgs::{ClipboardIcon, EyeIcon, EyeOffIcon};
use crate::components::globals::form_field::FormSecret;
use dioxus::prelude::*;
use secrecy::ExposeSecret;

/// Componente SecretDisplay - visualizza dati sensibili con toggle visibility
///
/// Usato per: password, location, e altri campi sensibili
#[component]
pub fn SecretDisplay(
    /// Il valore segreto da visualizzare (FormSecret wrappa SecretString)
    secret: FormSecret,
    /// Classe CSS aggiuntiva per il container (opzionale)
    #[props(default)]
    class: Option<String>,
    /// Larghezza massima del contenitore (default: 200px)
    #[props(default = "200px".to_string())]
    max_width: String,
    /// Callback quando si clicca sull'icona clipboard
    #[props(default)]
    on_copy: Option<EventHandler<()>>,
) -> Element {
    let mut visible = use_signal(|| false);

    let value_len = secret.expose_secret().len();
    let display_value = if value_len == 0 {
        String::new()
    } else if visible() {
        secret.expose_secret().to_string()
    } else {
        "•".repeat(value_len)
    };

    rsx! {
        div { class: "secret-display-wrapper {class.clone().unwrap_or_default()}",
            input {
                class: "pwd-secret-display font-mono",
                r#type: if visible() { "text" } else { "password" },
                value: "{display_value}",
                readonly: true,
                title: if visible() {
                    Some(secret.expose_secret().to_string())
                } else {
                    None
                },
                style: "max-width: {max_width}",
            }

            div { class: "secret-display-actions flex gap-1",
                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    onclick: move |_| visible.set(!visible()),
                    aria_label: if visible() { "Nascondi" } else { "Mostra" },
                    if visible() {
                        EyeOffIcon { class: Some("text-current".to_string()) }
                    } else {
                        EyeIcon { class: Some("text-current".to_string()) }
                    }
                }

                button {
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    disabled: on_copy.is_none(),
                    aria_label: "Copia",
                    ClipboardIcon { class: Some("text-current".to_string()) }
                }
            }
        }
    }
}
```

**Step 2: Update globals/mod.rs**

```rust
// Remove:
pub use password_display::PasswordDisplay;

// Add:
pub mod secret_display;
pub use secret_display::SecretDisplay;
```

**Step 3: Update CSS classes in input_main.css**

Search and replace:
- `.password-display-wrapper` → `.secret-display-wrapper`
- `.pwd-password-display` → `.pwd-secret-display`
- `.password-display-actions` → `.secret-display-actions`

**Step 4: Remove old directory**

Delete: `src/components/globals/password_display/`

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Errors about PasswordDisplay not found (will fix in next task)

**Step 6: Commit**

```bash
git add src/components/globals/ assets/input_main.css
git commit -m "refactor(ui): rename PasswordDisplay to SecretDisplay with CSS updates"
```

---

## Task 5: Create SecretNotesTooltip Component

**Files:**
- Create: `src/components/globals/secret_notes_tooltip/mod.rs`
- Modify: `src/components/globals/mod.rs`

**Step 1: Create component for revealing notes in tooltip**

```rust
use crate::components::globals::svgs::EyeIcon;
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};

/// Componente per mostrare notes segrete nel tooltip del burger button.
/// Le notes sono nascoste di default e rivelate solo dopo click.
#[component]
pub fn SecretNotesTooltip(
    /// Le notes segrete (opzionali)
    notes: Option<SecretString>,
    /// Data di creazione (non segreta)
    created_at: Option<String>,
) -> Element {
    let mut notes_visible = use_signal(|| false);

    rsx! {
        div { class: "dropdown-content mockup-code bg-base-200 shadow-lg rounded-lg p-3 min-w-[200px] max-w-[280px]",
            // Notes section
            if let Some(notes) = &notes {
                div { class: "mb-3",
                    h4 { class: "font-bold text-xs mb-1 text-gray-600", "Notes" }

                    // Toggle reveal button
                    div { class: "flex items-center gap-2",
                        if notes_visible() {
                            p { class: "text-xs text-gray-700 break-words flex-1",
                                "{notes.expose_secret()}"
                            }
                        } else {
                            p { class: "text-xs text-gray-500 italic flex-1",
                                "•••••••• (click to reveal)"
                            }
                        }

                        button {
                            class: "btn btn-ghost btn-xs",
                            r#type: "button",
                            onclick: move |_| notes_visible.set(!notes_visible()),
                            aria_label: if notes_visible() { "Nascondi notes" } else { "Mostra notes" },
                            EyeIcon { class: Some("w-4 h-4".to_string()) }
                        }
                    }
                }
            }

            // Created at section (not secret)
            if let Some(created_at) = &created_at {
                div {
                    h4 { class: "font-bold text-xs mb-1 text-gray-600", "Created" }
                    p { class: "text-xs text-gray-700", "{created_at}" }
                }
            }

            // Show placeholder if no info available
            if notes.is_none() && created_at.is_none() {
                p { class: "text-xs text-gray-500 italic", "No additional info" }
            }
        }
    }
}
```

**Step 2: Update globals/mod.rs**

```rust
pub mod secret_notes_tooltip;
pub use secret_notes_tooltip::SecretNotesTooltip;
```

**Step 3: Verify compilation**

Run: `cargo check`

**Step 4: Commit**

```bash
git add src/components/globals/
git commit -m "feat(ui): add SecretNotesTooltip component for secure notes display"
```

---

## Task 6: Update table_row.rs

**Files:**
- Modify: `src/components/globals/table/table_row.rs`

**Step 1: Update imports**

```rust
use crate::backend::password_types_helper::{PasswordScore, StoredRawPassword};
use crate::components::StoredPasswordUpsertDialogState;
use crate::components::globals::form_field::FormSecret;
use crate::components::globals::secret_display::SecretDisplay;
use crate::components::globals::secret_notes_tooltip::SecretNotesTooltip;
use crate::components::globals::password_handler::StrengthAnalyzer;
use crate::components::globals::svgs::{BurgerIcon, DeleteIcon, EditIcon};
use dioxus::prelude::*;
```

**Step 2: Update Column 1 (Location)**

```rust
// Column 1: Location (visualizzazione sicura con toggle)
td { class: "px-4 py-3",
    SecretDisplay {
        secret: FormSecret(store_raw_password_clone.location.clone()),
        max_width: "150px".to_string(),
    }
}
```

**Step 3: Update Column 2 (Password)**

```rust
// Column 2: Password (visualizzazione sicura con toggle)
td { class: "px-4 py-3",
    SecretDisplay {
        secret: FormSecret(store_raw_password_clone.password.clone()),
        max_width: "200px".to_string(),
    }
}
```

**Step 4: Update Burger Tooltip (Column 4)**

Replace the inline tooltip content with SecretNotesTooltip:

```rust
// Column 4: Burger button (tooltip for notes and created_at)
td { class: "px-2 py-3",
    div { class: "relative",
        button {
            class: "pwd-row-action-btn pwd-burger-btn",
            r#type: "button",
            onclick: move |_| show_info_tooltip.set(!show_info_tooltip()),
            BurgerIcon {}
        }

        if show_info_tooltip() {
            div {
                class: "fixed inset-0 z-[5]",
                onclick: move |_| show_info_tooltip.set(false),
            }

            div { class: "pwd-row-tooltip absolute right-0 top-full mt-2 z-10",
                SecretNotesTooltip {
                    notes: store_raw_password_clone.notes.clone(),
                    created_at: store_raw_password_clone.created_at.clone(),
                }
            }
        }
    }
}
```

**Step 5: Verify compilation**

Run: `cargo check`

**Step 6: Commit**

```bash
git add src/components/globals/table/table_row.rs
git commit -m "feat(ui): use SecretDisplay for location and SecretNotesTooltip for notes"
```

---

## Task 7: Update Form/Dialog Components

**Files:**
- Check: `src/components/` per form che usano StoredRawPassword

**Step 1: Search for StoredRawPassword usage**

```bash
grep -rn "StoredRawPassword" src/components/
```

Expected output based on current codebase:
- `src/components/features/dashboard.rs` - uses StoredRawPassword for filtering
- `src/components/globals/table/table_row.rs` - already updated in Task 6
- Dialog components if any

**Step 2: Check dashboard.rs for location access**

Run: `grep -n "\.location" src/components/features/dashboard.rs`

If any direct location access found, update to use `.location.expose_secret()`.

**Step 3: Check for edit/upsert dialogs**

Look for any dialog that pre-populates form fields with existing StoredRawPassword data. If found:

```rust
// Example: if dialog receives StoredRawPassword for editing
// The dialog will need to handle SecretString -> plaintext conversion for input fields
// This is acceptable since we're in an edit context (user explicitly chose to edit)

location_input: existing.location.expose_secret().to_string(),
notes_input: existing.notes.as_ref().map(|n| n.expose_secret().to_string()),
```

**Step 4: Verify compilation**

Run: `cargo check`

**Step 5: Commit (if changes made)**

```bash
git add src/components/
git commit -m "feat(ui): handle SecretString in form dialogs"
```

---

## Task 8: Final Verification

**Files:**
- All modified files

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Build the application**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Run the application**

Run: `dx serve --desktop`
Expected: App starts correctly

**Step 4: Manual verification**

1. Create password entry with location and notes
2. Verify table shows:
   - Location: `•••••` (click eye to reveal)
   - Password: `•••••` (click eye to reveal)
3. Click burger button → tooltip shows:
   - Notes: `•••••••• (click to reveal)` (click eye to reveal)
   - Created: visible
4. All three fields stay hidden until explicit user interaction

---

## Summary of Changes

| File | Changes |
|------|---------|
| `password_types_helper.rs` | `location`: String → SecretString, `notes`: Option<String> → Option<SecretString>, custom Debug impl |
| `password_utils.rs` | Wrap decrypted location/notes in SecretString |
| `password_utils_tests.rs` | Use `.expose_secret()` for assertions |
| `secret_display/mod.rs` | New component (renamed from PasswordDisplay) |
| `secret_notes_tooltip/mod.rs` | New component for tooltip with reveal |
| `globals/mod.rs` | Export new components |
| `table/table_row.rs` | Use SecretDisplay + SecretNotesTooltip |
| `input_main.css` | Rename CSS classes to secret-display-* |
| Form dialogs | Handle SecretString for location/notes |

---

## Security Model

```
┌─────────────────────────────────────────────────────────────┐
│  DATABASE (SQLite)                                          │
│  location: BLOB (encrypted)                                 │
│  notes: BLOB (encrypted)                                    │
│  password: BLOB (encrypted)                                 │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼ decrypt_bulk_stored_data()
┌─────────────────────────────────────────────────────────────┐
│  BACKEND - StoredRawPassword                                │
│  location: SecretString  ◄── mai esposta                   │
│  notes: Option<SecretString>  ◄── mai esposta              │
│  password: SecretString  ◄── mai esposta                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼ Frontend receives SecretString
┌─────────────────────────────────────────────────────────────┐
│  UI - TABLE ROW                                             │
│  ┌─────────────────┐  ┌─────────────────┐                  │
│  │ Location: ••••  │  │ Password: ••••  │  ← click to reveal│
│  └─────────────────┘  └─────────────────┘                  │
│                                                             │
│  BURGER BUTTON → Tooltip                                    │
│  ┌─────────────────────────────────────┐                   │
│  │ Notes: •••••••• (click to reveal)   │  ← click to reveal│
│  │ Created: 2024-01-15                 │  (visible)        │
│  └─────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

**Principle**: Plaintext is ONLY in memory during `.expose_secret()` call, triggered by explicit user action.
