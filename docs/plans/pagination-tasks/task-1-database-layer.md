# Task 1: Database Layer

> **Status:** ✅ COMPLETATO (2026-03-06)
>
> **Commit:** `d42f8aa feat(db): add fetch_passwords_paginated and fetch_password_stats`

**Goal:** Aggiungere funzioni di paginazione e stats al database layer.

**Architecture:** Utilizzare query raw SQL con LIMIT/OFFSET per paginazione. Query separata per stats con COUNT GROUP BY.

**Tech Stack:** Rust, sqlx, SQLite

**Dipendenze:**
- ✅ `PasswordStats` già disponibile in `pwd-types` (usato in `src/components/globals/stats_aside.rs`)

---

## Implementazione Completata

### Step 0: Verificare PasswordStats ✅
`PasswordStats` è già definito nel crate `pwd-types`.

### Step 1: fetch_passwords_paginated ✅
Aggiunta funzione in `src/backend/db_backend.rs` dopo `fetch_all_stored_passwords_for_user`.

** Firma implementata:**
```rust
pub async fn fetch_passwords_paginated(
    pool: &SqlitePool,
    user_id: i64,
    filter: Option<PasswordStrength>,
    page: usize,
    page_size: usize,
) -> Result<(Vec<StoredPassword>, u64), DBError>
```

### Step 2: fetch_password_stats ✅
Aggiunta funzione per statistiche aggregate.

### Step 3: Import verificati ✅
```rust
use pwd_types::{
    PasswordGeneratorConfig, PasswordPreset, PasswordStats, PasswordStrength, StoredPassword,
    UserAuth,
};
```

### Step 4: Verifica compilazione ✅
`cargo check` completato senza errori.

### Step 5: Commit ✅
`d42f8aa feat(db): add fetch_passwords_paginated and fetch_password_stats`

---

## Deviazioni dal Piano Originale

### 1. Tipo di ritorno di `fetch_passwords_paginated`

**Piano originale:** `Result<(Vec<StoredRawPassword>, u64), DBError>`
**Implementato:** `Result<(Vec<StoredPassword>, u64), DBError>`

**Motivazione:**
- Separazione delle responsabilità: la funzione DB non deve gestire la decifratura
- Coerenza con il pattern esistente (`fetch_all_stored_passwords_for_user`)
- Il chiamante può decifrare usando `decrypt_bulk_stored_data` da `password_utils`

### 2. Query raw SQL invece di sqlx-template builder

**Piano originale:** Usare `StoredPassword::builder_select()` con `score_gte`/`score_lte`
**Implementato:** Query raw SQL con LIMIT/OFFSET

**Motivazione:**
- I metodi `score_gte`/`score_lte` non sono generati dal builder di `StoredPassword`
- Query raw SQL più esplicita e manutenibile per logica di filtering complessa

### 3. Tipi `usize` invece di `u32` per PasswordStats

**Piano originale:** `count as u32`
**Implementato:** `count as usize`

**Motivazione:**
- `PasswordStats` in `pwd-types` usa `usize` per i campi, non `u32`

---

## Files Modificati

- `src/backend/db_backend.rs` (+177 righe)

---

## Note per Task Successivi

- Per decifrare le password paginate, usare `decrypt_bulk_stored_data(user_auth, stored_passwords, None)`
- Il filtro strength è mappato su range di score (WEAK: 0-49, MEDIUM: 50-69, etc.)
- `NotEvaluated` non è un filtro valido → restituisce 0 risultati
