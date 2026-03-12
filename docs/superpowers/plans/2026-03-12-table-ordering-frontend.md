# Table Ordering Frontend Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementare ordinamento A-Z/Z-A/Oldest/Newest lato frontend per la tabella password della dashboard.

**Architecture:** Fetch completa di tutti i dati, ordinamento in memoria, paginazione locale. Questo approccio è necessario perché il campo `location` è cifrato nel database.

**Tech Stack:** Rust, Dioxus 0.7, SQLx

---

## File Structure

| File | Azione | Responsabilità |
|------|--------|----------------|
| `src/components/globals/types.rs` | Modifica | Aggiungere metodo `sort()` a `TableOrder` |
| `src/backend/db_backend.rs` | Modifica | Aggiungere `fetch_all_passwords_for_user_with_filter()` |
| `src/backend/password_utils.rs` | Modifica | Aggiungere `get_all_stored_raw_passwords_with_filter()` |
| `src/components/features/dashboard.rs` | Modifica | Refactor per paginazione locale + ordinamento async |

---

## Chunk 1: Backend - Fetch Completa con Filtro

### Task 1.1: Aggiungere fetch completa con filtro in db_backend.rs

**Files:**
- Modify: `src/backend/db_backend.rs` (dopo `fetch_passwords_paginated`, ~riga 860)

- [ ] **Step 1: Aggiungere la nuova funzione di fetch**

Aggiungere dopo `fetch_passwords_paginated`:

```rust
/// Recupera TUTTE le password di un utente con filtro opzionale per strength.
///
/// A differenza di `fetch_passwords_paginated`, questa funzione restituisce
/// tutti i record senza paginazione. L'ordinamento rimane `created_at DESC`.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
///
/// # Returns
/// * `Ok(Vec<StoredPassword>)` - Tutte le password cifrate che matchano il filtro
/// * `Err(DBError)` - Errore database
#[instrument(skip(pool))]
pub async fn fetch_all_passwords_for_user_with_filter(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!(
        "Fetching all passwords for user_id={} with filter={:?}",
        user_id, filter
    );

    // Mappa filtro strength → range di score
    let (min_score, max_score) = match filter {
        None => (None, None),
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => (Some(255), Some(0)),
    };

    let results = match (min_score, max_score) {
        (None, None) => {
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, location, location_nonce, password, password_nonce,
                       notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch all passwords: {}", e))
            })?
        }
        (Some(min), Some(max)) => {
            sqlx::query_as::<_, StoredPassword>(
                r#"
                SELECT id, user_id, location, location_nonce, password, password_nonce,
                       notes, notes_nonce, score, created_at
                FROM passwords
                WHERE user_id = ? AND score >= ? AND score <= ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(user_id)
            .bind(min as i32)
            .bind(max as i32)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                DBError::new_list_error(format!("Failed to fetch all passwords: {}", e))
            })?
        }
        _ => unreachable!("min_score e max_score sono sempre entrambi Some o entrambi None"),
    };

    Ok(results)
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 3: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat(db): add fetch_all_passwords_for_user_with_filter

Add function to fetch all passwords with optional strength filter.
Used by frontend sorting which requires all decrypted data."
```

---

### Task 1.2: Aggiungere funzione di fetch completa decifrata in password_utils.rs

**Files:**
- Modify: `src/backend/password_utils.rs` (dopo `get_stored_raw_passwords`, ~riga 250)

- [ ] **Step 1: Aggiungere import della nuova funzione**

Modificare la riga di import da `db_backend` (~riga 10-13):

```rust
use crate::backend::db_backend::{
    fetch_all_passwords_for_user_with_filter, fetch_all_stored_passwords_for_user,
    fetch_passwords_paginated, fetch_user_auth_from_id, remove_temp_old_password,
    upsert_stored_passwords_batch,
};
```

- [ ] **Step 2: Aggiungere la nuova funzione**

Aggiungere dopo `get_stored_raw_passwords` (~riga 249):

```rust
/// Recupera e decifra TUTTE le password dell'utente con filtro opzionale.
///
/// Questa funzione è usata per l'ordinamento frontend che richiede
/// tutti i dati decifrati (location è cifrata nel DB).
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
///
/// # Returns
/// * `Ok(Vec<StoredRawPassword>)` - Tutte le password decifrate
/// * `Err(DBError)` - Errore database o decriptazione
pub async fn get_all_stored_raw_passwords_with_filter(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords = fetch_all_passwords_for_user_with_filter(pool, user_id, filter).await?;

    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None, // Nessun progress tracking
    )
    .await?;

    Ok(stored_raw_passwords)
}
```

- [ ] **Step 3: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 4: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): add get_all_stored_raw_passwords_with_filter

