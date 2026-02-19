# PasswordDisplay Component Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a secure password display component for table rows that shows masked passwords (•••••) with a toggle to reveal plaintext.

**Architecture:** New presentational component using Dioxus 0.7.3 with `<input type="password" readonly>` for security. Component manages its own visibility state via `use_signal`. Integrates into existing `StoredRawPasswordRow` replacing insecure plaintext display.

**Tech Stack:** Rust, Dioxus 0.7.3, secrecy crate, Tailwind CSS v4, project's CSS variable system

---

## Task 1: Create password_display module structure

**Files:**
- Create: `src/components/globals/password_display/mod.rs`

**Step 1: Create the module file with exports**

```rust
mod component;

pub use component::PasswordDisplay;
```

**Step 2: Create component.rs file placeholder**

Create empty file: `src/components/globals/password_display/component.rs`

**Step 3: Update table/mod.rs to include new module**

File: `src/components/globals/table/mod.rs`

Add after line 4:
```rust
mod password_display;
```

Add to exports (after line 11):
```rust
pub use password_display::PasswordDisplay;
```

**Step 4: Run cargo check to verify module structure**

Run: `cargo check`

Expected: SUCCESS (or warnings about empty component.rs)

**Step 5: Commit**

```bash
git add src/components/globals/password_display/ src/components/globals/table/mod.rs
git commit -m "feat: add password_display module structure"
```

---

## Task 2: Implement PasswordDisplay component - basic structure

**Files:**
- Modify: `src/components/globals/password_display/component.rs`

**Step 1: Write the component skeleton with imports**

```rust
use crate::components::globals::svgs::{EyeIcon, EyeOffIcon};
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};

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
    pub on_copy: Option<EventHandler<()>>,
}

#[component]
pub fn PasswordDisplay(props: PasswordDisplayProps) -> Element {
    // TODO: Implement component
    rsx! {
        div { "PasswordDisplay placeholder" }
    }
}
```

**Step 2: Run cargo check to verify types compile**

Run: `cargo check`

Expected: SUCCESS with warnings about unused code

**Step 3: Commit**

```bash
git add src/components/globals/password_display/component.rs
git commit -m "feat: add PasswordDisplay component skeleton with props"
```

---

## Task 3: Implement PasswordDisplay - internal state

**Files:**
- Modify: `src/components/globals/password_display/component.rs`

**Step 1: Add internal state for visibility toggle**

Replace the component function with:

```rust
#[component]
pub fn PasswordDisplay(props: PasswordDisplayProps) -> Element {
    /// Stato per la visibilità della password (false = nascosta/pallini)
    let mut password_visible = use_signal(|| false);

    rsx! {
        div { "PasswordDisplay placeholder - visible: {password_visible()}" }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`

Expected: SUCCESS

**Step 3: Run development server to verify component loads**

Run: `dx serve --desktop`

Expected: App launches successfully

**Step 4: Stop dev server**

Press Ctrl+C in terminal

**Step 5: Commit**

```bash
git add src/components/globals/password_display/component.rs
git commit -m "feat: add visibility state to PasswordDisplay"
```

---

## Task 4: Implement PasswordDisplay - RSX structure

**Files:**
- Modify: `src/components/globals/password_display/component.rs`

**Step 1: Write the complete RSX structure**

Replace the entire component function with:

```rust
#[component]
pub fn PasswordDisplay(props: PasswordDisplayProps) -> Element {
    /// Stato per la visibilità della password (false = nascosta/pallini)
    let mut password_visible = use_signal(|| false);

    // Calcola il valore da mostrare
    let password_len = props.password.expose_secret().len();
    let display_value = if password_len == 0 {
        String::new()
    } else if password_visible() {
        props.password.expose_secret().to_string()
    } else {
        "•".repeat(password_len)
    };

    rsx! {
        div { class: "password-display-wrapper {props.class.clone().unwrap_or_default()}",
            // Input password read-only con toggle visibility
            input {
                class: "pwd-password-display font-mono",
                r#type: if password_visible() { "text" } else { "password" },
                value: "{display_value}",
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
                    class: "pwd-display-action-btn",
                    r#type: "button",
                    disabled: props.on_copy.is_none(),
                    aria_label: "Copia password",
                    // TODO: Add clipboard icon and functionality
                    span { class: "text-xs", "📋" }
                }
            }
        }
    }
}
```

**Step 2: Run cargo check to verify RSX syntax**

Run: `cargo check`

Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/components/globals/password_display/component.rs
git commit -m "feat: implement PasswordDisplay RSX structure with toggle"
```

---

## Task 5: Add CSS styling for PasswordDisplay

**Files:**
- Modify: `assets/input_main.css`

**Step 1: Add CSS classes to input_main.css**

Add these classes at the end of the file (before `/* END OF CUSTOM STYLES */` if present):

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
    background-color: var(--primary-color-3);
}

.pwd-display-action-btn:focus-visible {
    outline: 2px solid var(--focused-border-color);
    outline-offset: 2px;
}

.pwd-display-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
}
```

