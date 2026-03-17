# Dashboard Search Filter — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a client-side search input to the Dashboard controls bar that filters the password table by name, combining with the existing strength filter.

**Architecture:** A new `search_query` signal in `dashboard.rs` feeds into a new `filtered_passwords` `use_memo` (pure computation, no side effects), which applies the text filter (case-insensitive substring on `name`). A separate `use_effect` syncs `total_count` from the filtered results. The existing `page_data` `use_memo` then slices the filtered data for pagination. CSS classes in `input_main.css` (all prefixed `pwd-`) style the input with a search icon and clear button. Responsive layout uses `flex-wrap` to prevent overflow on small screens.

**Tech Stack:** Dioxus 0.7 (signals, use_memo, use_effect, RSX), DaisyUI 5 (input classes), Tailwind CSS v4

---

### Task 1: Add CSS classes for the search input

**Files:**
- Modify: `assets/input_main.css` (append at end of file, after line 1815)

- [ ] **Step 1: Add search input CSS classes**

Append the following CSS at the end of `assets/input_main.css`. All classes use the `pwd-` prefix per project convention. The search input uses `width: 100%` inside its wrapper so it can grow/shrink with `flex-wrap`.

```css
/* === Search Input (Dashboard) === */
.pwd-search-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 14rem;
    flex-shrink: 1;
}

.pwd-search-icon {
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
    width: 100%;
    font-size: 0.875rem;
    height: 2rem;
}

.pwd-search-input:focus {
    outline: 2px solid theme('colors.primary');
    outline-offset: -2px;
    border-color: transparent;
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
    justify-content: center;
    opacity: 0.5;
    transition: opacity 0.15s;
    color: currentColor;
}

.pwd-search-clear:hover {
    opacity: 0.8;
}

/* Controls bar: flex-wrap per responsività */
.pwd-controls-bar {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
}

.pwd-controls-left {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
}

@media (max-width: 640px) {
    .pwd-search-wrapper {
        width: 100%;
    }
}
```

**Note on DaisyUI classes:** `input-bordered` and `input-sm` are DaisyUI 5 utility classes. Since `input_main.css` uses `@source "../src/**/*.rs"`, Tailwind will scan the Rust source and generate these classes automatically when they appear in the RSX. No manual addition to `input_main.css` is needed for them.

- [ ] **Step 2: Verify build**

Run: `dx build --desktop 2>&1 | tail -20`
Expected: Build succeeds (CSS is embedded via `include_str!`)

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "style: add CSS classes for dashboard search input"
```

---

### Task 2: Add search signal, filter logic, and RSX in dashboard.rs

**Files:**
- Modify: `src/components/features/dashboard.rs`

- [ ] **Step 1: Add the search_query signal**

After line 62 (`let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());`), add:

```rust
    // Search query per filtro client-side
    let mut search_query = use_signal(|| String::new());
```

- [ ] **Step 2: Add a filtered_passwords memo (pure computation)**

After the existing `use_effect` that sets `all_passwords` (lines 95-100), add a new `use_memo` that computes the filtered list. This is a **pure computation** — no side effects:

```rust
    // Filtro per nome (case-insensitive). Computazione pura.
    let filtered_passwords = use_memo(move || {
        let query = search_query();
        let query_lower = query.to_lowercase();
        let all = all_passwords();

        if query_lower.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|p| p.name.to_lowercase().contains(&query_lower))
                .collect()
        }
    });
```

**Why a separate memo:** Keeping the filter logic separate from the pagination slice makes each memo focused on one responsibility. The `page_data` memo will read `filtered_passwords()` instead of `all_passwords()`.

- [ ] **Step 3: Add a use_effect to sync total_count from filtered results**

After the new memo, add a `use_effect` to sync `total_count` — this keeps side effects out of `use_memo`:

```rust
    // Sync total_count con i risultati filtrati
    use_effect(move || {
        let count = filtered_passwords().len();
        pagination.total_count.set(count as u64);
    });
```

- [ ] **Step 4: Update page_data to use filtered_passwords instead of all_passwords**

Replace the existing `page_data` `use_memo` (lines 103-114):

```rust
    // Paginazione locale: slice dei dati filtrati
    let page_data = use_memo(move || {
        let page = pagination.current_page();
        let page_size = pagination.page_size();
        let filtered = filtered_passwords();

        let start = page * page_size;
        let end = (start + page_size).min(filtered.len());
        if start < filtered.len() {
            Some(filtered[start..end].to_vec())
        } else {
            Some(Vec::new())
        }
    });
```

- [ ] **Step 5: Remove the total_count update from the use_effect on line 95-100**

The existing `use_effect` that copies resource data into `all_passwords` also sets `total_count`. Remove that line since `total_count` is now synced via the new `use_effect` in Step 3:

```rust
    // Aggiorna all_passwords quando la resource completa
    use_effect(move || {
        if let Some(data) = sorted_passwords_resource.read().as_ref() {
            all_passwords.set(data.clone());
        }
    });
```

- [ ] **Step 6: Replace the controls bar RSX with search input + responsive layout**

Replace the controls bar `div` (line 227-248). The current layout is a single `flex-row` with Combobox on the left and button on the right. Replace it with a `pwd-controls-bar` wrapper that uses `flex-wrap` for responsiveness:

```rust
            div { class: "pwd-controls-bar",
                div { class: "pwd-controls-left",
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
                                search_query.set(value);
                                pagination.go_to_page(0);
                            },
                        }
                        if !search_query().is_empty() {
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
```

**Key differences from the old layout:**
- `pwd-controls-bar` replaces `flex flex-row justify-between` — adds `flex-wrap` so items flow to next row on small screens
- `pwd-controls-left` wraps search + Combobox — they stay together
- "New Password" button is now a direct child of the bar (no extra wrapper div needed since `flex-wrap` handles alignment)
- `search_query()` used consistently (not `.read()`) — matches the existing codebase style
- Removed `mb-4` from the button wrapper — no longer needed with flex-wrap layout

- [ ] **Step 7: Verify build**

Run: `dx build --desktop 2>&1 | tail -20`
Expected: Build succeeds

- [ ] **Step 8: Manual test**

Run: `dx serve --desktop`
Verify:
1. Search input appears to the left of the sort Combobox
2. Typing text filters the table by name (case-insensitive)
3. Clear button (X) appears when text is present and resets the filter
4. Clicking a strength filter in StatsAside combines with the search query (AND)
5. Pagination resets to page 1 when search query changes
6. Pagination shows correct count for filtered results
7. On small screens (<640px), controls wrap to multiple rows without overflow
