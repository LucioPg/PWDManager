# Task 5: StatsAside Update

> **Per Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Aggiornare StatsAside per usare le stats dalla query separata invece di calcolarle lato client.

**Architecture:** StatsAside riceve già le stats come prop dalla Dashboard. Verificare che funzioni correttamente con la nuova fonte dati.

**Tech Stack:** Rust, Dioxus 0.7

**Dipendenze:** Task 4 (Dashboard Integration) - StatsAside riceve le stats come prop dalla Dashboard

---

## Files

- **Review:** `src/components/globals/stats_aside.rs`
- **Modify:** Solo se necessario

---

## Step 1: Verificare StatsAside component

Leggere il file esistente:

```bash
# Just review, no changes needed if it already accepts stats as prop
```

Se `StatsAside` accetta già `stats: PasswordStats` come prop, **nessuna modifica necessaria**.

---

## Step 2: Verificare props StatsAside

Il componente dovrebbe avere questa firma (o simile):

```rust
#[component]
pub fn StatsAside(
    stats: PasswordStats,
    on_stat_click: Callback<Option<PasswordStrength>>,
    active_filter: Option<PasswordStrength>,
) -> Element {
    // ...
}
```

Se manca `active_filter`, aggiungerlo per evidenziare il filtro attivo.

---

## Step 3: Se necessario, aggiungere active_filter highlighting

Se il componente non ha `active_filter`, aggiungere evidenziazione visiva:

```rust
// Nella parte che renderizza i vari StatCard
StatCard {
    // ...
    class: if active_filter == Some(PasswordStrength::STRONG) {
        "ring-2 ring-primary"
    } else {
        ""
    },
}
```

---

## Step 4: Verificare compilazione

```bash
cargo check
```

---

## Step 5: Commit (solo se modificato)

```bash
git add src/components/globals/stats_aside.rs
git commit -m "refactor(stats-aside): support active filter highlighting"
```

Se non sono necessarie modifiche:

```bash
git commit --allow-empty -m "docs(task-5): stats-aside already compatible"
```

---

## Merge Instructions

```bash
git checkout dev-dashboard-pagination-38
git merge task-5-stats-aside --no-ff -m "Merge task-5: stats aside update"
git branch -d task-5-stats-aside
```

---

## Notes

- Questo task potrebbe essere un "no-op" se StatsAside è già corretto
- L'importante è verificare che le stats vengano dalla query DB, non calcolate lato client
- `active_filter` visivo aiuta l'utente a capire quale filtro è attivo

---

## Implementation Log

**Data:** 2026-03-06
**Commit:** `d5d3150`

**Modifiche effettuate:**
1. Il componente `StatsAside` aveva già `active_filter` come prop, ma non lo usava
2. Aggiunta logica di evidenziazione nel rendering degli stat items
3. Creata classe CSS `pwd-stats-aside__item--active` con sfondo indigo e ring

**File modificati:**
- `src/components/globals/stats_aside.rs` — uso di `active_filter` per highlighting
- `assets/input_main.css` — aggiunta classe `.pwd-stats-aside__item--active`

**Verifica:** `cargo check` ✅ passato
