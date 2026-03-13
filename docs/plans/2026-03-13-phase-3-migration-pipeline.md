# Phase 3: Aggiornamento pipeline di migrazione

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verificare che la pipeline di migrazione gestisca correttamente i nuovi campi `name` e `username`.

**Architecture:** La pipeline di migrazione usa `decrypt_bulk_stored_data` e `create_stored_data_records` già aggiornate in Phase 2. Questa fase verifica l'integrazione.

**Tech Stack:** Rust, async/await, progress tracking

---

## Context

### Funzioni coinvolte
1. `stored_passwords_migration_pipeline` - Migrazione semplice
2. `stored_passwords_migration_pipeline_with_progress` - Migrazione con progress

### Flusso dati
```
fetch_all_stored_passwords_for_user (Phase 1)
           ↓
decrypt_bulk_stored_data (Phase 2) ← include name, username
           ↓
create_stored_data_records (Phase 2) ← include name, username
           ↓
upsert_stored_passwords_batch (sqlx-template auto)
           ↓
remove_temp_old_password
```

---

## Analysis

### Verifica automatica
Le funzioni di migrazione usano indirettamente le funzioni aggiornate in Phase 1 e Phase 2:
- ✅ `fetch_all_stored_passwords_for_user` → aggiorna in Phase 1
- ✅ `decrypt_bulk_stored_data` → aggiorna in Phase 2
- ✅ `create_stored_data_records` → aggiorna in Phase 2
- ✅ `upsert_stored_passwords_batch` → sqlx-template auto-generato

### Conclusione
**Nessuna modifica diretta richiesta** alle funzioni di migrazione se Phase 1 e Phase 2 sono completate correttamente.

---

## Task 1: Verifica integrazione

**Files:**
- Read: `src/backend/password_utils.rs:432-516`

- [ ] **Step 1: Verificare che le funzioni di migrazione usino solo funzioni già aggiornate**

Controllare che:
1. `stored_passwords_migration_pipeline` usa `fetch_all_stored_passwords_for_user`, `decrypt_bulk_stored_data`, `create_stored_data_pipeline_bulk`
2. `stored_passwords_migration_pipeline_with_progress` usa le stesse funzioni con progress tracking

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 2: Aggiornare documentazione

- [ ] **Step 1: Aggiornare commenti doc se necessario**

Se i commenti delle funzioni di migrazione menzionano i campi, aggiungere `name` e `username`.

- [ ] **Step 2: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "docs(crypto): update migration pipeline docs for name/username

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Notes

### Dipendenze
- **Richiede Phase 1** completata (query DB)
- **Richiede Phase 2** completata (encrypt/decrypt)

### No-code-change phase
Questa fase è principalmente di verifica. Se Phase 1 e Phase 2 sono corrette, la migrazione funzionerà automaticamente.

---

## Verification Checklist

- [ ] Verificato che migrazione usa funzioni aggiornate
- [ ] `cargo check` passa senza errori
- [ ] Documentazione aggiornata (se necessario)
- [ ] Commit effettuato
