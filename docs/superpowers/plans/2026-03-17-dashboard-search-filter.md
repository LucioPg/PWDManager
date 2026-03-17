# Dashboard Search Filter — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a client-side search input to the Dashboard controls bar that filters the password table by name, combining with the existing strength filter.

**Architecture:** A new `search_query` signal in `dashboard.rs` feeds into the existing `page_data` `use_memo`, which applies both the text filter (case-insensitive substring on `name`) and the strength filter (from StatsAside) via AND logic. Pagination resets to page 1 when the search query changes. CSS classes in `input_main.css` style the input with a search icon and clear button.

**Tech Stack:** Dioxus 0.7 (signals, use_memo, RSX), DaisyUI 5 (input classes), Tailwind CSS

---

### Task 1: Add CSS classes for the search input

**Files:**
- Modify: `assets/input_main.css`

- [ ] **Step 1: Add search input CSS classes**

Add the following CSS at an appropriate location in `assets/input_main.css` (after the existing `.content-container` block or in a new section):

```css
/* === Search Input === */
.pwd-search-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.pwd-search-wrapper svg {
  position: absolute;
  left: 0.75rem;
  width: 1rem;
  height: 1rem;
  pointer-events: none;
  opacity: 0.5;
}

.pwd-search-input {
  padding-left: 2.25rem;
  padding-right: 2.25rem;
  width: 14rem;
}

.pwd-search-clear {
  position: absolute;
  right: 0.5rem;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.25rem;
  display: flex;
  align-items: center;
  opacity: 0.5;
  transition: opacity 0.15s;
}

.pwd-search-clear:hover {
  opacity: 0.8;
}
```

Note: check if `input-bordered` and `input-sm` (or similar DaisyUI input classes) exist in `input_main.css` already. If not, add them to the appropriate `@layer` so Tailwind generates the styles. Search for `input` in `input_main.css` first.

- [ ] **Step 2: Verify build**

Run: `dx build --desktop 2>&1 | tail -20`
Expected: Build succeeds (CSS is embedded via `include_str!`)

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "style: add CSS classes for dashboard search input"
```

---

### Task 2: Add search signal and filter logic in dashboard.rs

**Files:**
- Modify: `src/components/features/dashboard.rs`

- [ ] **Step 1: Add the search_query signal**

After line 62 (`let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());`), add:

```rust
    // Search query per filtro client-side
    let mut search_query = use_signal(|| String::new());
```

- [ ] **Step 2: Update the page_data use_memo to apply name filtering**

Replace the existing `page_data` use_memo (lines 103-114) with:

```rust
    // Paginazione locale: filtro search + strength, poi slice
    let page_data = use_memo(move || {
        let page = pagination.current_page();
        let page_size = pagination.page_size();
        let query = search_query();
        let query_lower = query.to_lowercase();
        let all = all_passwords();

        // Filtro per nome (case-insensitive) + strength (AND)
        let filtered: Vec<StoredRawPassword> = all
            .into_iter()
            .filter(|p| {
                let name_match = query_lower.is_empty()
                    || p.name.to_lowercase().contains(&query_lower);
                name_match
            })
            .collect();

        // Aggiorna total_count con i risultati filtrati
        pagination.total_count.set(filtered.len() as u64);

        let start = page * page_size;
        let end = (start + page_size).min(filtered.len());
        if start < filtered.len() {
            Some(filtered[start..end].to_vec())
        } else {
            Some(Vec::new())
        }
    });
```

**Key detail:** `total_count` is set inside the memo so the pagination component always reflects the filtered count. The strength filter from StatsAside is already handled by the SQL query (the `sorted_passwords_resource` only fetches passwords matching the active strength range), so the client-side filter only needs to handle the search query.

- [ ] **Step 3: Remove the total_count update from the use_effect**

In the existing `use_effect` (lines 95-100), remove the `pagination.total_count.set(data.len() as u64);` line since `total_count` is now managed inside `page_data`:

```rust
    // Aggiorna all_passwords quando la resource completa
    use_effect(move || {
        if let Some(data) = sorted_passwords_resource.read().as_ref() {
            all_passwords.set(data.clone());
        }
    });
```

- [ ] **Step 4: Add search input element in the controls bar RSX**

In the RSX, modify the controls bar `div` (line 227) to include the search input before the Combobox. The current layout is:

```rust
div { class: "flex flex-row justify-between",
    Combobox::<TableOrder> { ... }
    div { class: "flex flex-row gap-3 mb-4 justify-end align-center",
        button { class: "btn btn-success", ... }
    }
}
```

Change it to:

```rust
div { class: "flex flex-row justify-between items-center",
    div { class: "flex flex-row items-center gap-3",
        // Search input
        div { class: "pwd-search-wrapper",
            svg {
                class: "pwd-search-icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                path { d: "M21 21l-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z" }
            }
            input {
                class: "input input-bordered input-sm pwd-search-input",
                r#type: "text",
                placeholder: "Cerca per nome...",
                value: "{search_query}",
                oninput: move |e| {
                    let value = e.value();
                    search_query.set(value.clone());
                    pagination.go_to_page(0);
                },
            }
            if !search_query.read().is_empty() {
                button {
                    class: "pwd-search-clear",
                    onclick: move |_| {
                        search_query.set(String::new());
                        pagination.go_to_page(0);
                    },
                    svg {
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        path { d: "M18 6L6 18M6 6l12 12" }
                    }
                }
            }
        }
        // Sort Combobox
        Combobox::<TableOrder> {
            options: options.clone(),
            placeholder: "Order by".to_string(),
            on_change: move |v| {
                current_table_order.set(v);
                pagination.go_to_page(0);
                sorted_passwords_resource.restart();
            },
        }
    }
    div { class: "flex flex-row gap-3 mb-4 justify-end align-center",
        button {
            class: "btn btn-success",
            r#type: "button",
            onclick: move |_| {
                stored_password_dialog_state.current_stored_raw_password.set(None);
                stored_password_dialog_state.is_open.set(true);
            },
            "New Password"
        }
    }
}
```

**Important:** Before writing this RSX, check that `input-bordered` and `input-sm` classes exist in `input_main.css`. If not, add them to ensure Tailwind generates the styles. Run `grep -n "input-bordered\|input-sm\|input-" assets/input_main.css` to check.

- [ ] **Step 5: Verify build**

Run: `dx build --desktop 2>&1 | tail -20`
Expected: Build succeeds

- [ ] **Step 6: Manual test**

Run: `dx serve --desktop`
Verify:
1. Search input appears to the left of the sort Combobox
2. Typing text filters the table by name (case-insensitive)
3. Clear button (X) appears when text is present and resets the filter
4. Clicking a strength filter in StatsAside combines with the search query (AND)
5. Pagination resets to page 1 when search query changes
6. Pagination shows correct count for filtered results

- [ ] **Step 7: Commit**

```bash
git add src/components/features/dashboard.rs
git commit -m "feat: add search input filter to dashboard table"
```
