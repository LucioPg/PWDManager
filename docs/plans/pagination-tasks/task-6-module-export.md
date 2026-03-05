# Task 6: Module Export

> **Per Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Creare il modulo `pagination` e aggiornare gli export per rendere disponibili i nuovi componenti.

**Architecture:** Modulo `pagination` con re-export di `PaginationState` e `PaginationControls`.

**Tech Stack:** Rust

**Dipendenze:** Task 2 (PaginationState), Task 3 (PaginationControls)

---

## Files

- **Create:** `src/components/globals/pagination/mod.rs`
- **Modify:** `src/components/globals/mod.rs`

---

## Step 1: Creare mod.rs per il modulo pagination

In `src/components/globals/pagination/mod.rs`:

```rust
mod pagination_controls;
mod pagination_state;

pub use pagination_controls::PaginationControls;
pub use pagination_state::{CacheKey, PaginationState};
```

---

## Step 2: Aggiornare globals/mod.rs

**CERCARE** in `src/components/globals/mod.rs` la sezione dei moduli e aggiungere:

```rust
pub mod pagination;
```

**CERCARE** nella sezione `pub use` e aggiungere:

```rust
pub use pagination::{CacheKey, PaginationControls, PaginationState};
```

---

## Step 3: Verificare struttura finale

La struttura dovrebbe essere:

```
src/components/globals/
├── mod.rs
├── pagination/
│   ├── mod.rs
│   ├── pagination_controls.rs
│   └── pagination_state.rs
├── table/
│   └── ...
└── ... (altri file)
```

---

## Step 4: Verificare compilazione completa

```bash
cargo check
```

**Expected:** Nessun errore.

---

## Step 5: Verificare build release

```bash
dx build --desktop --release
```

**Expected:** Build completato senza errori.

---

## Step 6: Commit

```bash
git add src/components/globals/pagination/mod.rs src/components/globals/mod.rs
git commit -m "feat(pagination): add module exports"
```

---

## Merge Instructions

```bash
git checkout dev-dashboard-pagination-38
git merge task-6-module-export --no-ff -m "Merge task-6: pagination module export"
git branch -d task-6-module-export
```

---

## Notes

- Questo è l'ultimo task, completa l'integrazione
- Verificare che non ci siano warning del compilatore
- La build release assicura che tutto funzioni in produzione