Add function to fetch and decrypt all passwords with filter.
Required for frontend sorting of encrypted location field."
```

---

## Chunk 2: TableOrder Sort Implementation

### Task 2.1: Implementare metodo sort() in TableOrder

**Files:**
- Modify: `src/components/globals/types.rs`

- [ ] **Step 1: Aggiungere import e implementazione**

Sostituire l'intero contenuto del file con:

```rust
use pwd_types::StoredRawPassword;
use secrecy::ExposeSecret;

#[derive(Clone, PartialEq, Copy)]
pub enum TableOrder {
    AZ,
    ZA,
    Oldest,
    Newest,
}

impl TableOrder {
    /// Ordina un slice di password in-place secondo il criterio selezionato.
    ///
    /// # Arguments
    /// * `passwords` - Slice mutabile di password da ordinare
    pub fn sort(&self, passwords: &mut [StoredRawPassword]) {
        match self {
            TableOrder::AZ => passwords.sort_by(|a, b| {
                a.location.expose_secret().cmp(b.location.expose_secret())
            }),
            TableOrder::ZA => passwords.sort_by(|a, b| {
                b.location.expose_secret().cmp(a.location.expose_secret())
            }),
            TableOrder::Oldest => passwords.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            TableOrder::Newest => passwords.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        }
    }
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 3: Commit**

```bash
git add src/components/globals/types.rs
git commit -m "feat(ui): add sort method to TableOrder enum

Implement in-place sorting for StoredRawPassword slice.
Supports A-Z, Z-A, Oldest, Newest ordering."
```

---

## Chunk 3: Dashboard Refactor

### Task 3.1: Refactor dashboard per paginazione locale e ordinamento

**Files:**
- Modify: `src/components/features/dashboard.rs`

- [ ] **Step 1: Aggiungere nuovo import per la funzione di fetch completa**

Aggiungere dopo l'import esistente di `get_stored_raw_passwords_paginated` (~riga 2):

```rust
use crate::backend::password_utils::get_all_stored_raw_passwords_with_filter;
```

- [ ] **Step 2: Rimuovere la resource paginata esistente**

Eliminare il blocco `password_page_data` use_resource (~righe 67-96).

- [ ] **Step 3: Modificare la sezione SIGNALS**

Sostituire la sezione SIGNALS (~righe 54-59) con:

```rust
    // SIGNALS
    let mut unlock_locations = use_signal(|| false);
    let unlock_locations_clone = unlock_locations.clone();
    let mut unlock_passwords = use_signal(|| false);
    let unlock_passwords_clone = unlock_passwords.clone();

    // Ordinamento: default Newest (coincide con ORDER BY created_at DESC del DB)
    let mut current_table_order = use_signal(|| Some(TableOrder::Newest));

    // Dati completi per ordinamento frontend
    let mut all_passwords = use_signal(|| Vec::<StoredRawPassword>::new());
```

- [ ] **Step 4: Aggiungere use_resource per fetch + ordinamento**

Aggiungere dopo la sezione SIGNALS, dove prima c'era `password_page_data`:

```rust
    // Resource per fetch completa + ordinamento
    // Reagisce a: current_table_order, pagination.active_filter()
    let mut sorted_passwords_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id.clone();
        let filter = pagination.active_filter();
        let order = current_table_order();

        async move {
            if user_id == -1 {
                return Vec::new();
            }

            let mut passwords = get_all_stored_raw_passwords_with_filter(&pool, user_id, filter)
                .await
                .unwrap_or_else(|e| {
                    error.set(Some(e));
                    Vec::new()
                });

            // Applica ordinamento
            if let Some(order) = order {
                order.sort(&mut passwords);
            }

            passwords
        }
    });

    // Aggiorna all_passwords quando la resource completa
    use_effect(move || {
        if let Some(data) = sorted_passwords_resource.read().as_ref() {
            all_passwords.set(data.clone());
            pagination.total_count.set(data.len());
        }
    });
```

- [ ] **Step 5: Aggiungere use_memo per paginazione locale**

Aggiungere dopo l'use_effect:

```rust
    // Paginazione locale: slice dei dati completi
    let page_data = use_memo(move || {
        let page = pagination.current_page();
        let page_size = pagination.page_size();
        let all = all_passwords();
        let start = page * page_size;
        let end = (start + page_size).min(all.len());
        if start < all.len() {
            Some(all[start..end].to_vec())
        } else {
            Some(Vec::new())
        }
    });
```

- [ ] **Step 6: Modificare on_stat_click per triggerare restart della resource**

Modificare `on_stat_click` nella sezione StatsAside (~righe 198-201):

```rust
            StatsAside {
                stats: stats(),
                on_stat_click: move |strength| {
                    pagination.set_filter(strength);
                    pagination.go_to_page(0);
                    sorted_passwords_resource.restart();
                },
                active_filter: pagination.active_filter(),
            }
```

- [ ] **Step 7: Modificare on_page_change per paginazione locale**

Modificare `PaginationControls` (~righe 281-287):

```rust
            PaginationControls {
                pagination: pagination.clone(),
                on_page_change: move |new_page| {
                    pagination.go_to_page(new_page);
                    // Non serve restart: paginazione è locale
                },
            }
```

- [ ] **Step 8: Modificare on_confirm_upsert per restart della resource**

Modificare `on_confirm_upsert` (~righe 121-130):

```rust
    let on_confirm_upsert = {
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        move |_| {
            stats_data.restart();
            sorted_passwords_resource.restart();
        }
    };
```

- [ ] **Step 9: Modificare on_confirm_delete per restart della resource**

Modificare `on_confirm_delete` (~righe 133-172), rimuovendo riferimenti a `password_page_data`:

```rust
    let on_confirm_delete = {
        let pool = pool_for_delete.clone();
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        let mut deletion_password_dialog_state = deletion_password_dialog_state.clone();
        let mut error = error.clone();

        move |_| {
            let pool = pool.clone();
            let mut delete_state = deletion_password_dialog_state.clone();
            let mut error_signal = error.clone();
            let mut stats_data = stats_data.clone();
            let mut sorted_passwords_resource = sorted_passwords_resource.clone();

            let Some(password_id) = (delete_state.password_id)() else {
                error_signal.set(Some(DBError::new_general_error(
                    "A Stored Password id is required".to_string(),
                )));
                return;
            };

            spawn(async move {
                let result = delete_stored_password(&pool, password_id).await;
                match result {
                    Ok(_) => {
                        stats_data.restart();
                        delete_state.is_open.set(false);
                        sorted_passwords_resource.restart();
                    }
                    Err(e) => {
                        error_signal.set(Some(e));
                    }
                }
            });

            deletion_password_dialog_state.password_id.set(None);
        }
    };
```

- [ ] **Step 10: Modificare on_need_restart per restart della resource**

Modificare l'use_effect per `on_need_restart` (~righe 181-192):

```rust
    use_effect(move || {
        let mut need_restart = on_need_restart.clone();
        let mut stats_data = stats_data.clone();
        let mut sorted_passwords_resource = sorted_passwords_resource.clone();
        if need_restart() {
            stats_data.restart();
            sorted_passwords_resource.restart();
            need_restart.set(false);
        }
    });
```

- [ ] **Step 11: Modificare on_change della Combobox per restart della resource + reset pagina**

Modificare la Combobox (~righe 215-220):

```rust
                Combobox::<TableOrder> {
                    options: options.clone(),
                    placeholder: "Order by".to_string(),
                    on_change: move |v| {
                        current_table_order.set(v);
                        pagination.go_to_page(0);
                        sorted_passwords_resource.restart();
                    },
                }
```

- [ ] **Step 12: Modificare il render della tabella**

Sostituire la sezione della tabella (~righe 262-278) con:

```rust
            {
                let table_data = page_data();
                if sorted_passwords_resource.read().is_none() {
                    rsx! {
                        div { class: "card card-lg",
                            div { class: "flex justify-center py-8",
                                Spinner { size: SpinnerSize::Medium, color_class: "text-blue-500" }
                            }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "card card-lg",
                            StoredRawPasswordsTable {
                                data: table_data,
                                unlocked_locations: unlock_locations_clone,
                                unlocked_passwords: unlock_passwords_clone,
                            }
                        }
                    }
                }
            }
```

- [ ] **Step 13: Rimuovere variabili non più utilizzate**

Rimuovere la riga:
```rust
let pool_for_page = pool.clone();
```

- [ ] **Step 14: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore (possono esserci warning unused)

- [ ] **Step 15: Commit**

```bash
git add src/components/features/dashboard.rs
git commit -m "feat(ui): implement frontend table ordering with local pagination

- Add async fetch + sort with spinner indicator
- Replace server-side pagination with client-side slicing
- Default sort order: Newest (matches DB ORDER BY created_at DESC)
- Reset to page 1 on order/filter change"
```

---

## Testing Checklist

Dopo l'implementazione, testare manualmente con `dx serve --desktop`:

- [ ] All'avvio dashboard, spinner visibile poi tabella caricata
- [ ] Combobox mostra "Newest" come valore iniziale
- [ ] Cambio ordinamento A-Z → tabella ordinata alfabeticamente per location
- [ ] Cambio ordinamento Z-A → tabella ordinata alfabeticamente inversa
- [ ] Cambio ordinamento Oldest → tabella ordinata per data crescente
- [ ] Cambio ordinamento Newest → tabella ordinata per data decrescente
- [ ] Spinner visibile durante ogni cambio ordinamento
- [ ] Reset a pagina 1 dopo cambio ordinamento
- [ ] Filtro strength (click su stat card) → refetch + reset + riordinamento
- [ ] After upsert password → refetch + tabella aggiornata
- [ ] After delete password → refetch + tabella aggiornata
- [ ] Paginazione funziona correttamente con dati locali
