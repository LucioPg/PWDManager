# Dashboard Pagination Design

**Data:** 2026-03-05
**Branch:** dev-dashboard-pagination-38
**Approccio:** Paginazione SQL con LIMIT/OFFSET + Cache lato client

## Obiettivo

Migliorare le performance della Dashboard quando sono presenti numerose password, implementando:
- Paginazione classica (Previous/1/2/3/Next)
- Lazy loading con cache
- Stats globali sempre aggiornate
- Reset paginazione su cambio filtro

## Requisiti

| Requisito | Dettaglio |
|-----------|-----------|
| UI | Paginazione classica con controlli Previous/1/2/3/Next |
| Caricamento | Lazy loading con cache lato client |
| Stats | Sempre aggiornate (query separata) |
| Filtro | Reset a pagina 1 quando cambia |
| Invalidazione | Manuale dopo create/update/delete |
| Scala | Centinaia di password (page size: 20) |

## Architettura

```
┌─────────────────────────────────────────────────────────────────┐
│                        Dashboard                                 │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────────┐  ┌─────────────────┐   │
│  │ StatsAside   │  │ PasswordTable    │  │ Pagination      │   │
│  │ (sempre      │  │ (pagina corrente)│  │ Controls        │   │
│  │  fresche)    │  └────────┬─────────┘  └────────┬────────┘   │
│  └──────┬───────┘           │                     │            │
│         │                   │                     │            │
│         ▼                   ▼                     ▼            │
│  ┌──────────────────────────────────────────────────────┐      │
│  │              PaginationState                         │      │
│  │  - current_page: usize                               │      │
│  │  - page_size: usize (default: 20)                    │      │
│  │  - active_filter: Option<PasswordStrength>           │      │
│  │  - cache: HashMap<(filter, page), Vec<Password>>     │      │
│  │  - total_count: usize                                │      │
│  └────────────────────────┬─────────────────────────────┘      │
└───────────────────────────┼─────────────────────────────────────┘
                            │
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Database Layer                             │
├───────────────────────────────────────────────────────────────┤
│  fetch_passwords_paginated(user_id, filter, limit, offset)    │
│  fetch_password_stats(user_id) → PasswordStats                │
└───────────────────────────────────────────────────────────────┘
```

## Data Flow

### Al mount della Dashboard
1. Carica stats (query COUNT separata, sempre fresca)
2. Carica pagina 0 (nessun filtro)
3. Popola cache con `(None, 0) → Vec<Password>`

### Cambio pagina
1. Controlla cache per `(filter, page)`
2. Se presente → usa cache
3. Se assente → query DB, aggiungi a cache

### Cambio filtro
1. Reset `current_page = 0`
2. Mantiene cache per altri filtri
3. Carica pagina 0 del nuovo filtro

### Operazione CRUD (create/update/delete)
1. `cache.clear()` — invalida tutta la cache
2. Ricarica stats (per aggiornare conteggi)
3. Ricarica pagina corrente

## Database Layer

### Query paginata

```rust
pub async fn fetch_passwords_paginated(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredRawPassword>, u64), DBError>
```

Utilizza `sqlx-template::find_page((offset, limit, count), pool)` che restituisce `(Vec<T>, Page, u64)`.

### Mappatura filtro → score range

| Strength | Score Range |
|----------|-------------|
| WEAK | 0-49 |
| MEDIUM | 50-69 |
| STRONG | 70-84 |
| EPIC | 85-95 |
| GOD | 96-100 |

### Stats query

```rust
pub async fn fetch_password_stats(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<PasswordStats, DBError>
```

Query COUNT con GROUP BY score_range per ottenere conteggi per ogni strength.

## Frontend

### PaginationState

```rust
#[derive(Clone, Default)]
pub struct PaginationState {
    pub current_page: Signal<usize>,
    pub page_size: Signal<usize>,
    pub active_filter: Signal<Option<PasswordStrength>>,
    pub cache: Signal<HashMap<CacheKey, Vec<StoredRawPassword>>>,
    pub total_count: Signal<u64>,
    pub is_loading: Signal<bool>,
}
```

### PaginationControls Component

Componente DaisyUI con `join` per i bottoni:
- Previous («) e Next (»)
- Max 5 numeri di pagina visibili
- Display 1-indexed per l'utente
- Loading state disabilita i controlli

## File da modificare/creare

### Nuovi file
- `src/components/globals/pagination/mod.rs` — modulo pagination
- `src/components/globals/pagination/pagination_controls.rs` — UI component
- `src/components/globals/pagination/pagination_state.rs` — stato e cache

### File modificati
- `src/backend/db_backend.rs` — aggiungere funzioni paginated
- `src/components/features/dashboard.rs` — integrare paginazione
- `src/components/globals/mod.rs` — export nuovo modulo
- `src/components/globals/stats_aside.rs` — usare stats query separata

## Stili DaisyUI

- `.join` — contenitore bottoni collegati
- `.join-item` — singolo bottone nel gruppo
- `.btn-active` — bottone pagina corrente
