# PWDManager-StoredPassword-Refactor - Orchestrator Status

## Project Info
- **Project**: PWDManager-StoredPassword-Refactor
- **Started**: 2026-03-13
- **Total Phases**: 7

## Current State
- **Current Phase**: 3
- **Current Stage**: pending
- **Branch**: refactor/stored-password-phase-1
- **Phase 1 Commit**: 4d1f8e4
- **Phase 2 Commit**: 510481c

---

## Phase Progress

### Phase 1: Aggiornamento db_backend.rs queries
- **Plan**: [`docs/plans/2026-03-13-phase-1-db-backend-queries.md`](./plans/2026-03-13-phase-1-db-backend-queries.md)
- **Status**: ✅ WIP committed
- [x] Implementation (WIP - requires Phase 2-7 to compile)
- [ ] Verification (pending compilation)

### Phase 2: Aggiornamento password_utils.rs
- **Plan**: [`docs/plans/2026-03-13-phase-2-password-utils.md`](./plans/2026-03-13-phase-2-password-utils.md)
- **Status**: ✅ completed
- [x] Implementation (WIP - requires Phase 3-7 to compile)
- [ ] Verification (pending compilation)

### Phase 3: Aggiornamento pipeline di migrazione
- **Plan**: [`docs/plans/2026-03-13-phase-3-migration-pipeline.md`](./plans/2026-03-13-phase-3-migration-pipeline.md)
- **Status**: ✅ ready
- [ ] Verification (no code changes)

### Phase 4: Aggiornamento pipeline di export
- **Plan**: [`docs/plans/2026-03-13-phase-4-export-pipeline.md`](./plans/2026-03-13-phase-4-export-pipeline.md)
- **Status**: ✅ ready
- [ ] Implementation
- [ ] Verification

### Phase 5: Aggiornamento pipeline di import
- **Plan**: [`docs/plans/2026-03-13-phase-5-import-pipeline.md`](./plans/2026-03-13-phase-5-import-pipeline.md)
- **Status**: ✅ ready
- [ ] Implementation
- [ ] Verification

### Phase 6: Aggiornamento test password_utils_tests.rs
- **Plan**: [`docs/plans/2026-03-13-phase-6-tests.md`](./plans/2026-03-13-phase-6-tests.md)
- **Status**: ✅ ready
- [ ] Implementation
- [ ] Verification

### Phase 7: Verifica finale e integrazione
- **Plan**: [`docs/plans/2026-03-13-phase-7-verification.md`](./plans/2026-03-13-phase-7-verification.md)
- **Status**: ✅ ready
- [ ] Final verification
- [ ] Integration

---

## Design Decisions

### Campo `name`
- **Decisione**: Campo libero compilato dall'utente (es. "Google", "GitHub")
- **Backwards compatibility**: Non richiesta (database droppato)

### Campo `username`
- **Tipo**: Criptato con AES-256-GCM (come `location` e `password`)
- **Nonce**: `username_nonce` (12 byte, UNIQUE)

---

## Retry Info
- **Retry Count**: 0
- **Max Retries**: 5

---

## Changelog

### 2026-03-13
- Creato orchestrator config
- Creati tutti i piani (Phase 1-7)
- Definito ordine fasi: DB → password_utils → resto
- Tutti i piani in stato `needs_review`
- **Phase 1 WIP completata** - Aggiornate 4 query SQL in db_backend.rs
  - Commit: 4d1f8e4
  - Branch: refactor/stored-password-phase-1
  - Nota: Richiede Phase 2-7 per compilazione completa
- **Phase 2 WIP completata** - Aggiornato password_utils.rs con encryption/decryption username
  - Commit: 510481c
  - Modifiche: create_stored_data_records(), decrypt_bulk_stored_data()
  - Nota: Richiede Phase 3-7 per compilazione completa (errore in export_types.rs)

