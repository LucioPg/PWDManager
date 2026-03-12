# Design: Ordinamento Frontend Tabella Password

**Data:** 2026-03-12
**Stato:** Approvato
**Approccio:** Fetch completa + ordinamento locale

## Contesto

La tabella della dashboard mostra le password utente con paginazione. Attualmente l'ordinamento è fisso (`ORDER BY created_at DESC` nel database). L'utente vuole poter ordinare per:
- **A-Z** / **Z-A**: ordinamento alfabetico sulla colonna `location`
- **Oldest** / **Newest**: ordinamento per data creazione

**Vincolo critico:** Il campo `location` è cifrato nel database, quindi l'ordinamento A-Z/Z-A deve avvenire lato frontend sui dati decifrati.

## Architettura

### 1. Nuova Funzione di Fetch Completa

**File:** `src/backend/password_utils.rs`

```rust
pub async fn get_all_stored_raw_passwords(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
) -> Result<Vec<StoredRawPassword>, DBError>
```

Recupera **tutte** le password dell'utente (rispetto al filtro strength), le decifra e restituisce un `Vec<StoredRawPassword>`.

### 2. Logica Ordinamento in `TableOrder`

**File:** `src/components/globals/types.rs`

```rust
impl TableOrder {
    pub fn sort(&self, passwords: &mut [StoredRawPassword]) {
        match self {
            TableOrder::AZ => passwords.sort_by(|a, b| a.location.cmp(&b.location)),
            TableOrder::ZA => passwords.sort_by(|a, b| b.location.cmp(&a.location)),
            TableOrder::Oldest => passwords.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            TableOrder::Newest => passwords.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        }
    }
}
```

### 3. Modifiche alla Dashboard

**File:** `src/components/features/dashboard.rs`

#### Nuovi Signal

```rust
// Dati completi (non paginati)
let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());

// Stato ordinamento in corso
let mut is_sorting = use_signal(|| false);

// Default: Newest (coincide con ORDER BY created_at DESC del DB)
let mut current_table_order = use_signal(|| Some(TableOrder::Newest));

// Flag per triggerare refetch
let mut needs_refetch = use_signal(|| true);
```

#### Flusso Reattivo

`use_effect` che reagisce a `current_table_order` e `needs_refetch`:

1. Se `is_sorting()`, esci (evita re-entrancy)
2. Imposta `is_sorting = true`
3. Fetch completa con `get_all_stored_raw_passwords`
4. Se errore → toast, esci
5. Applica `TableOrder::sort()` ai dati
6. Aggiorna `all_passwords`
7. Reset paginazione a pagina 1
8. Imposta `is_sorting = false`, `needs_refetch = false`

#### Paginazione Locale

```rust
let page_data = use_memo(move || {
    let page = pagination.current_page();
    let page_size = pagination.page_size();
    let all = all_passwords();
    let start = page * page_size;
    all.get(start..start + page_size)
        .map(|s| s.to_vec())
});
```

### 4. UI e Spinner

```rust
if is_sorting() {
    rsx! {
        div { class: "flex justify-center py-8",
            Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }
        }
    }
} else {
    StoredRawPasswordsTable {
        data: page_data(),
        // ... altri props
    }
}
```

### 5. Integrazione con Filtri

Quando l'utente clicca su una statistica (filtro strength):

```rust
on_stat_click: move |strength| {
    pagination.set_filter(strength);
    needs_refetch.set(true);
}
```

Il `use_effect` reagirà a `needs_refetch` e rifarà la fetch con il nuovo filtro.

### 6. Gestione Errori

- Errori durante fetch → toast con messaggio dettagliato
- `all_passwords` rimane vuoto
- `is_sorting = false` per rimuovere lo spinner

## Flusso Dati

```
┌─────────────────┐
│  Combobox       │ ──on_change──> current_table_order.set(v)
│  (A-Z/Z-A/...)  │
└─────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│ use_effect                                                  │
│  1. is_sorting.set(true)                                    │
│  2. get_all_stored_raw_passwords(pool, user_id, filter)     │
│  3. TableOrder::sort(&mut passwords)                        │
│  4. all_passwords.set(passwords)                            │
│  5. pagination.go_to_page(0)                                │
│  6. is_sorting.set(false)                                   │
└─────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│ use_memo (page_data)                                        │
│  - Prende slice di all_passwords per pagina corrente        │
└─────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│ StoredRawPasswordsTable                                     │
│  - Renderizza page_data()                                   │
└─────────────────────────────────────────────────────────────┘
```

## File Coinvolti

| File | Modifica |
|------|----------|
| `src/components/globals/types.rs` | Aggiungere `impl TableOrder` con metodo `sort()` |
| `src/backend/password_utils.rs` | Aggiungere `get_all_stored_raw_passwords()` |
| `src/components/features/dashboard.rs` | Refactor per paginazione locale + ordinamento |

## Note Tecniche

- **Prima fetch:** All'avvio della dashboard, `needs_refetch` è `true`, quindi viene triggerata la fetch completa con ordinamento default (Newest)
- **Cache paginazione:** La cache esistente in `PaginationState` potrebbe non essere più necessaria, valutare rimozione
- **Performance:** Con migliaia di password, la fetch completa potrebbe essere lenta. Considerare in futuro un limite massimo o lazy loading
