# Library Extraction Orchestrator

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
>
> **IMPORTANTE:** Questo è un piano ORCHESTRATORE. Ogni Step richiede:
> 1. Approvazione di questo orchestratore prima di procedere
> 2. Un piano dedicato in sessione separata (vedi `docs/plans/2026-02-26-extract-<crate>.md`)
> 3. Aggiornamento di questo documento dopo il completamento
> 4. **Aggiornamento del Reference Document** (`docs/library-extraction-analysis.md`) con lezioni apprese e modifiche tecniche
>
> **⚠️ Per la skill writing-plans:** Quando crei il piano dedicato per uno step, DEVI leggere:
> - Questo orchestratore (per stato e prerequisiti)
> - Il file `docs/library-extraction-analysis.md` (per dettagli tecnici living)

**Goal:** Coordinare l'estrazione di 3 librerie Rust dal progetto PWDManager, con checkpoint e review tra ogni
estrazione.

**Architecture:** Estrazione incrementale con dipendenze unidirezionali: `pwd-types` → `pwd-strength` → `pwd-crypto`.
Ogni step è atomico e reversibile.

**Tech Stack:** Rust workspace, Cargo features, sqlx, secrecy, argon2, aes-gcm

**Reference Document:** `docs/library-extraction-analysis.md`

---

## Stato Estrazioni

| Step | Libreria       | Stato          | Piano                                           | Completato |
|------|----------------|----------------|-------------------------------------------------|------------|
| 0    | Prerequisiti   | ✅ COMPLETATO   | -                                               | 2026-02-26 |
| 1    | `pwd-types`    | ✅ COMPLETATO   | `docs/plans/2026-02-26-extract-pwd-types.md`    | 2026-02-26 |
| 2    | `pwd-strength` | ⏳ NON INIZIATO | `docs/plans/2026-02-26-extract-pwd-strength.md` | -          |
| 3    | `pwd-crypto`   | ⏳ NON INIZIATO | `docs/plans/2026-02-26-extract-pwd-crypto.md`   | -          |
| F    | Finalizzazione | ⏳ NON INIZIATO | -                                               | -          |

**Legenda stati:**

- ⏳ NON INIZIATO - In attesa di approvazione
- 🔄 IN CORSO - Piano in esecuzione in sessione separata
- ✅ COMPLETATO - Verificato e documentato
- ❌ BLOCCATO - Richiede intervento

---

## Flusso di Esecuzione

```
┌────────────────────────────────────────────────────────────┐
│                    ORCHESTRATORE                           │
│                                                            │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ Step 0  │───▶│ Step 1  │───▶│ Step 2  │───▶│ Step 3  │  │
│  │Pre-reqs │    │pwd-types│    │pwd-strn │    │pwd-crypt│  │
│  └────┬────┘    └────┬────┘    └────┬────┘    └────┬────┘  │
│       │              │              │              │       │
│       ▼              ▼              ▼              ▼       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              CHECKPOINT REVIEW                      │   │
│  │  • Verifica test passano                            │   │
│  │  • Aggiorna orchestratore (lezioni apprese)         │   │
│  │  • Aggiorna reference document (dati tecnici)       │   │
│  │  • Approva step successivo                          │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
```

---

## Step 0: Prerequisiti

**Obiettivo:** Verificare che il progetto sia in stato pulito prima di iniziare.

### Task 0.1: Verifica Stato

**Eseguire in questa sessione:**

```bash
# Verificare test
cargo test --workspace

# Verificare compilazione
cargo check --workspace

# Verificare git
git status
```

**Criterio di successo:**

- [ ] `cargo test --workspace` passa al 100%
- [ ] `cargo check --workspace` completa senza errori
- [ ] `git status` mostra workspace pulito (o modifiche documentate)

### Task 0.2: Branch di Lavoro

**Eseguire:**

```bash
git checkout -b feat/extract-libs-31
```

**Criterio di successo:**

- [ ] Branch `feat/extract-libs-31` creato e attivo

### Task 0.3: Checkpoint

**Prima di procedere a Step 1, aggiornare questo documento:**

- [ ] Cambiare stato Step 0 da ⏳ a ✅
- [ ] Aggiungere data completamento
- [ ] Documentare eventuali anomalie riscontrate

