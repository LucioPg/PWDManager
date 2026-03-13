# Phase 5: Aggiornamento pipeline di import

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verificare che la pipeline di import gestisca correttamente i nuovi campi `name` e `username`.

**Architecture:** L'import usa `ExportablePassword.to_stored_raw()` (aggiornata in Phase 4). La deduplicazione rimane basata su `(location, password)`.

**Tech Stack:** Rust, serde, csv, quick-xml

---

## Context

### Flusso Import
```
File (JSON/CSV/XML)
       ↓ parse_passwords()
ExportablePassword (con name, username)
       ↓ deduplicate_passwords()
ExportablePassword (unici nel file)
       ↓ confronto con existing DB
ExportablePassword (nuovi)
       ↓ to_stored_raw() [Phase 4]
StoredRawPassword
       ↓ create_stored_data_records() [Phase 2]
StoredPassword (criptato)
       ↓ upsert_stored_passwords_batch()
Database
```

### Deduplicazione
La logica di deduplicazione rimane basata su `(location, password)`:
- `name` è un identificativo display, non univoco
- `username` è dati aggiuntivi, non usato per deduplicazione

---

## File Structure

### Files to Verify
- `src/backend/import.rs` - Import pipeline

### Files Already Updated (Phase 4)
- `src/backend/export_types.rs` - `ExportablePassword` e `to_stored_raw()`

---

## Task 1: Aggiornare test create_test_password()

**Files:**
- Modify: `src/backend/import.rs:338-346`

**Current code:**
```rust
fn create_test_password() -> ExportablePassword {
    ExportablePassword {
        location: "example.com".to_string(),
        password: "secret123".to_string(),
        notes: Some("test notes".to_string()),
        score: Some(85),
        created_at: Some("2024-01-01".to_string()),
    }
}
```

- [ ] **Step 1: Aggiungere name e username al'interno del test helper**

```rust
fn create_test_password() -> ExportablePassword {
    ExportablePassword {
        name: "Example Service".to_string(),
        username: "user@example.com".to_string(),
        location: "example.com".to_string(),
        password: "secret123".to_string(),
        notes: Some("test notes".to_string()),
        score: Some(85),
        created_at: Some("2024-01-01".to_string()),
    }
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 2: Verificare deduplicazione

**Files:**
- Read: `src/backend/import.rs:74-92`

**Current logic:**
```rust
pub fn deduplicate_passwords(
    passwords: Vec<ExportablePassword>,
) -> (Vec<ExportablePassword>, usize) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    let original_count = passwords.len();

    for pwd in passwords {
        let key = (pwd.location.clone(), pwd.password.clone());
        if seen.insert(key) {
            unique.push(pwd);
        }
    }

    let duplicates_count = original_count - unique.len();
    (unique, duplicates_count)
}
```

- [ ] **Step 1: Verificare che la deduplicazione rimane corretta**

La logica attuale deduplica per `(location, password)`. Questo è **corretto** perché:
- `name` è solo display, non identificativo
- `username` può variare per stesso servizio

**Nessuna modifica richiesta.**

---

## Task 3: Verificare test import

**Files:**
- Read: `src/backend/import.rs:349-405` (test module)

- [ ] **Step 1: Verificare test parse con nuovi campi**

I test esistenti testano solo `location` e `password`. Verificare che passino:

Run: `cargo test --lib import::tests`
Expected: PASS

- [ ] **Step 2: Aggiungere test con name e username**

Se si vuole testare esplicitamente i nuovi campi, aggiungere:

```rust
#[test]
fn test_parse_from_json_with_name_username() {
    let json = r#"[{"name":"GitHub","username":"devuser","location":"github.com","password":"pass123"}]"#;
    let result = parse_from_json(json);
    assert!(result.is_ok());
    let passwords = result.unwrap();
    assert_eq!(passwords.len(), 1);
    assert_eq!(passwords[0].name, "GitHub");
    assert_eq!(passwords[0].username, "devuser");
}
```

---

## Task 4: Verifica finale

- [ ] **Step 1: Eseguire cargo check**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 2: Eseguire test import**

Run: `cargo test --lib import`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/backend/import.rs
git commit -m "test(import): update test helper with name and username fields

- Add name and username to create_test_password() helper
- Verify deduplication logic remains correct for new fields
- Add optional test for parsing name/username from JSON

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Notes

### Dipendenze
- **Richiede Phase 4** completata (`ExportablePassword` aggiornato)
- **Richiede Phase 2** completata (`create_stored_data_records` gestisce `username`)

### Deduplicazione
La chiave di deduplicazione `(location, password)` è **intenzionale**:
- Due voci con stesso `location` + `password` = duplicato
- `name` e `username` non influenzano l'unicità

### Backwards compatibility
Grazie a `#[serde(default)]` in Phase 4:
- File senza `name` → stringa vuota
- File senza `username` → stringa vuota

---

## Verification Checklist

- [ ] `create_test_password()` ha `name` e `username`
- [ ] Deduplicazione verificata (nessuna modifica necessaria)
- [ ] Test esistenti passano
- [ ] `cargo check` passa
- [ ] Commit effettuato
