# Dashboard Pagination - Orchestrazione

> **Per Claude:** Questo file traccia il progresso dell'implementazione della paginazione.
> Ogni task ha il proprio piano dettagliato in `docs/plans/pagination-tasks/`.
> Usa `superpowers:executing-plans` per eseguire i task.

**Branch target:** `dev-dashboard-pagination-38`
**Design doc:** `docs/plans/2026-03-05-dashboard-pagination-design.md`

---

## Stato Progresso

| Task | File Piano | Stato | Worktree | Branch |
|------|------------|-------|----------|--------|
| 1-database-layer | [task-1-database-layer.md](./pagination-tasks/task-1-database-layer.md) | ✅ COMPLETATO | - | `dd42e25` |
| 2-pagination-state | [task-2-pagination-state.md](./pagination-tasks/task-2-pagination-state.md) | ✅ COMPLETATO | - | `350ec04` |
| 3-pagination-controls | [task-3-pagination-controls.md](./pagination-tasks/task-3-pagination-controls.md) | ✅ COMPLETATO | - | `dd42e25` |
| 4-dashboard-integration | [task-4-dashboard-integration.md](./pagination-tasks/task-4-dashboard-integration.md) | ✅ COMPLETATO | - | `00d3bff` |
| 5-stats-aside-update | [task-5-stats-aside-update.md](./pagination-tasks/task-5-stats-aside-update.md) | ✅ COMPLETATO | - | `d5d3150` |
| 6-module-export | [task-6-module-export.md](./pagination-tasks/task-6-module-export.md) | 🔵 DA FARE | - | - |

**Legenda:** 🔵 DA FARE | 🟡 IN CORSO | 🟢 COMPLETATO | 🔴 BLOCCATO

---

## Dipendenze tra Task

```
Task 1 (Database) ─────┬─────────────────────────┐
                        │                          │
                        ▼                          ▼
Task 2 (State)     Task 5 (Stats)           Task 3 (Controls)
     │                                              │
     └──────────────────┬───────────────────────────┘
                        │
                        ▼
              Task 4 (Dashboard Integration)
                        │
                        ▼
              Task 6 (Module Export)
```

**Parallelizzazione possibile:**
- **Batch 1 (parallelo):** Task 1, Task 2, Task 3
- **Batch 2 (dopo Batch 1):** Task 4, Task 5
- **Batch 3 (finale):** Task 6

---

## Istruzioni per Esecuzione

### Avvio Task

1. Leggere il file del task in `docs/plans/pagination-tasks/task-X-*.md`
2. Aggiornare questo file: stato → 🟡 IN CORSO, worktree path, branch name
3. Creare worktree:
   ```bash
   git worktree add .worktrees/pagination-task-X dev-dashboard-pagination-38
   cd .worktrees/pagination-task-X
   git checkout -b task-X-pagination-<name>
   ```
4. Eseguire il piano
5. Committare e mergiare:
   ```bash
   git add .
   git commit -m "feat(pagination): <description>"
   git checkout dev-dashboard-pagination-38
   git merge task-X-pagination-<name> --no-ff
   git branch -d task-X-pagination-<name>
   ```
6. Aggiornare questo file: stato → 🟢 COMPLETATO
7. Rimuovere worktree:
   ```bash
   git worktree remove .worktrees/pagination-task-X
   ```

### Rollback

Se qualcosa va storto:
```bash
git checkout dev-dashboard-pagination-38
git reset --hard HEAD~1  # Rimuove ultimo merge
git worktree remove .worktrees/pagination-task-X
git branch -D task-X-pagination-<name>
```

---

## Riepilogo File Modificati

### Nuovi File
- `src/components/globals/pagination/mod.rs`
- `src/components/globals/pagination/pagination_state.rs`
- `src/components/globals/pagination/pagination_controls.rs`

### File Modificati
- `src/backend/db_backend.rs` — funzioni paginated
- `src/components/features/dashboard.rs` — integrazione paginazione
- `src/components/globals/mod.rs` — export modulo
- `src/components/globals/stats_aside.rs` — query stats separata

---

## Checklist Finale

Dopo completamento di tutti i task:

- [ ] Paginazione funziona con 100+ password
- [ ] Stats sempre corrette (non solo pagina corrente)
- [ ] Filtro resetta a pagina 1
- [ ] CRUD invalida cache
- [ ] Controlli Previous/Next disabilitati appropriatamente
- [ ] Nessun warning del compilatore
- [ ] Build release funziona: `dx build --desktop --release`
