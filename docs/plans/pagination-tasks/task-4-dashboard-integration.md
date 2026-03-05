# Task 4: Dashboard Integration

> **Per Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrare la paginazione nella Dashboard esistente.

**Architecture:** Sostituire `use_resource` che carica tutto con uno che carica pagine. Aggiungere `PaginationControls`. Gestire invalidazione cache dopo CRUD.

**Tech Stack:** Rust, Dioxus 0.7, Signal

**Dipendenze:** Task 1 (Database Layer), Task 2 (PaginationState), Task 3 (PaginationControls)

---

## Files

- **Modify:** `src/components/features/dashboard.rs`
- **Reference:** `src/components/features/dashboard.rs` (codice esistente)

---

## Step 1: Aggiungere imports

All'inizio di `src/components/features/dashboard.rs`, aggiungere:

```rust
use crate::backend::db_backend::{fetch_password_stats, fetch_passwords_paginated};
use crate::components::globals::pagination::{PaginationControls, PaginationState};
```

---

## Step 2: Sostituire stored_raw_passwords_data con paginazione

**CERCARE** (righe ~38-57):
```rust
let stored_raw_passwords_data = use_resource(move || {
    let pool_clone = pool.clone();
    async move {
        // ... codice esistente
    }
});
```

**SOSTITUIRE CON:**

```rust
// Stato paginazione
let mut pagination = use_context_provider(|| PaginationState::default());

// Resource per pagina corrente
let password_page_data = use_resource(move || {
    let pool = pool.clone();
    let page = *pagination.current_page.read();
    let filter = *pagination.active_filter.read();
    let page_size = *pagination.page_size.read();

    async move {
        // Controlla cache prima di fetchare
        if let Some(cached) = pagination.get_current_page_from_cache() {
            return Some(cached);
        }

        pagination.is_loading.set(true);
        let result = fetch_passwords_paginated(&pool, user_id, filter, page, page_size).await;
        pagination.is_loading.set(false);

        match result {
            Ok((passwords, total)) => {
                pagination.total_count.set(total);
                pagination.cache_page(filter, page, passwords.clone());
                Some(passwords)
            }
            Err(e) => {
                error.set(Some(e));
                None
            }
        }
    }
});
```

---

## Step 3: Aggiungere resource per stats

Dopo `password_page_data`, aggiungere:

```rust
// Stats sempre fresche (query separata)
let stats_data = use_resource(move || {
    let pool = pool.clone();
    async move {
        match fetch_password_stats(&pool, user_id).await {
            Ok(stats) => Some(stats),
            Err(e) => {
                error.set(Some(e));
                None
            }
        }
    }
});
```

---

## Step 4: Aggiornare stats memo

**CERCARE** (righe ~64-81):
```rust
let stats = use_memo(move || {
    // ... codice esistente che itera su stored_raw_passwords_data
});
```

**SOSTITUIRE CON:**

```rust
let stats = use_memo(move || {
    stats_data.read().clone().unwrap_or_default()
});
```

---

## Step 5: Rimuovere filtered_stored_raw_passwords

**ELIMINARE** il blocco (righe ~83-102):
```rust
let filtered_stored_raw_passwords = use_memo(move || {
    // ... tutto il blocco
});
```

Non serve più perché il filtraggio è fatto dal DB.

---

## Step 6: Aggiornare callback on_confirm_upsert

**CERCARE** (righe ~105-113):
```rust
let on_confirm_upsert = {
    let stored_raw_passwords_data = stored_raw_passwords_data.clone();
    move |_| {
        let mut resource = stored_raw_passwords_data.clone();
        spawn(async move {
            resource.restart();
        });
    }
};
```

**SOSTITUIRE CON:**

```rust
let on_confirm_upsert = {
    let mut pagination = pagination.clone();
    let mut stats_data = stats_data.clone();
    move |_| {
        pagination.invalidate();
        stats_data.restart();
        password_page_data.restart();
    }
};
```

---

## Step 7: Aggiornare callback on_confirm_delete

**CERCARE** nel blocco `on_confirm_delete` la parte che fa `resource.restart()` e aggiungere invalidazione:

```rust
// Dentro on_confirm_delete, dopo delete_stored_password success:
{
    pagination.invalidate();
    stats_data.restart();
    resource.restart();
    delete_state.is_open.set(false);
}
```

---

## Step 8: Aggiornare StatsAside callback

**CERCARE** (riga ~174):
```rust
on_stat_click: move |strength| current_filter.clone().set(strength),
```

**SOSTITUIRE CON:**

```rust
on_stat_click: move |strength| {
    pagination.set_filter(strength);
    password_page_data.restart();
},
```

---

## Step 9: Aggiornare table_data

**CERCARE** (righe ~199-207):
```rust
{
    let table_data = filtered_stored_raw_passwords();
    // ...
}
```

**SOSTITUIRE CON:**

```rust
{
    let table_data = password_page_data.read().clone();
    let count = table_data.as_ref().map(|p| p.len()).unwrap_or(0);
    rsx! {
        div { class: "card card-lg",
            StoredRawPasswordsTable { key: "{count}", data: table_data }
        }
    }
}
```

---

## Step 10: Aggiungere PaginationControls

Dopo il blocco della tabella, aggiungere:

```rust
// Controlli paginazione
PaginationControls {
    pagination: pagination.clone(),
    on_page_change: move |new_page| {
        pagination.go_to_page(new_page);
        password_page_data.restart();
    },
}
```

---

## Step 11: Rimuovere current_filter inutilizzato

Eliminare la riga:
```rust
let current_filter = use_signal(|| <Option<PasswordStrength>>::None);
```

---

## Step 12: Verificare compilazione

```bash
cargo check
```

**Expected:** Nessun errore. Risolvere eventuali warning.

---

## Step 13: Commit

```bash
git add src/components/features/dashboard.rs
git commit -m "feat(dashboard): integrate pagination with cache"
```

---

## Merge Instructions

```bash
git checkout dev-dashboard-pagination-38
git merge task-4-dashboard-integration --no-ff -m "Merge task-4: dashboard pagination integration"
git branch -d task-4-dashboard-integration
```

---

## Notes

- Il filtraggio ora avviene nel DB, non più lato client
- `pagination.set_filter()` resetta automaticamente a pagina 0
- Dopo CRUD: `invalidate()` + `restart()` su entrambe le resource
- `stats_data` è separato da `password_page_data` per avere stats sempre fresche