---

## Step 1: Estrazione `pwd-types`

**Obiettivo:** Estrarre tipi puri da `src/backend/password_types_helper.rs`.

**Dipendenze:** Nessuna (libreria base)

**Stato:** ✅ COMPLETATO

**Piano dedicato:** `docs/plans/2026-02-26-extract-pwd-types.md`

### Prerequisiti per avviare Step 1

- [ ] Step 0 completato ✅
- [ ] Branch `feat/extract-libs-31` attivo
- [ ] Piano dedicato creato e approvato

### Cosa deve contenere il piano dedicato

Il piano `docs/plans/2026-02-26-extract-pwd-types.md` deve coprire:

1. Creazione struttura crate `pwd-types/`
2. Configurazione `Cargo.toml` con feature flags
3. Estrazione moduli (score, stats, secrets, stored, generator, form)
4. Aggiornamento workspace e dipendenze
5. Aggiornamento use paths nel progetto padre
6. Test di verifica
7. Commit

### Output atteso dal piano

- [ ] Crate `pwd-types` funzionante
- [ ] Feature flags: `secrecy`, `sqlx`, `generator`, `dioxus`
- [ ] Tutti i test passano
- [ ] Commit: `feat: extract pwd-types library`

### Checkpoint post-Step 1

**Dopo completamento, aggiornare i documenti living:**

**Orchestratore (questo file):**
1. Cambiare stato Step 1 da ⏳ a ✅
2. Aggiungere data completamento
3. Compilare sezione "Lezioni Apprese"

**Reference Document (`docs/library-extraction-analysis.md`):**
1. Aggiornare mappa dipendenze se cambiata
2. Documentare problemi riscontrati e soluzioni
3. Aggiornare API se modificate
4. Aggiungere note per Step 2 e Step 3

---

## Step 2: Estrazione `pwd-strength`

**Obiettivo:** Estrarre logica valutazione password da `src/backend/strength_utils.rs`.

**Dipendenze:** `pwd-types` (deve essere completato)

**Stato:** ⏳ NON INIZIATO

**Piano dedicato:** `docs/plans/2026-02-26-extract-pwd-strength.md` (da creare)

### Prerequisiti per avviare Step 2

- [ ] Step 1 completato ✅
- [ ] `cargo test --workspace` passa al 100%
- [ ] Piano dedicato creato e approvato

### Cosa deve contenere il piano dedicato

1. Creazione struttura crate `pwd-strength/`
2. Estrazione `evaluate_password_strength` e varianti
3. Sistema caricamento blacklist con `PWD_BLACKLIST_PATH`
4. Estrazione sezioni (blacklist, length, variety, pattern)
5. Aggiornamento use paths
6. Test di verifica
7. Commit

### Output atteso dal piano

- [ ] Crate `pwd-strength` funzionante
- [ ] Feature flags: `async`, `tracing`
- [ ] Blacklist caricabile da file esterno
- [ ] Tutti i test passano
- [ ] Commit: `feat: extract pwd-strength library`

### Checkpoint post-Step 2

**Dopo completamento, aggiornare i documenti living:**

**Orchestratore (questo file):**
1. Cambiare stato Step 2 da ⏳ a ✅
2. Aggiungere data completamento
3. Compilare sezione "Lezioni Apprese"

**Reference Document (`docs/library-extraction-analysis.md`):**
1. Aggiornare integrazione con pwd-types
2. Documentare configurazione blacklist
3. Aggiungere note per Step 3

---

## Step 3: Estrazione `pwd-crypto`

**Obiettivo:** Estrarre funzioni crittografiche da `utils.rs` e `password_utils.rs`.

**Dipendenze:** `pwd-types` (Step 1)

**Stato:** ⏳ NON INIZIATO

**Piano dedicato:** `docs/plans/2026-02-26-extract-pwd-crypto.md` (da creare)

### Prerequisiti per avviare Step 3

- [ ] Step 1 completato ✅
- [ ] Step 2 completato ✅ (opzionale ma consigliato)
- [ ] `cargo test --workspace` passa al 100%
- [ ] Piano dedicato creato e approvato

