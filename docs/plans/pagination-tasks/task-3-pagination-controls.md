# Task 3: PaginationControls UI

> **Status:** ✅ COMPLETATO (2026-03-06)
>
> **Commit:** `dd42e25` - feat(pagination): add PaginationControls component

**Goal:** Creare il componente UI per i controlli di paginazione.

**Architecture:** Componente DaisyUI con `join` per bottoni collegati. Mostra Previous, numeri di pagina (max 5), Next.

**Tech Stack:** Rust, Dioxus 0.7, DaisyUI 5

**Dipendenze:** Task 2 (richiede `PaginationState` da `src/components/globals/pagination/`)

---

## Modifiche Extra Risolte Durante Implementazione

Durante l'implementazione sono emersi problemi nel Task 2 che sono stati risolti:

1. **Modulo pagination non esportato** - Aggiunto `pub mod pagination;` e `pub use pagination::*;` in `src/components/globals/mod.rs`
2. **Getter mancanti in PaginationState** - Aggiunti metodi `current_page()`, `page_size()`, `active_filter()`, `total_count()`, `is_loading()`
3. **PartialEq mancante** - Aggiunto `#[derive(PartialEq)]` a `PaginationState` (richiesto dalla macro `#[component]` di Dioxus)
4. **Hash + Eq su PasswordStrength** - Richiesto aggiornamento del crate esterno `pwd-types` per permettere l'uso di `HashMap<CacheKey, ...>`

---

## Files

- **Create:** `src/components/globals/pagination/pagination_controls.rs`
- **Reference:** `docs/DaisyUI/` per componenti join/btn

---

## Step 1: Creare il file

```bash
touch src/components/globals/pagination/pagination_controls.rs
```

---

## Step 2: Aggiornare mod.rs del modulo pagination

Verificare che `src/components/globals/pagination/mod.rs` contenga il modulo `pagination_controls`. Aggiungere se mancante:

```rust
mod pagination_controls;

pub use pagination_controls::PaginationControls;
```

**Nota:** Questo file è stato creato in Task 2. Qui si aggiunge solo il nuovo modulo.

---

## Step 3: Implementare PaginationControls

In `src/components/globals/pagination/pagination_controls.rs`:

