# Task 2: PaginationState

> **Per Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Creare lo stato di paginazione con cache lato client.

**Architecture:** Struct `PaginationState` con Signal per reattività Dioxus. Cache come HashMap<(filter, page), Vec<Password>>.

**Tech Stack:** Rust, Dioxus 0.7, Signal

**Dipendenze:** Nessuna (task indipendente, può essere sviluppato in parallelo con Task 1 e 3)

---

## Files

- **Create:** `src/components/globals/pagination/pagination_state.rs`
- **Reference:** `MEMORY.md` per pattern Signal

---

## Step 1: Creare directory e file

```bash
mkdir -p src/components/globals/pagination
touch src/components/globals/pagination/pagination_state.rs
```

---

## Step 2: Implementare PaginationState

In `src/components/globals/pagination/pagination_state.rs`:

```rust
use dioxus::prelude::*;
use pwd_types::{PasswordStrength, StoredRawPassword};
use std::collections::HashMap;

/// Chiave della cache: (filtro attivo, numero pagina)
pub type CacheKey = (Option<PasswordStrength>, usize);

/// Stato globale della paginazione.
///
/// Gestisce pagina corrente, filtro, cache e loading state.
/// Tutti i campi sono Signal per la reattività Dioxus.
#[derive(Clone)]
pub struct PaginationState {
    /// Pagina corrente (0-indexed, display come 1-indexed)
    pub current_page: Signal<usize>,

    /// Dimensione pagina (default: 20)
    pub page_size: Signal<usize>,

    /// Filtro attivo per PasswordStrength
    pub active_filter: Signal<Option<PasswordStrength>>,

    /// Cache: (filter, page) → passwords
    /// Mantiene pagine già caricate per evitare re-fetch
    pub cache: Signal<HashMap<CacheKey, Vec<StoredRawPassword>>>,

    /// Totale risultati per il filtro corrente (dal DB)
    pub total_count: Signal<u64>,

    /// Loading state per mostrare spinner
    pub is_loading: Signal<bool>,
}

impl Default for PaginationState {
    fn default() -> Self {
        Self {
            current_page: Signal::new(0),
            page_size: Signal::new(20),
            active_filter: Signal::new(None),
            cache: Signal::new(HashMap::new()),
            total_count: Signal::new(0),
            is_loading: Signal::new(false),
        }
    }
}

impl PaginationState {
    /// Crea un nuovo stato con page_size personalizzato
    pub fn new(page_size: usize) -> Self {
        Self {
            page_size: Signal::new(page_size),
            ..Default::default()
        }
    }

    /// Invalida tutta la cache.
    ///
    /// Chiamare dopo operazioni CRUD (create/update/delete)
    /// per forzare il refresh dei dati.
    pub fn invalidate(&mut self) {
        self.cache.write().clear();
    }

    /// Imposta un nuovo filtro e resetta a pagina 0.
    ///
    /// NON invalida la cache - le pagine già caricate
    /// per altri filtri rimangono disponibili.
    pub fn set_filter(&mut self, filter: Option<PasswordStrength>) {
        if *self.active_filter.read() != filter {
            self.active_filter.set(filter);
            self.current_page.set(0);
        }
    }

    /// Vai a una pagina specifica.
    ///
    /// Non fa nulla se la pagina è fuori range o uguale alla corrente.
    pub fn go_to_page(&mut self, page: usize) {
        if page != *self.current_page.read() {
            self.current_page.set(page);
        }
    }

    /// Pagina successiva (se disponibile)
    pub fn next_page(&mut self) {
        let current = *self.current_page.read();
        let total_pages = self.total_pages();
        if current + 1 < total_pages {
            self.current_page.set(current + 1);
        }
    }

    /// Pagina precedente (se disponibile)
    pub fn prev_page(&mut self) {
        let current = *self.current_page.read();
        if current > 0 {
            self.current_page.set(current - 1);
        }
    }

    /// Calcola numero totale di pagine
    pub fn total_pages(&self) -> usize {
        let total = *self.total_count.read() as usize;
        let page_size = *self.page_size.read();
        if page_size == 0 {
            return 0;
        }
        (total + page_size - 1) / page_size
    }

    /// Verifica se può andare a pagina precedente
    pub fn has_prev(&self) -> bool {
        *self.current_page.read() > 0
    }

    /// Verifica se può andare a pagina successiva
    pub fn has_next(&self) -> bool {
        let current = *self.current_page.read();
        current + 1 < self.total_pages()
    }

    /// Ottiene la chiave cache corrente
    pub fn current_cache_key(&self) -> CacheKey {
        (*self.active_filter.read(), *self.current_page.read())
    }

    /// Verifica se la pagina corrente è in cache
    pub fn is_current_page_cached(&self) -> bool {
        let key = self.current_cache_key();
        self.cache.read().contains_key(&key)
    }

    /// Ottiene la pagina corrente dalla cache (se presente)
    pub fn get_current_page_from_cache(&self) -> Option<Vec<StoredRawPassword>> {
        let key = self.current_cache_key();
        self.cache.read().get(&key).cloned()
    }

    /// Salva una pagina in cache
    pub fn cache_page(&mut self, filter: Option<PasswordStrength>, page: usize, passwords: Vec<StoredRawPassword>) {
        let key = (filter, page);
        self.cache.write().insert(key, passwords);
    }
}
```

---

## Step 3: Verificare compilazione

```bash
cargo check
```

**Expected:** Nessun errore.

---

## Step 4: Commit

```bash
git add src/components/globals/pagination/pagination_state.rs
git commit -m "feat(pagination): add PaginationState with cache"
```

---

## Merge Instructions

```bash
git checkout dev-dashboard-pagination-38
git merge task-2-pagination-state --no-ff -m "Merge task-2: pagination state"
git branch -d task-2-pagination-state
```

---

## Notes

- ⚠️ **IMPORTANTE**: Tutti i Signal devono essere `mut` anche se il compilatore suggerisce il contrario (vedi MEMORY.md)
- La cache usa `HashMap` con chiave `(filter, page)` per supportare più filtri contemporaneamente
- `invalidate()` pulisce tutta la cache, ma `set_filter()` mantiene le pagine di altri filtri
