# Task 1: Database Layer

> **Per Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Aggiungere funzioni di paginazione e stats al database layer.

**Architecture:** Utilizzare `sqlx-template::find_page()` per query paginate con LIMIT/OFFSET. Query separata per stats con COUNT GROUP BY.

**Tech Stack:** Rust, sqlx, sqlx-template, SQLite

**Dipendenze:** Nessuna (task indipendente)

---

## Files

- **Modify:** `src/backend/db_backend.rs`
- **Reference:** `docs/howto_sqlitetemplate.md` (per sintassi find_page)

---

## Step 1: Aggiungere funzione fetch_passwords_paginated

In `src/backend/db_backend.rs`, aggiungere dopo `fetch_all_stored_passwords_for_user` (riga ~738):

```rust
/// Fetch passwords paginate con filtro opzionale per strength.
///
/// # Arguments
/// * `pool` - Connection pool SQLite
/// * `user_id` - ID dell'utente
/// * `filter` - Filtro opzionale per PasswordStrength
/// * `page` - Pagina (0-indexed)
/// * `page_size` - Numero di elementi per pagina
///
/// # Returns
/// * `Ok((Vec<StoredRawPassword>, u64))` - Passwords decifrate e totale count
/// * `Err(DBError)` - Errore database o decifratura
#[instrument(skip(pool))]
pub async fn fetch_passwords_paginated(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredRawPassword>, u64), DBError> {
    debug!(
        "Fetching passwords paginated: user_id={}, filter={:?}, page={}, page_size={}",
        user_id, filter, page, page_size
    );

    // Mappa filtro strength → range di score
    let (min_score, max_score) = match filter {
        None => (None, None),
        Some(PasswordStrength::WEAK) => (Some(0), Some(49)),
        Some(PasswordStrength::MEDIUM) => (Some(50), Some(69)),
        Some(PasswordStrength::STRONG) => (Some(70), Some(84)),
        Some(PasswordStrength::EPIC) => (Some(85), Some(95)),
        Some(PasswordStrength::GOD) => (Some(96), Some(100)),
        Some(PasswordStrength::NotEvaluated) => (None, None),
    };

    let offset = page * page_size;

    // Costruisci query con builder
    let mut builder = StoredPassword::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .order_by_created_at_desc()
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;

    // Applica filtro score se presente
    if let Some(min) = min_score {
        builder = builder
            .score_gte(&PasswordScore::new(min))
            .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;
    }
    if let Some(max) = max_score {
        builder = builder
            .score_lte(&PasswordScore::new(max))
            .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;
    }

    // Esegui query paginata: (offset, limit, count_total)
    let (results, _page_info, total) = builder
        .find_page((offset, page_size, true), pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch paginated passwords: {}", e)))?;

    // Converti StoredPassword → StoredRawPassword (decifra)
    let raw_passwords = results
        .into_iter()
        .map(|sp| StoredRawPassword::try_from(sp))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DBError::new_general_error(format!("Decryption error: {}", e)))?;

    Ok((raw_passwords, total))
}
```

**Nota:** Aggiungere import per `PasswordStrength` e `PasswordScore` se non presenti.

---

## Step 2: Aggiungere funzione fetch_password_stats

In `src/backend/db_backend.rs`, aggiungere dopo `fetch_passwords_paginated`:

```rust
/// Fetch statistiche password per l'utente (conteggi per strength).
///
/// Questa query è sempre "fresca" perché viene eseguita separatamente
/// dalla paginazione e non viene cacheata.
#[instrument(skip(pool))]
pub async fn fetch_password_stats(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<PasswordStats, DBError> {
    debug!("Fetching password stats for user_id: {}", user_id);

    // Query con CASE per raggruppare per strength
    let rows = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            CASE
                WHEN score < 50 THEN 0
                WHEN score < 70 THEN 1
                WHEN score < 85 THEN 2
                WHEN score < 96 THEN 3
                ELSE 4
            END as strength_group,
            COUNT(*) as count
        FROM passwords
        WHERE user_id = ?
        GROUP BY strength_group
        ORDER BY strength_group
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| DBError::new_list_error(format!("Failed to fetch password stats: {}", e)))?;

    let mut stats = PasswordStats::default();

    for (group, count) in rows {
        match group {
            0 => stats.weak = count as u32,
            1 => stats.medium = count as u32,
            2 => stats.strong = count as u32,
            3 => stats.epic = count as u32,
            4 => stats.god = count as u32,
            _ => {}
        }
    }

    stats.total = stats.weak + stats.medium + stats.strong + stats.epic + stats.god + stats.not_evaluated;

    Ok(stats)
}
```

---

## Step 3: Verificare import necessari

All'inizio di `db_backend.rs`, verificare che siano presenti:

```rust
use pwd_types::{PasswordScore, PasswordStrength, PasswordStats, StoredPassword, StoredRawPassword};
```

---

## Step 4: Verificare compilazione

```bash
cargo check
```

**Expected:** Nessun errore. Warning accettabili.

---

## Step 5: Commit

```bash
git add src/backend/db_backend.rs
git commit -m "feat(db): add fetch_passwords_paginated and fetch_password_stats"
```

---

## Merge Instructions

```bash
# Torna al branch principale del task
git checkout dev-dashboard-pagination-38

# Merge
git merge task-1-pagination-database --no-ff -m "Merge task-1: database layer for pagination"

# Pulisci branch
git branch -d task-1-pagination-database
```

---

## Notes

- `find_page((offset, limit, count), pool)` restituisce `(Vec<T>, Page, u64)` dove `u64` è il totale
- Il filtro strength viene mappato su range di score per evitare query N+1
- La query stats usa CASE per raggruppare, è efficiente anche con migliaia di righe