```rust
use super::PaginationState;
use dioxus::prelude::*;

/// Calcola le pagine visibili nei controlli.
///
/// Mostra max 5 numeri di pagina, centrati sulla pagina corrente.
/// Esempio: se current=5, total=10 → [3, 4, 5, 6, 7]
fn calculate_visible_pages(current: usize, total: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }

    if total <= 5 {
        return (0..total).collect();
    }

    // Centra la pagina corrente, ma rispetta i limiti
    let start = current.saturating_sub(2);
    let end = (start + 5).min(total);
    let start = end.saturating_sub(5);

    (start..end).collect()
}

/// Controlli di paginazione con DaisyUI.
///
/// Visualizza: [«] [1] [2] [3] [4] [5] [»]
/// + info "Page X of Y (Z items)"
#[component]
pub fn PaginationControls(
    /// Stato di paginazione (da context o props)
    pagination: PaginationState,
    /// Callback chiamato quando si cambia pagina
    on_page_change: Callback<usize>,
) -> Element {
    let current_page = pagination.current_page();
    let total_pages = pagination.total_pages();
    let total_count = pagination.total_count();
    let is_loading = pagination.is_loading();

    let visible_pages = calculate_visible_pages(current_page, total_pages);
    let has_prev = pagination.has_prev();
    let has_next = pagination.has_next();

    rsx! {
        div { class: "pwd-pagination-container mt-4",
            // Controlli paginazione
            div { class: "join justify-center w-full flex",
                // Previous button
                button {
                    class: "join-item btn btn-sm",
                    disabled: !has_prev || is_loading,
                    onclick: move |_| {
                        if has_prev {
                            on_page_change.call(current_page.saturating_sub(1));
                        }
                    },
                    "«"
                }

                // Page numbers
                for page_num in visible_pages {
                    {
                        let is_current = page_num == current_page;
                        let page_for_closure = page_num;
                        rsx! {
                            button {
                                class: if is_current {
                                    "join-item btn btn-sm btn-primary"
                                } else {
                                    "join-item btn btn-sm"
                                },
                                disabled: is_loading,
                                onclick: move |_| {
                                    on_page_change.call(page_for_closure);
                                },
                                "{page_num + 1}"  // Display 1-indexed
                            }
                        }
                    }
                }

                // Next button
                button {
                    class: "join-item btn btn-sm",
                    disabled: !has_next || is_loading,
                    onclick: move |_| {
                        if has_next {
                            on_page_change.call(current_page + 1);
                        }
                    },
                    "»"
                }
            }

            // Page info
            div { class: "text-center text-sm mt-2 opacity-70",
                if total_pages > 0 {
                    "Page {current_page + 1} of {total_pages} ({total_count} items)"
                } else {
                    "No items"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_visible_pages_empty() {
        assert_eq!(calculate_visible_pages(0, 0), Vec::<usize>::new());
    }

    #[test]
    fn test_calculate_visible_pages_single_page() {
        // Singola pagina → [0]
        assert_eq!(calculate_visible_pages(0, 1), vec![0]);
    }

    #[test]
    fn test_calculate_visible_pages_exactly_5() {
        // Esattamente 5 pagine → mostra tutte
        assert_eq!(calculate_visible_pages(0, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(calculate_visible_pages(2, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(calculate_visible_pages(4, 5), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_calculate_visible_pages_less_than_5() {
        assert_eq!(calculate_visible_pages(0, 3), vec![0, 1, 2]);
        assert_eq!(calculate_visible_pages(2, 4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_calculate_visible_pages_middle() {
        // Pagina 5 di 10 → [3, 4, 5, 6, 7]
        assert_eq!(calculate_visible_pages(5, 10), vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_calculate_visible_pages_near_start() {
        // Pagina 1 di 10 → [0, 1, 2, 3, 4]
        assert_eq!(calculate_visible_pages(1, 10), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_calculate_visible_pages_near_end() {
        // Pagina 9 di 10 → [5, 6, 7, 8, 9]
        assert_eq!(calculate_visible_pages(9, 10), vec![5, 6, 7, 8, 9]);
    }
}
```

---

## Step 4: Verificare compilazione

```bash
cargo check
```

**Expected:** Nessun errore.

---

## Step 5: Eseguire test

```bash
cargo test calculate_visible_pages
```

**Expected:** 5 test passati.

---

## Step 6: Commit

```bash
git add src/components/globals/pagination/
git commit -m "feat(pagination): add PaginationControls component"
```

---

## Merge Instructions

```bash
git checkout dev-dashboard-pagination-38
git merge task-3-pagination-controls --no-ff -m "Merge task-3: pagination controls UI"
git branch -d task-3-pagination-controls
```

---

## Notes

- DaisyUI `join` e `join-item` sono classi built-in per gruppi di bottoni
- `btn-primary` evidenzia il bottone della pagina corrente
- Display 1-indexed per l'utente (Page 1, non Page 0)
- I test verificano la logica di calcolo pagine visibili (inclusi edge case: 0, 1, 5 pagine)
- **Signal API**: `pagination.current_page()` è il pattern corretto per tipi `Copy` in Dioxus 0.7

---

## Risultati Verifica

| Step | Risultato |
|------|-----------|
| Step 1: Creare file | ✅ Completato |
| Step 2: Aggiornare mod.rs | ✅ Completato |
| Step 3: Implementare componente | ✅ Completato |
| Step 4: Verificare compilazione | ✅ Nessun errore |
| Step 5: Eseguire test | ✅ 7/7 passati |
| Step 6: Commit | ✅ `dd42e25` |

**Test output:**
```
running 7 tests
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_empty ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_exactly_5 ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_less_than_5 ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_middle ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_near_end ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_single_page ... ok
test components::globals::pagination::pagination_controls::tests::test_calculate_visible_pages_near_start ... ok

test result: ok. 7 passed; 0 failed
```
