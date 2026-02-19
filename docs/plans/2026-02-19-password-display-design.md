# PasswordDisplay Component Design Document

**Date:** 2026-02-19
**Author:** Design generated via brainstorming session
**Status:** Approved - Ready for implementation planning

## Overview

A read-only password display component for secure visualization of passwords in table rows. The component shows masked passwords (bullets) with a toggle to reveal the plaintext, along with a clipboard button placeholder for future copy functionality.

## Problem Statement

Currently, the `StoredRawPasswordRow` component displays passwords in plaintext using `password.expose_secret()`. This is a security vulnerability and poor UX - passwords should never be visible in a table view unless explicitly revealed by the user.

**Current code (INSECURE):**
```rust
td { class: "px-4 py-3",
    div {
        class: "truncate max-w-[200px] font-mono",
        title: "{props.stored_raw_password.password.expose_secret()}",
        "{props.stored_raw_password.password.expose_secret()}"
    }
}
```

## Solution

Create a new `PasswordDisplay` component that:
1. Always displays passwords masked by default (•••••)
2. Provides a toggle button to reveal/hide the password
3. Shows tooltip password only when unlocked
4. Prevents row height changes (no shuttering)
5. Includes a clipboard button for future functionality

## Architecture

### Component Structure

```
src/components/globals/password_display/
├── mod.rs          # Exports PasswordDisplay
└── component.rs    # Implementation
```

### Dependencies

- `secrecy::SecretString` - Secure password handling
- `dioxus::prelude::*` - Framework hooks and RSX
- `crate::components::globals::svgs::{EyeIcon, EyeOffIcon}` - Toggle icons
- `crate::components::globals::svgs::action_icons::*` - Clipboard icon (future)

## Component Specification

### Props

```rust
#[derive(Props, Clone, PartialEq)]
pub struct PasswordDisplayProps {
    /// La password da visualizzare (SecretString per sicurezza)
    pub password: SecretString,

    /// Classe CSS aggiuntiva per il container (opzionale)
    #[props(default)]
    pub class: Option<String>,

    /// Larghezza massima del contenitore (default: 200px come in table_row)
    #[props(default = "200px".to_string())]
    pub max_width: String,

    /// Callback quando si clicca sull'icona clipboard (TODO: implementare copia)
    /// Se None, il button clipboard viene mostrato ma disabilitato
    #[props(default)]
    pub on_copy: Option<Callback<()>>,
}
```

### Internal State

```rust
/// Stato per la visibilità della password (false = nascosta/pallini)
let mut password_visible = use_signal(|| false);
```

### RSX Structure

```rust
rsx! {
    div { class: "password-display-wrapper {class}",
        // Input password read-only con toggle visibility
        input {
            class: "pwd-password-display font-mono",
            r#type: if password_visible() { "text" } else { "password" },
            value: if password_visible() {
                props.password.expose_secret().to_string()
            } else {
                "•".repeat(props.password.expose_secret().len())
            },
            readonly: true,
            title: if password_visible() {
                Some(props.password.expose_secret().to_string())
            } else {
                None
            },
            style: "max-width: {props.max_width}",
        }

        // Actions container (toggle + clipboard)
        div { class: "password-display-actions flex gap-1",
            // Toggle visibility button
            button {
                class: "pwd-display-action-btn",
                r#type: "button",
                onclick: move |_| password_visible.set(!password_visible()),
                aria_label: if password_visible() { "Nascondi password" } else { "Mostra password" },
                if password_visible() {
                    EyeOffIcon { class: Some("text-current".to_string()) }
                } else {
                    EyeIcon { class: Some("text-current".to_string()) }
                }
            }

            // Copy to clipboard button (placeholder for future implementation)
            button {
                class: "pwd-display-action-btn {props.on_copy.is_none().then(|| "opacity-50 cursor-not-allowed").unwrap_or_default()}",
                r#type: "button",
                disabled: props.on_copy.is_none(),
                aria_label: "Copia password",
                // TODO: Add clipboard functionality
            }
        }
    }
}
```

## CSS Styling (Tailwind v4)

Add to `assets/input_main.css`:

```css
/* Password Display Component */
.password-display-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: fit-content;
}

.pwd-password-display {
    background-color: transparent;
    border: none;
    outline: none;
    font-size: 1rem;
    cursor: text;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.pwd-display-action-btn {
    padding: 0.25rem;
    border-radius: 0.25rem;
    transition: background-color 150ms cubic-bezier(0.4, 0, 0.2, 1);
    color: inherit;
}

.pwd-display-action-btn:hover {
    background-color: color-mix(in srgb, var(--fallback-bc, oklch(0.2 0 0)) 10%, transparent);
}

.pwd-display-action-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--fallback-pc, oklch(0.6 0.2 250)) 30%, transparent);
    outline-offset: 2px;
}

.pwd-display-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
}
```

