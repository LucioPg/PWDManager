# Phase 7: Verifica finale e integrazione

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verificare che tutte le modifiche delle fasi precedenti siano integrate correttamente e che il sistema funzioni end-to-end.

**Architecture:** Eseguire test completi, verificare compilazione, e assicurarsi che non ci siano regressioni.

**Tech Stack:** Rust, cargo, clippy

---

## Context

### Fasi completate
- ✅ Phase 1: Query DB aggiornate
- ✅ Phase 2: password_utils.rs encryption/decryption
- ✅ Phase 3: Migration pipeline verificata
- ✅ Phase 4: Export pipeline aggiornata
- ✅ Phase 5: Import pipeline verificata
- ✅ Phase 6: Test aggiornati

### Ordine di Esecuzione

```
Phase 1: DB Queries
     │
     ▼
Phase 2: Encrypt/Decrypt ─────────────┬─────────────────┐
     │                                │                 │
     ▼                                ▼                 ▼
Phase 3: Migration Verify      Phase 4: Export    Phase 6: Tests
     │                                │                 │
     │                                ▼                 │
     │                          Phase 5: Import         │
     │                                │                 │
     └────────────────────────────────┴─────────────────┘
                                      │
                                      ▼
                               Phase 7: Verification
```

**Esecuzione parallela consigliata:**
- Batch 1: Phase 1
- Batch 2: Phase 2
- Batch 3: Phase 3, Phase 4 (possono andare in parallelo)
- Batch 4: Phase 5, Phase 6 (possono andare in parallelo dopo le rispettive dipendenze)
- Batch 5: Phase 7

### Obiettivo finale
Verificare che l'intero flusso funzioni:
1. Creare password con `name` e `username`
2. Salvare nel DB
3. Recuperare dal DB
4. Decriptare correttamente
5. Esportare e importare

---

## Task 1: Verifica compilazione completa

- [ ] **Step 1: Eseguire cargo check completo**

Run: `cargo check`
Expected: Nessun errore di compilazione

- [ ] **Step 2: Verificare warnings**

Run: `cargo check 2>&1 | grep -i warning`
Expected: Nessun warning critico (ignorare unused warnings se presenti)

---

## Task 2: Eseguire tutti i test del modulo backend

- [ ] **Step 1: Test password_utils**

Run: `cargo test --lib password_utils_tests`
Expected: PASS

- [ ] **Step 2: Test import**

Run: `cargo test --lib import`
Expected: PASS

- [ ] **Step 3: Test export (se presenti)**

Run: `cargo test --lib export`
Expected: PASS

- [ ] **Step 4: Test db_backend**

Run: `cargo test --lib db_backend`
Expected: PASS

---

## Task 3: Eseguire clippy

- [ ] **Step 1: Clippy con warnings come errori**

Run: `cargo clippy -- -D warnings`
Expected: Nessun errore

Se ci sono errori, analizzare e fixare.

---

## Task 4: Test di integrazione manuale (opzionale)

Se si vuole verificare end-to-end:

- [ ] **Step 1: Avviare l'applicazione**

Run: `dx serve --desktop`
Expected: App avvia senza errori

- [ ] **Step 2: Creare una nuova password con name e username**

Verificare che:
- Il campo `name` viene salvato
- Il campo `username` viene salvato e criptato
- La password viene visualizzata correttamente

- [ ] **Step 3: Verificare export/import**

Esportare una password e re-importarla:
- Il file JSON contiene `name` e `username`
- L'import recupera correttamente i campi

---

## Task 5: Aggiornare documentazione

- [ ] **Step 1: Aggiornare orchestrator-status.md**

Aggiornare lo stato di tutte le fasi a "completed":

```markdown
### Phase 1: Aggiornamento db_backend.rs queries
- **Status**: ✅ completed

### Phase 2: Aggiornamento password_utils.rs
- **Status**: ✅ completed

... (etc)
```

- [ ] **Step 2: Commit finale**

```bash
git add docs/orchestrator-status.md
git commit -m "docs: mark all StoredPassword refactor phases as completed

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Verifica finale

- [ ] **Step 1: Eseguire tutti i test**

Run: `cargo test`
Expected: PASS

- [ ] **Step 2: Verificare copertura (opzionale)**

Run: `cargo tarpaulin --ignore-tests` (se installato)
Expected: Copertura mantenuta o migliorata

---

## Verification Checklist

- [ ] `cargo check` passa senza errori
- [ ] `cargo clippy -- -D warnings` passa
- [ ] Tutti i test `--lib` passano
- [ ] `cargo test` completo passa
- [ ] orchestrator-status.md aggiornato
- [ ] Commit finale effettuato

---

## Riepilogo modifiche totali

### File modificati
| File | Fase | Modifica |
|------|------|----------|
| `src/backend/db_backend.rs` | 1 | Query SELECT con name, username |
| `src/backend/password_utils.rs` | 2 | Encrypt/decrypt username |
| `src/backend/export_types.rs` | 4 | ExportablePassword con name, username |
| `src/backend/export.rs` | 4 | Export con nuovi campi |
| `src/backend/import.rs` | 5 | Test helper aggiornato |
| `src/backend/password_utils_tests.rs` | 6 | Tutti i test con name, username |

### Schema DB (già aggiornato in init_queries.rs)
```sql
name TEXT NOT NULL,
username BLOB NOT NULL,
username_nonce BLOB NOT NULL UNIQUE,
```

### Struct (già aggiornate in pwd-types)
- `StoredPassword`: name, username, username_nonce
- `StoredRawPassword`: name, username