### Pre-refactoring richiesto

Prima di estrarre `pwd-crypto`, è necessario:

1. Creare `src/backend/avatar_utils.rs`
2. Spostare funzioni avatar da `utils.rs` (rimangono nel progetto padre)

### Cosa deve contenere il piano dedicato

1. Refactoring avatar (prerequisito)
2. Creazione struttura crate `pwd-crypto/`
3. Estrazione funzioni hash (Argon2)
4. Estrazione funzioni cipher (AES-256-GCM)
5. Creazione `CryptoError` unificato
6. Aggiornamento use paths
7. Test di verifica
8. Commit

### Output atteso dal piano

- [ ] Refactoring avatar completato
- [ ] Crate `pwd-crypto` funzionante
- [ ] Feature flags: `hash`, `cipher`, `full`, `base64`
- [ ] Tutti i test passano
- [ ] Commit: `feat: extract pwd-crypto library`

### Checkpoint post-Step 3

**Dopo completamento, aggiornare i documenti living:**

**Orchestratore (questo file):**
1. Cambiare stato Step 3 da ⏳ a ✅
2. Aggiungere data completamento
3. Compilare sezione "Lezioni Apprese"

**Reference Document (`docs/library-extraction-analysis.md`):**
1. Documentare refactoring avatar
2. Aggiornare CryptoError unificato
3. Preparare riepilogo finale per Step F

---

## Step F: Finalizzazione

**Obiettivo:** Verificare integrità workspace e aggiornare documentazione.

**Stato:** ⏳ NON INIZIATO

### Prerequisiti

- [ ] Tutti gli Step 1-3 completati ✅
- [ ] `cargo test --workspace --all-features` passa al 100%

### Task F.1: Verifica Finale

```bash
cargo test --workspace --all-features
cargo check --workspace --all-features
cargo tree --depth 1
```

### Task F.2: Aggiornare Documentazione

1. Aggiornare `docs/library-extraction-analysis.md` con stato finale
2. Aggiornare `CLAUDE.md` con sezione workspace libraries

### Task F.3: Merge

1. Push branch
2. Creare PR o merge secondo workflow progetto

---

## Lezioni Apprese

> **IMPORTANTE:** Questa sezione viene compilata dopo ogni step completato. Serve a migliorare gli step futuri.

### Dopo Step 0

| Aspetto   | Riscontrato                             | Azione                                 |
|-----------|-----------------------------------------|----------------------------------------|
| Warnings  | 10-12 warnings unused imports/variables | Non bloccanti, da pulire eventualmente |
| Git stato | File docs non committati                | Accettabile, sono documentazione       |

### Dopo Step 1 (pwd-types)

| Aspetto | Riscontrato | Azione |
|---------|-------------|--------|
| Derive sqlx | Le derive `#[sqlx(...)]` richiedono feature sqlx | Usato `#[cfg_attr(feature = "sqlx", derive(...))]` per derive condizionali |
| Dipendenza futures | `SqlxTemplate` richiede crate `futures` | Aggiunto `futures` come dipendenza opzionale della feature sqlx |
| AegisPasswordConfig | Era re-exportato ma non esposto nel lib.rs | Aggiunto all'export pubblico del modulo generator |
| Doctest sqlx-template | I doctest generati falliscono senza chrono | Ignorati per ora (non bloccanti per il progetto principale) |
| Workspace members | Necessario aggiungere prima di verificare compilazione | Anticipato Task 5 all'inizio del Task 2 |
| **chrono per DateTime** | `DateTime<Utc>` in stored.rs richiede crate chrono + feature sqlx/chrono | Aggiunto `chrono` come dipendenza opzionale e feature `chrono` a sqlx |

### Dopo Step 2 (pwd-strength)

| Aspetto | Riscontrato | Azione |
|---------|-------------|--------|
| -       | -           | -      |

### Dopo Step 3 (pwd-crypto)

| Aspetto | Riscontrato | Azione |
|---------|-------------|--------|
| -       | -           | -      |

---

## Workflow Documenti Living

> **CRITICO:** Ogni step aggiorna due documenti che alimentano lo step successivo.

### Documenti da Aggiornare dopo ogni Step