**Design Notes:**
- No `@apply` or `@layer` - Tailwind v4 uses native CSS
- `color-mix()` for opacity and color variations
- CSS variables for theme colors (`--fallback-bc`, `--fallback-pc`)
- All classes use `pwd-` prefix to avoid DaisyUI 5 conflicts

## Integration with Existing Code

### Modify `src/components/globals/table/table_row.rs`

**Remove (lines 42-49):**
```rust
// Column 2: Password (with ellipsis)
td { class: "px-4 py-3",
    div {
        class: "truncate max-w-[200px] font-mono",
        title: "{props.stored_raw_password.password.expose_secret()}",
        "{props.stored_raw_password.password.expose_secret()}"
    }
}
```

**Add:**
```rust
// Column 2: Password (visualizzazione sicura con toggle)
td { class: "px-4 py-3",
    PasswordDisplay {
        password: props.stored_raw_password.password.clone(),
        max_width: "200px".to_string(),
    }
}
```

**Add import:**
```rust
use crate::components::globals::password_display::PasswordDisplay;
```

### Update `src/components/globals/table/mod.rs`

```rust
mod table_row;
mod password_display;  // New module

pub use table_row::{StoredRawPasswordRow, StoredRawPasswordRowProps};
pub use password_display::PasswordDisplay;  // New export
```

## Edge Cases & Error Handling

| Edge Case | Solution |
|-----------|----------|
| Empty password | Returns empty string, component renders without crash |
| Very long passwords | CSS `text-overflow: ellipsis` handles truncate |
| Performance with many rows | Toggle only re-renders single row, `repeat()` is O(n) but n < 50 typically |
| Memory safety | Exposed string is temporary, GC'd on re-render |
| Accessibility | `readonly: true` announces correctly to screen readers |

## Testing Checklist

### Manual Browser Tests
- [ ] Default: shows bullets (•••••)
- [ ] Click eye: shows password in plaintext
- [ ] Click eye-closed: returns to bullets
- [ ] Hover tooltip: shows password only when unlocked
- [ ] No row height changes (shuttering) on toggle
- [ ] Truncate works with long passwords (> 200px)
- [ ] Icons aligned correctly
- [ ] Clipboard button is disabled and grayed out
- [ ] Screen reader announces "password, read only"
- [ ] Tab navigation works correctly
- [ ] Focus state visible on action buttons

### Regression Tests
- [ ] Table loads correctly
- [ ] Edit/Delete buttons still work
- [ ] No console errors (F12)
- [ ] Performance acceptable with 100+ rows

## Design Decisions

### Why `readonly` instead of `disabled`?
- **Semantics:** "read only" means user can read but not modify (correct for our use case)
- **Accessibility:** Screen readers announce correctly
- **Functionality:** Allows text selection for manual copy

### Why separate component instead of extending FormField?
- **Separation of concerns:** Display vs editing are different use cases
- **Simplicity:** No props pollution from FormField's editing features
- **Maintainability:** Isolated code, easier to test and modify

### Why not use `use_memo` for display value?
- Toggle is local interaction (doesn't affect other rows)
- `repeat()` calculation is cheap for typical password lengths
- Re-render only on click, not continuous

### Why Tailwind v4 CSS instead of v3?
- Project uses Tailwind v4 syntax
- No `@apply` or `@layer` needed
- Better performance with native CSS and `color-mix()`

## Future Work

1. **Clipboard functionality:** Implement `on_copy` callback to copy password to clipboard
2. **Password strength indicator:** Optional color coding based on strength
3. **Keyboard shortcuts:** Support for Ctrl+C to copy when password is visible
4. **Animation:** Smooth transition between masked and visible states

## Security Considerations

- ✅ Passwords never shown in plaintext by default
- ✅ Tooltip only shows when explicitly unlocked
- ✅ No password logging in debug output
- ✅ `SecretString` ensures secure memory handling
- ⚠️ When visible, password is in DOM - user should be aware of shoulder surfing

## Implementation Status

- [ ] Design approved
- [ ] Implementation plan created
- [ ] Component implemented
- [ ] CSS added to input_main.css
- [ ] Integrated with table_row
- [ ] Testing completed
- [ ] Ready for clipboard feature implementation

---

**Next Steps:**
1. Create implementation plan using `writing-plans` skill
2. Verify Dioxus 0.7.3 compatibility via agent review
3. Implement component following TDD best practices