**Step 2: Build Tailwind CSS to regenerate styles**

Run: `npx tailwindcss -i ./assets/input.css -o ./assets/tailwind.css`

Expected: Success with no errors

**Step 3: Run cargo check**

Run: `cargo check`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add assets/input_main.css assets/tailwind.css
git commit -m "feat: add CSS styling for PasswordDisplay component"
```

---

## Task 6: Integrate PasswordDisplay into table_row

**Files:**
- Modify: `src/components/globals/table/table_row.rs`

**Step 1: Add PasswordDisplay import**

Add at the top after other imports (after line 4):

```rust
use crate::components::globals::password_display::PasswordDisplay;
```

**Step 2: Replace the password display (lines 42-49)**

Find this section:
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

Replace with:
```rust
// Column 2: Password (visualizzazione sicura con toggle)
td { class: "px-4 py-3",
    PasswordDisplay {
        password: props.stored_raw_password.password.clone(),
        max_width: "200px".to_string(),
    }
}
```

**Step 3: Run cargo check**

Run: `cargo check`

Expected: SUCCESS

**Step 4: Build and run the app**

Run: `dx serve --desktop`

Expected:
- App launches successfully
- Navigate to password table
- Passwords show as bullets (•••••)
- Toggle button works
- No console errors

**Step 5: Stop dev server**

Press Ctrl+C

**Step 6: Commit**

```bash
git add src/components/globals/table/table_row.rs
git commit -m "feat: integrate PasswordDisplay into table_row"
```

---

## Task 7: Manual browser testing

**Files:**
- None (testing only)

**Step 1: Run development server**

Run: `dx serve --desktop`

**Step 2: Log in and navigate to password table**

- Launch app
- Log in with test credentials
- Navigate to the page showing stored passwords

**Step 3: Test default state**

Checklist:
- [ ] Passwords display as bullets (•••••), not plaintext
- [ ] Row height is consistent (no shuttering)
- [ ] No console errors (F12 → Console tab)

**Step 4: Test toggle functionality**

Checklist:
- [ ] Click eye icon → password shows in plaintext
- [ ] Click eye-off icon → password returns to bullets
- [ ] Tooltip shows password only when unlocked
- [ ] Toggle doesn't affect other rows

**Step 5: Test layout edge cases**

Checklist:
- [ ] Long passwords truncate correctly (> 200px)
- [ ] Empty passwords don't crash
- [ ] Icons align properly with password text
- [ ] Clipboard button appears disabled (grayed out)

**Step 6: Test accessibility**

Checklist:
- [ ] Tab key navigates to toggle button
- [ ] Focus state visible on button
- [ ] Screen reader announces button purpose (if available)

**Step 7: Test with multiple rows**

Checklist:
- [ ] Scroll through 10+ rows
- [ ] Toggle multiple different rows
- [ ] No performance issues
- [ ] No memory leaks

**Step 8: Stop dev server**

Press Ctrl+C

**Step 9: Create test report**

Create: `docs/plans/2026-02-19-password-display-test-report.md`

```markdown
# PasswordDisplay Test Report

**Date:** 2026-02-19
**Tester:** [Your name]
**Status:** PASSED / FAILED

## Test Results

### Default State
- [x] Passwords show as bullets
- [x] No shuttering
- [x] No console errors

### Toggle Functionality
- [x] Eye icon reveals password
- [x] Eye-off hides password
- [x] Tooltip only when unlocked
- [x] Independent per row

### Layout
- [x] Long passwords truncate
- [x] Empty passwords handled
- [x] Icons aligned properly
- [x] Clipboard disabled

### Accessibility
- [x] Tab navigation works
- [x] Focus visible
- [x] Screen reader friendly

### Performance
- [x] No issues with 10+ rows
- [x] No memory leaks

## Issues Found

None / [List any issues found]

## Screenshots

