# Tabs Restyling Design

**Date:** 2026-03-19
**Status:** Approved

## Goal

Restyle the `Tabs`, `TabList`, `TabTrigger`, `TabContent` components from the current "futuristic" shadow/border style to a clean border-bottom indicator style, matching classic TailwindCSS pill-style tabs.

## Decisions

- **Single variant only** — remove `TabsVariant` enum and `Ghost` variant
- **Border-bottom indicator** — 3px border on active tab, color = `--color-primary`
- **Full-width triggers** — `flex-grow: 1` on each tab trigger
- **Title Case text** — no uppercase transformation
- **Keep `futuristic` class** — required for custom font (`pwd-futuristic`, `pwd-common`)
- **Nested tabs** — inner tabs use slightly smaller font (`0.8125rem`) and reduced padding

## Files to Modify

### 1. `src/components/globals/tabs/component.rs`

- Remove `TabsVariant` enum and its `to_class()` method
- Remove `variant` field from `TabsProps`
- Remove `data-variant` attribute from `Tabs` RSX
- Update CSS classes (keep `futuristic`):
  - `Tabs`: `class + " pwd-tabs futuristic"`
  - `TabList`: `"pwd-tabs-list futuristic"`
  - `TabTrigger`: `"pwd-tabs-trigger"` (no futuristic — buttons get `pwd-common` from `.futuristic button`)
  - `TabContent`: `class.unwrap_or_default() + " pwd-tabs-content futuristic"`

### 2. `assets/input_main.css`

**Remove** all existing tab CSS:
- `.tabs` (line 1031-1037)
- `.tabs-list` (line 1039-1049)
- `[data-variant="default"] .tabs-list` (line 1051-1053)
- `.tabs-trigger` (line 1055-1064)
- `[data-variant="default"] .tabs-trigger[data-state="active"]` (line 1066-1070)
- `[data-theme="dark"] [data-variant="default"] .tabs-trigger[data-state="active"]` (line 1072-1076)
- `.tabs-trigger[data-state="active"]` (line 1079-1081)
- `.tabs-trigger[data-disabled="true"]` (line 1083-1086)
- `.tabs-trigger:hover:not(...)` (line 1088-1091)
- `.tabs-content` (line 1093-1097)
- `.tabs-content[data-state="inactive"]` (line 1100-1102)
- Dark theme tab overrides (line 2140-2152)
- Tab shadow CSS variables (line 174-177)

**Add** new CSS rules:

```css
/* Tab shadow variables - REMOVE */

/* Main tabs container */
.pwd-tabs {
    display: flex;
    flex-direction: column;
    width: 100%;
}

/* Tab navigation list */
.pwd-tabs-list {
    display: flex;
    flex-direction: row;
    border-bottom: 1px solid var(--color-base-300);
    list-style: none;
    padding: 0;
    margin: 0 0 1rem 0;
}

/* Individual tab trigger */
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

.pwd-tabs-trigger:hover:not([data-disabled="true"]) {
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

/* Tab content panels */
.pwd-tabs-content {
    width: 100%;
}

.pwd-tabs-content[data-state="inactive"] {
    display: none;
}

/* Nested (inner) tabs — smaller size */
.pwd-tabs-inner .pwd-tabs-list {
    margin-bottom: 0.75rem;
}

.pwd-tabs-inner .pwd-tabs-trigger {
    font-size: 0.8125rem;
    padding: 0.5rem 1rem;
}
```

### 3. `assets/input_main.css` — dark theme overrides

**Remove** the dark theme tab overrides (lines 2140-2152) since the new styles use DaisyUI CSS variables (`--color-primary`, `--color-base-content`, `--color-base-300`) that already adapt to the active theme.

## Impact on Consumers

- **`settings.rs`** — no changes needed. Does not use `variant` prop. Class changes are transparent.
- No other files import or use the tab components.

## Visual Reference

Mockups saved in `.superpowers/`:
- `tab-mockup-1.html` — side-by-side comparison (Option A vs B)
- `tab-nested-mockup.html` — nested tabs preview (light + dark mode)
