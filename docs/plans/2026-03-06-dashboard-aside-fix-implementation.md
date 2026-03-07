# Dashboard Aside Fix - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the centered badge alignment issue in the collapsed stats aside of the dashboard.

**Architecture:** CSS-only fix - add a flexbox rule to center stat items horizontally and remove lateral padding when the aside is in collapsed state. No component or logic changes required.

**Tech Stack:** CSS, Dioxus 0.7 desktop app

---

## Task 1: Add CSS Rule for Centered Items in Collapsed Aside

**Files:**
- Modify: `assets/input_main.css:1478` (after `.pwd-stats-aside--expanded .pwd-stats-aside__item`)

**Step 1: Locate the insertion point**

Open `assets/input_main.css` and find the `.pwd-stats-aside--expanded .pwd-stats-aside__item` rule at line 1475:

```css
.pwd-stats-aside--expanded .pwd-stats-aside__item {
    border-radius: 10px;
}
```

**Step 2: Add the new CSS rule**

Insert the following rule immediately after line 1477 (after the closing brace):

```css
/* Center the badge in collapsed aside */
.pwd-stats-aside--collapsed .pwd-stats-aside__item {
    justify-content: center;
    padding-left: 0;
    padding-right: 0;
}
```

This rule:
- `justify-content: center` - Centers the badge horizontally in the flex container
- `padding-left: 0` - Removes left padding to maximize available space
- `padding-right: 0` - Removes right padding to maximize available space

**Step 3: Verify the CSS file is correct**

Check that the new rule is properly formatted and positioned:

```bash
grep -A5 "pwd-stats-aside--collapsed .pwd-stats-aside__item" assets/input_main.css
```

Expected output:
```css
/* Center the badge in collapsed aside */
.pwd-stats-aside--collapsed .pwd-stats-aside__item {
    justify-content: center;
    padding-left: 0;
    padding-right: 0;
}
```

**Step 4: Rebuild Tailwind CSS**

Since we modified `input_main.css`, we need to rebuild the compiled CSS:

```bash
npx tailwindcss -i ./assets/input.css -o ./assets/tailwind.css
```

Expected: CSS recompiled without errors.

**Step 5: Test the changes**

Run the application in development mode:

```bash
dx serve --desktop
```

Expected: Application starts successfully.

**Step 6: Manual verification in the app**

1. Navigate to the Dashboard
2. Click the toggle button to collapse the stats aside
3. Verify the badge (letter T, G, E, S, M, W) is perfectly centered horizontally
4. Click the toggle again to expand
5. Verify the expanded layout still works correctly (badge + value + label aligned)

Expected results:
- ✅ Badge is centered in collapsed state
- ✅ Badge maintains square shape (28px x 28px)
- ✅ Letter is centered within the badge
- ✅ Expanded state works as before
- ✅ Transition animation is smooth

**Step 7: Commit the changes**

```bash
git add assets/input_main.css
git commit -m "$(cat <<'EOF'
fix(dashboard): center badge in collapsed stats aside

Add CSS rule to center stat items horizontally when the stats aside
is in collapsed state. Removes lateral padding to maximize available
space and uses flexbox justify-content for perfect centering.

Fixes issue where the badge appeared uncentered and stretched in
the collapsed aside view.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

This implementation plan addresses the badge centering issue with a minimal CSS change:

- **Single file modified:** `assets/input_main.css`
- **Lines of code added:** 6 (1 comment + 5 CSS rules)
- **No breaking changes:** Existing expanded state behavior unchanged
- **No component changes:** Pure CSS fix, no Rust/RSX modifications

The fix uses flexbox's `justify-content: center` property combined with zero lateral padding to ensure the badge is perfectly centered in the collapsed aside's limited 52px width.
