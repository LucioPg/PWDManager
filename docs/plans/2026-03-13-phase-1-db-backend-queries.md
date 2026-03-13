# Phase 1: Aggiornamento db_backend.rs queries

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aggiornare le query SQL manuali in db_backend.rs per includere i nuovi campi `name`, `username` e `username_nonce`.

**Architecture:** Le query SQL manuali devono includere tutti i campi della tabella `passwords` per mappare correttamente la struct `StoredPassword` che è già aggiornata in `pwd-types`.

**Tech Stack:** Rust, SQLx, SQLite

---

## Context

### Schema Tabella `passwords` (aggiornato in init_queries.rs)
```sql
CREATE TABLE passwords (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,                    -- NUOVO: nome del servizio
    username BLOB NOT NULL,                -- NUOVO: nome utente criptato
    username_nonce BLOB NOT NULL UNIQUE,   -- NUOVO: nonce per username
    location BLOB NOT NULL,
    location_nonce BLOB NOT NULL UNIQUE,
    password BLOB NOT NULL,
    password_nonce BLOB NOT NULL UNIQUE,
    notes BLOB,
    notes_nonce BLOB UNIQUE,
    score INTEGER NOT NULL CHECK (0 <= score <= 100),
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
)
```

### Struct `StoredPassword` (già aggiornata in pwd-types)
```rust
pub struct StoredPassword {
    pub id: Option<i64>,
    pub user_id: i64,
    pub name: String,                    // NUOVO
    pub username: DbSecretVec,           // NUOVO
    pub username_nonce: Vec<u8>,         // NUOVO
    pub location: DbSecretVec,
    pub location_nonce: Vec<u8>,
    pub password: DbSecretVec,
    pub password_nonce: Vec<u8>,
    pub notes: Option<DbSecretVec>,
    pub notes_nonce: Option<Vec<u8>>,
    pub score: PasswordScore,
    pub created_at: Option<String>,
}
```

---

## File Structure

### Files to Modify
- `src/backend/db_backend.rs` - Query SQL manuali

### Files NOT to Modify (usa sqlx-template auto-generato)
- Funzione `fetch_all_stored_passwords_for_user` - usa `StoredPassword::builder_select()`
- Funzione `upsert_stored_passwords_batch` - usa `StoredPassword::upsert_by_id()`

---

## Task 1: Aggiornare fetch_passwords_paginated - query senza filtro

**Files:**
- Modify: `src/backend/db_backend.rs:791-799`

**Current code:**
```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, location, location_nonce, password, password_nonce,
           notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ?
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#,
)
```

- [ ] **Step 1: Aggiornare la query SELECT per includere name, username, username_nonce**

```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, name, username, username_nonce, location, location_nonce,
           password, password_nonce, notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ?
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#,
)
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 2: Aggiornare fetch_passwords_paginated - query con filtro score

**Files:**
- Modify: `src/backend/db_backend.rs:812-820`

**Current code:**
```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, location, location_nonce, password, password_nonce,
           notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ? AND score >= ? AND score <= ?
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#,
)
```

- [ ] **Step 1: Aggiornare la query SELECT per includere name, username, username_nonce**

```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, name, username, username_nonce, location, location_nonce,
           password, password_nonce, notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ? AND score >= ? AND score <= ?
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#,
)
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 3: Aggiornare fetch_all_passwords_for_user_with_filter - query senza filtro

**Files:**
- Modify: `src/backend/db_backend.rs:899-906`

**Current code:**
```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, location, location_nonce, password, password_nonce,
           notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ?
    ORDER BY created_at DESC
    "#,
)
```

- [ ] **Step 1: Aggiornare la query SELECT per includere name, username, username_nonce**

```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, name, username, username_nonce, location, location_nonce,
           password, password_nonce, notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ?
    ORDER BY created_at DESC
    "#,
)
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 4: Aggiornare fetch_all_passwords_for_user_with_filter - query con filtro score

**Files:**
- Modify: `src/backend/db_backend.rs:916-923`

**Current code:**
```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, location, location_nonce, password, password_nonce,
           notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ? AND score >= ? AND score <= ?
    ORDER BY created_at DESC
    "#,
)
```

- [ ] **Step 1: Aggiornare la query SELECT per includere name, username, username_nonce**

```rust
sqlx::query_as::<_, StoredPassword>(
    r#"
    SELECT id, user_id, name, username, username_nonce, location, location_nonce,
           password, password_nonce, notes, notes_nonce, score, created_at
    FROM passwords
    WHERE user_id = ? AND score >= ? AND score <= ?
    ORDER BY created_at DESC
    "#,
)
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 5: Verifica finale

- [ ] **Step 1: Eseguire cargo check completo**

Run: `cargo check`
Expected: Nessun errore di compilazione

- [ ] **Step 2: Commit delle modifiche**

```bash
git add src/backend/db_backend.rs
git commit -m "feat(db): add name and username fields to StoredPassword queries

- Update fetch_passwords_paginated queries (with/without filter)
- Update fetch_all_passwords_for_user_with_filter queries (with/without filter)
- Add name, username, username_nonce to SELECT statements

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Notes

- `fetch_all_stored_passwords_for_user` NON richiede modifiche perché usa `StoredPassword::builder_select()` generato da sqlx-template
- `upsert_stored_passwords_batch` NON richiede modifiche perché usa `StoredPassword::upsert_by_id()` generato da sqlx-template
- `fetch_password_stats` NON richiede modifiche perché seleziona solo `score` e fa `COUNT(*)`

---

## Verification Checklist

- [ ] Tutte e 4 le query SELECT aggiornate con i nuovi campi
- [ ] `cargo check` passa senza errori
- [ ] Commit effettuato
