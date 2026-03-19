# Tabs Restyling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle Tab components from "futuristic shadow/border" to clean border-bottom indicator style.

**Architecture:** Thin wrapper components (`Tabs`, `TabList`, `TabTrigger`, `TabContent`) around `dioxus_primitives::tabs`. Only CSS classes and Rust class names change — no logic changes. The `TabsVariant` enum is removed (single style only).

**Tech Stack:** Rust, Dioxus 0.7, Tailwind CSS, DaisyUI 5 CSS variables

**Spec:** `docs/superpowers/specs/2026-03-19-tabs-restyling-design.md`

**Mockups:** `.superpowers/tab-nested-mockup.html`

---

### Task 1: Update Rust wrapper — remove TabsVariant and update CSS classes

**Files:**
- Modify: `src/components/globals/tabs/component.rs`

- [ ] **Step 1: Remove `TabsVariant` and simplify `TabsProps`**

Remove the entire `TabsVariant` enum (lines 46-65) and its `variant` field from `TabsProps` (line 34-36). Remove the `data-variant` attribute from the `Tabs` component RSX (line 72).

The final file should look like:

```rust
use dioxus::prelude::*;
use dioxus_primitives::tabs::{self, TabContentProps, TabListProps, TabTriggerProps};

/// The props for the [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    /// The class of the tabs component.
    #[props(default)]
    pub class: String,

    /// The controlled value of the active tab.
    pub value: ReadSignal<Option<String>>,

    /// The default active tab value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired when the active tab changes.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Whether the tabs are disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the tabs are horizontal.
    #[props(default)]
    pub horizontal: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the tabs element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tabs component.
    pub children: Element,
}

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    rsx! {
        tabs::Tabs {
            class: props.class + " pwd-tabs" + " futuristic",
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabList(props: TabListProps) -> Element {
    rsx! {
        tabs::TabList { class: "pwd-tabs-list futuristic", attributes: props.attributes, {props.children} }
    }
}

#[component]
pub fn TabTrigger(props: TabTriggerProps) -> Element {
    rsx! {
        tabs::TabTrigger {
            class: "pwd-tabs-trigger",
            id: props.id,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    rsx! {
        tabs::TabContent {
            class: props.class.unwrap_or_default() + " pwd-tabs-content" + " futuristic",
            value: props.value,
            id: props.id,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}
```

**Key changes:**
- `TabsVariant` enum removed entirely
- `variant` field removed from `TabsProps`
- `data-variant` attribute removed from RSX
- CSS classes renamed: `tabs` → `pwd-tabs`, `tabs-list` → `pwd-tabs-list`, `tabs-trigger` → `pwd-tabs-trigger`, `tabs-content` → `pwd-tabs-content`
- `futuristic` class **kept** on `Tabs`, `TabList`, `TabContent` (for custom font)
- `TabTrigger` does NOT get `futuristic` (buttons get `pwd-common` via `.futuristic button` CSS rule)

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compiles without errors (no other file references `TabsVariant`)

- [ ] **Step 3: Commit**

```bash
git add src/components/globals/tabs/component.rs
git commit -m "refactor: remove TabsVariant, update tab CSS class names"
```

---

### Task 2: Update consumer — rename old class names in settings.rs

**Files:**
- Modify: `src/components/features/settings.rs`

- [ ] **Step 1: Rename stale `tabs-content` class names and add `pwd-tabs-inner`**

In `settings.rs`, make these changes:

1. **Line 37** — change `class: "tabs-content border-none shadow-none"` to `class: "pwd-tabs-content border-none shadow-none"`
2. **Line 46** — change `class: "tabs-content"` to `class: "pwd-tabs-content"`
3. **Line 48** — add `class: "pwd-tabs-inner".to_string()` to the nested `Tabs` component. The line becomes:

```rust
                Tabs {
                    class: "pwd-tabs-inner".to_string(),
                    default_value: "Password Casuale".to_string(),
                    horizontal: true,
```

4. **Line 61** — change `class: "tabs-content"` to `class: "pwd-tabs-content"`
5. **Line 67** — change `class: "tabs-content"` to `class: "pwd-tabs-content"`
6. **Line 75** — change `class: "tabs-content"` to `class: "pwd-tabs-content"`

- [ ] **Step 2: Commit**

```bash
git add src/components/features/settings.rs
git commit -m "refactor: rename tab class names and add pwd-tabs-inner to nested tabs"
```

---

### Task 3: Replace old tab CSS with new border-bottom style

