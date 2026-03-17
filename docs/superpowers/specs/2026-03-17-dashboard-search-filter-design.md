# Dashboard Search Filter — Design Spec

## Summary

Add a text search input to the Dashboard controls bar that filters the password table by name. The filter operates client-side on the already-decrypted data, combining with the existing strength filter via AND logic.

## Motivation

Users need to quickly find specific passwords by name without scrolling through the full list. The current UI only supports sorting (A-Z, Z-A, Oldest, Newest) and strength-based filtering via the StatsAside sidebar.

## Requirements

- Search input positioned inline in the controls bar, to the left of the sort Combobox
- Case-insensitive substring match on the `name` field of `StoredRawPassword`
- Combined with the existing strength filter (AND logic): if both filters are active, a password must match both
- Clear button (X) to reset the search query
- Pagination resets to page 1 when the search query changes
- Blue border on focus to indicate active state

## Non-Goals

- Server-side filtering (SQL LIKE) — not needed for typical password manager datasets
- Highlighting matched text in table rows
- Debounce — `use_memo` reactivity handles this naturally
- Searching on encrypted fields (username, URL, password, notes)

## Architecture

### Data Flow

```
search_query signal (String)
        │
        ▼
page_data use_memo
  ├── reads all_passwords signal
  ├── filters by search_query (case-insensitive substring on name)
  ├── filters by pagination.filter (strength range from StatsAside)
  └── slices by current page
        │
        ▼
StoredRawPasswordsTable { data: filtered_page }
```

### Files Modified

1. **`src/components/features/dashboard.rs`**
   - Add `search_query: Signal<String>` signal
   - Update `page_data` `use_memo` to filter `all_passwords` by `search_query` (lowercase comparison on `name`)
   - Reset `pagination.current_page` to 1 when `search_query` changes
   - Add search input element in the controls bar RSX (left of the Combobox)

2. **`assets/input_main.css`**
   - Add `.pwd-search-input` class for the input styling
   - Add `.pwd-search-clear` class for the clear button
   - Add `.pwd-search-input:focus` state with blue border
   - Add `.pwd-search-wrapper` for the relative-positioned container (icon + input + clear)

### Filter Logic (pseudocode)

```
filtered = all_passwords.iter().filter(|p| {
    let name_match = search_query.is_empty()
        || p.name.to_lowercase().contains(&search_query.to_lowercase());
    let strength_match = strength_filter match {
        None => true,
        Some(range) => p.score in range,
    };
    name_match && strength_match
}).collect()
```

## UI Layout

```
┌──────────────────────────────────────────────────────┐
│  🔍 [Cerca per nome...  ✕]  [Più recenti ▾]  [+ Nuova Password]  │
└──────────────────────────────────────────────────────┘
```

- Search input: ~200px wide, with magnifying glass icon on the left, clear (X) button on the right (visible only when text is present)
- DaisyUI 5 `input` classes for base styling, custom `.pwd-search-*` classes for specific behavior

## Constraints

- Only the `name` field is searchable (it is the only non-encrypted field)
- Client-side filtering is sufficient: password managers typically handle < 500 entries
- No backend changes required