[Attach screenshots if applicable]
```

**Step 10: Commit test report (optional)**

```bash
git add docs/plans/2026-02-19-password-display-test-report.md
git commit -m "test: add PasswordDisplay manual test report"
```

---

## Task 8: Add clipboard icon placeholder (future-ready)

**Files:**
- Modify: `src/components/globals/password_display/component.rs`

**Step 1: Check clipboard icon availability**

First, verify the clipboard icon exists:

Run: `grep -r "clipboard\|Clipboard" src/components/globals/svgs/`

Expected: Find clipboard icon in action_icons or similar

**Step 2: Import clipboard icon**

If clipboard icon exists, add to imports at top of file:

```rust
use crate::components::globals::svgs::{EyeIcon, EyeOffIcon};
use crate::components::globals::svgs::action_icons::ClipboardIcon;  // Adjust path as needed
```

**Step 3: Replace clipboard button placeholder**

Find this section in RSX:
```rust
// Copy to clipboard button (placeholder for future implementation)
button {
    class: "pwd-display-action-btn",
    r#type: "button",
    disabled: props.on_copy.is_none(),
    aria_label: "Copia password",
    // TODO: Add clipboard icon and functionality
    span { class: "text-xs", "📋" }
}
```

Replace with (if clipboard icon exists):
```rust
// Copy to clipboard button (placeholder for future implementation)
button {
    class: "pwd-display-action-btn",
    r#type: "button",
    disabled: props.on_copy.is_none(),
    aria_label: "Copia password",
    // TODO: Implement clipboard functionality
    ClipboardIcon { class: Some("text-current".to_string()) }
}
```

If clipboard icon doesn't exist, keep the placeholder emoji for now.

**Step 4: Run cargo check**

Run: `cargo check`

Expected: SUCCESS

**Step 5: Run dev server to verify**

Run: `dx serve --desktop`

Expected: Clipboard icon appears (or placeholder if icon doesn't exist)

**Step 6: Stop dev server**

Press Ctrl+C

**Step 7: Commit**

```bash
git add src/components/globals/password_display/component.rs
git commit -m "feat: add clipboard icon placeholder to PasswordDisplay"
```

---

## Task 9: Final verification and documentation

**Files:**
- Create: `docs/password-display-usage.md` (optional)

**Step 1: Run full test suite (if exists)**

Run: `cargo test`

Expected: All tests pass (or no tests exist yet)

**Step 2: Run cargo clippy for linting**

Run: `cargo clippy --all-targets --all-features`

Expected: No clippy warnings (or acceptable warnings)

**Step 3: Build release version**

Run: `dx build --desktop --release`

Expected: Successful build

**Step 4: Create usage documentation (optional)**

Create: `docs/password-display-usage.md`

```markdown
# PasswordDisplay Component Usage

## Overview

`PasswordDisplay` is a secure password display component for showing passwords in a masked state with toggle visibility.

## Basic Usage

```rust
use crate::components::globals::password_display::PasswordDisplay;
use secrecy::SecretString;

PasswordDisplay {
    password: SecretString::new("my-password".into()),
    max_width: "200px".to_string(),
}
```

## Props

- `password: SecretString` - The password to display (required)
- `class: Option<String>` - Additional CSS class (optional)
- `max_width: String` - Maximum width (default: "200px")
- `on_copy: Option<EventHandler<()>>` - Copy callback (TODO: not implemented)

## Future Work

- Implement clipboard copy functionality
- Add keyboard shortcuts (Ctrl+C when visible)
- Add smooth reveal/hide animations
```

**Step 5: Final commit**

```bash
git add docs/password-display-usage.md
git commit -m "docs: add PasswordDisplay usage documentation"
```

**Step 6: Create summary of changes**

Create: `docs/plans/2026-02-19-password-display-summary.md`

```markdown
# PasswordDisplay Implementation Summary

**Date:** 2026-02-19
**Feature:** Secure password display component

## What Was Built

- New `PasswordDisplay` component for secure password visualization
- Toggle visibility (bullets ↔ plaintext)
- Placeholder for clipboard functionality
- CSS styling with project's variable system
- Integration into existing table rows

## Files Changed

### Created
- `src/components/globals/password_display/mod.rs`
- `src/components/globals/password_display/component.rs`

### Modified
- `src/components/globals/table/mod.rs`
- `src/components/globals/table/table_row.rs`
- `assets/input_main.css`

## Testing

Manual browser testing completed successfully. All checklist items passed.

## Known Limitations

- Clipboard functionality not implemented (button disabled)
- No keyboard shortcuts yet
- No reveal/hide animations

## Next Steps

1. Implement clipboard copy functionality
2. Add keyboard shortcut support
3. Consider adding reveal animations
4. Add unit tests if Dioxus testing framework available
```

**Step 7: Commit summary**

```bash
git add docs/plans/2026-02-19-password-display-summary.md
git commit -m "docs: add PasswordDisplay implementation summary"
```

---

## Completion Checklist

After all tasks complete:

- [ ] All commits pushed to branch
- [ ] Design document updated with implementation status
- [ ] Test report filed
- [ ] Code review requested (if applicable)
- [ ] Ready for merge to main branch

---

## Notes for Implementation

### Dioxus 0.7.3 Specifics

- Use `EventHandler<T>`, not `Callback<T>`
- Use `use_signal` for mutable state
- Use `r#type` for reserved HTML keywords
- Use snake_case for attributes with special characters (e.g., `aria_label`)
- All `Signal` must be `mut` even if compiler suggests removing it

### Project Conventions

- All custom CSS classes use `pwd-` prefix
- Use existing CSS variables (`--primary-color-3`, `--focused-border-color`)
- No `color-mix()` - use direct CSS variables
- No `@apply` or `@layer` - use native CSS

### Testing Approach

- Manual browser testing (Dioxus has limited unit test support)
- Test with multiple rows to verify no performance issues
- Check accessibility (tab navigation, screen reader)
- Verify no console errors

### Git Commit Pattern

Use conventional commits:
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation
- `test:` for tests
- `refactor:` for refactoring

Always include `Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>` in commit messages.