**Files:**
- Modify: `assets/input_main.css`

- [ ] **Step 1: Remove tab shadow CSS variables (lines 174-177)**

Delete these 4 lines:
```css
    --tab-shadow-light: 0 5px 13px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.4);
    --tab-shadow-light-active: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.4);
    --tab-shadow-dark: 0 5px 13px -5px rgba(255, 255, 255, 0.05), 0 8px 10px -6px rgba(255, 255, 255, 0.05);
    --tab-shadow-dark-active: 0 10px 25px -5px rgba(255, 255, 255, 0.05), 0 8px 10px -6px rgba(255, 255, 255, 0.05);
```

- [ ] **Step 2: Replace old tab rules (lines 1031-1102) with new border-bottom style**

Replace the entire block from `.tabs {` through `.tabs-content[data-state="inactive"] { display: none; }` with:

```css
/* ---- pwd-tabs: main container ---- */

.pwd-tabs {
    display: flex;
    flex-direction: column;
    width: 100%;
}

/* ---- pwd-tabs-list: navigation bar ---- */

.pwd-tabs-list {
    display: flex;
    flex-direction: row;
    border-bottom: 1px solid var(--color-base-300);
    list-style: none;
    padding: 0;
    margin: 0 0 1rem 0;
}

/* ---- pwd-tabs-trigger: individual tab button ---- */

.pwd-tabs-trigger {
    flex-grow: 1;
    text-align: center;
    padding: 0.75rem 1.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-base-content);
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    margin-bottom: -1px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
}

.pwd-tabs-trigger:hover:not([data-disabled="true"]),
.pwd-tabs-trigger:focus-visible {
    color: var(--color-base-content);
    border-bottom-color: var(--color-base-300);
}

.pwd-tabs-trigger[data-state="active"] {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
    font-weight: 600;
}

.pwd-tabs-trigger[data-disabled="true"] {
    color: var(--color-base-300);
    pointer-events: none;
}

/* ---- pwd-tabs-content: tab panels ---- */

.pwd-tabs-content {
    width: 100%;
}

.pwd-tabs-content[data-state="inactive"] {
    display: none;
}

/* ---- pwd-tabs-inner: nested (inner) tabs ---- */

.pwd-tabs-inner .pwd-tabs-list {
    margin-bottom: 0.75rem;
}

.pwd-tabs-inner .pwd-tabs-trigger {
    font-size: 0.8125rem;
    padding: 0.5rem 1rem;
}
```

- [ ] **Step 3: Remove dark theme tab overrides (lines 2140-2152)**

Delete these lines in the dark theme `@layer components` section:
```css
    /* ---- Tabs trigger (non-active) ---- */

    .tabs-trigger {
        background: var(--color-base-200);
        color: var(--color-base-content);
        border-color: var(--color-base-300);
    }

    /* ---- Tab shadow ---- */

    [data-variant="default"] .tabs-list {
        background: var(--color-base-300);
    }
```

These are no longer needed — the new styles use DaisyUI CSS variables (`--color-primary`, `--color-base-content`, `--color-base-300`) that already adapt to the active theme.

- [ ] **Step 4: Verify no remaining references to old class names**

Search `input_main.css` for any remaining `.tabs-trigger`, `.tabs-list`, `.tabs-content`, or `[data-variant` references. There should be none.

Run: `grep -n "\.tabs-trigger\|\.tabs-list\|\.tabs-content\|\.tabs\b\|data-variant\|tab-shadow" assets/input_main.css`
Expected: no matches

- [ ] **Step 5: Commit**

```bash
git add assets/input_main.css
git commit -m "style: replace old tab CSS with border-bottom indicator style"
```

---

### Task 4: Visual verification

**No TDD — this is a pure CSS/styling change. Verification is visual.**

- [ ] **Step 1: Run the app with hot reload**

Run: `dx serve --desktop`

- [ ] **Step 2: Navigate to Settings page**

Verify:
1. Outer tabs (Account / Security / General) show border-bottom indicator on active tab
2. Active tab has primary color text + 3px border-bottom in primary color
3. Inactive tabs show neutral text, no border-bottom
4. Hover on inactive tabs shows a light border-bottom
5. Tabs are full-width (fill container)
6. Navigate to Security tab — inner tabs (Password Casuale / Diceware) render with smaller size
7. Toggle dark theme — verify colors adapt correctly
8. Font is still `pwd-futuristic` (from `.futuristic` class)

- [ ] **Step 3: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: tab styling adjustments from visual review"
```

Only commit if changes were needed.