| Documento                                    | Contenuto da Aggiornare                                      |
|----------------------------------------------|--------------------------------------------------------------|
| `docs/plans/*-orchestrator.md` (questo file) | Stato estrazioni, Lezioni Apprese, Checkpoint completati     |
| `docs/library-extraction-analysis.md`        | Problemi riscontrati, Modifiche API, Nuove dipendenze emerse |

### Cosa Registrare nel Reference Document

Dopo ogni step, aggiornare `docs/library-extraction-analysis.md` con:

1. **Problemi incontrati** e soluzioni adottate
2. **Modifiche alle API** rispetto al piano originale
3. **Nuove dipendenze** emerse durante l'implementazione
4. **Aggiornamento mappa delle dipendenze** se necessario
5. **Note per step futuri** (warning, best practice scoperte)

### Input per la skill writing-plans

Quando si crea il piano dedicato per uno step, la skill `superpowers:writing-plans` DEVE leggere:

```
Input files:
1. docs/plans/2026-02-26-library-extraction-orchestrator.md
   → Stato attuale, prerequisiti verificati, lezioni apprese

2. docs/library-extraction-analysis.md
   → Dettagli tecnici living: API, feature flags, dipendenze aggiornate
```

Questo garantisce che ogni piano sia basato su informazioni fresche e tenga conto delle esperienze degli step precedenti.

---

## Dipendenze tra Step

```
Step 0 (Prerequisiti)
    │
    ▼
Step 1 (pwd-types) ◄─────────────────────────────────────┐
    │                                                    │
    ├──▶ Step 2 (pwd-strength) dipende da pwd-types      │
    │                                                    │
    └──▶ Step 3 (pwd-crypto) dipende da pwd-types        │
                                    │                    │
                                    ▼                    │
                              Step F (Finalizzazione) ───┘
```

**Nota:** Step 2 e Step 3 possono essere eseguiti in parallelo dopo Step 1, ma per semplicità questo orchestratore li
esegue sequenzialmente.

---

## Troubleshooting

### Problema: "cannot find crate pwd_types"

Verificare che `pwd-types` sia in `workspace.members` nel `Cargo.toml` root.

### Problema: "feature `sqlx` is required"

Assicurarsi che il crate che usa il tipo abbia la feature abilitata nel `Cargo.toml`.

### Problema: Test falliscono dopo estrazione

1. Verificare che tutti i `use` paths siano aggiornati
2. Verificare feature flags corrette
3. Controllare re-exports in `src/backend/mod.rs`

---

## Feature Flags Riepilogo

### pwd-types

| Feature     | Descrizione                    |
|-------------|--------------------------------|
| `secrecy`   | SecretString support (default) |
| `sqlx`      | Tipi database                  |
| `generator` | PasswordGeneratorConfig        |
| `dioxus`    | FormSecret per UI              |

### pwd-strength

| Feature   | Descrizione              |
|-----------|--------------------------|
| `async`   | Supporto async (default) |
| `tracing` | Logging                  |

### pwd-crypto

| Feature  | Descrizione              |
|----------|--------------------------|
| `hash`   | Argon2 hashing (default) |
| `cipher` | AES-256-GCM              |
| `full`   | Tutto incluso            |
| `base64` | Base64 utilities         |

---

## Variabili Ambiente

```bash
PWD_BLACKLIST_PATH=./assets/10k-most-common.txt
```

---

## Changelog Documento

| Data       | Versione | Modifica                                                            |
|------------|----------|---------------------------------------------------------------------|
| 2026-02-26 | 1.0      | Creazione iniziale orchestratore                                    |
| 2026-02-26 | 1.1      | Aggiunto workflow documenti living e aggiornamento reference doc    |
| 2026-02-26 | 1.2      | Creato piano dettagliato per Step 1 (pwd-types)                     |
| 2026-02-26 | 1.3      | Completato Step 1 (pwd-types), aggiornate lezioni apprese           |
| 2026-02-26 | 1.4      | Fix chrono dependency per pwd-types, aggiornate lezioni apprese     |

---

**Prossima azione:** Creare piano dedicato per Step 2 (pwd-strength) e procedere con l'estrazione.
