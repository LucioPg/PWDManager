# Phase 4: Aggiornamento pipeline di export

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aggiornare `ExportablePassword` e le funzioni di export per includere i nuovi campi `name` e `username`.

**Architecture:** `name` e `username` vengono aggiunti come campi opzionali per compatibilità con file esportati da versioni precedenti.

**Tech Stack:** Rust, serde (JSON/CSV/XML serialization)

---

## Context

### Nuovi campi in ExportablePassword
```rust
pub struct ExportablePassword {
    pub name: String,                    // NUOVO
    pub username: String,                // NUOVO
    pub location: String,
    pub password: String,
    pub notes: Option<String>,
    pub score: Option<u8>,
    pub created_at: Option<String>,
}
```

---

## File Structure

### Files to Modify
- `src/backend/export_types.rs` - DTO e conversion functions
- `src/backend/export.rs` - Export logic (se presente)

---

## Task 1: Aggiornare ExportablePassword struct

**Files:**
- Modify: `src/backend/export_types.rs:45-57`

**Current code:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportablePassword {
    pub location: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub created_at: Option<String>,
}
```

- [ ] **Step 1: Aggiungere campi name e username alla struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportablePassword {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub username: String,
    pub location: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub created_at: Option<String>,
}
```

Nota: `name` e `username` usano `#[serde(default)]` per compatibilità con file importati da versioni vecchie.

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 2: Aggiornare from_stored_raw()

**Files:**
- Modify: `src/backend/export_types.rs:59-71`

**Current code:**
```rust
impl ExportablePassword {
    pub fn from_stored_raw(stored: &pwd_types::StoredRawPassword) -> Self {
        Self {
            location: stored.location.expose_secret().to_string(),
            password: stored.password.expose_secret().to_string(),
            notes: stored.notes.as_ref().map(|n| n.expose_secret().to_string()),
            score: stored.score.map(|s| s.value()),
            created_at: stored.created_at.clone(),
        }
    }
}
```

- [ ] **Step 1: Aggiungere estrazione di name e username**

```rust
impl ExportablePassword {
    pub fn from_stored_raw(stored: &pwd_types::StoredRawPassword) -> Self {
        Self {
            name: stored.name.clone(),
            username: stored.username.expose_secret().to_string(),
            location: stored.location.expose_secret().to_string(),
            password: stored.password.expose_secret().to_string(),
            notes: stored.notes.as_ref().map(|n| n.expose_secret().to_string()),
            score: stored.score.map(|s| s.value()),
            created_at: stored.created_at.clone(),
        }
    }
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 3: Aggiornare to_stored_raw()

**Files:**
- Modify: `src/backend/export_types.rs:73-94`

**Current code:**
```rust
impl ExportablePassword {
    pub fn to_stored_raw(&self, user_id: i64) -> pwd_types::StoredRawPassword {
        use pwd_types::PasswordScore;
        use secrecy::SecretString;
        use uuid::Uuid;

        pwd_types::StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            location: SecretString::new(self.location.clone().into()),
            password: SecretString::new(self.password.clone().into()),
            notes: self.notes.as_ref().map(|n| SecretString::new(n.clone().into())),
            score: self.score.map(PasswordScore::new),
            created_at: self.created_at.clone(),
        }
    }
}
```

- [ ] **Step 1: Aggiungere name e username nella costruzione**

```rust
impl ExportablePassword {
    pub fn to_stored_raw(&self, user_id: i64) -> pwd_types::StoredRawPassword {
        use pwd_types::PasswordScore;
        use secrecy::SecretString;
        use uuid::Uuid;

        pwd_types::StoredRawPassword {
            uuid: Uuid::new_v4(),
            id: None,
            user_id,
            name: self.name.clone(),
            username: SecretString::new(self.username.clone().into()),
            location: SecretString::new(self.location.clone().into()),
            password: SecretString::new(self.password.clone().into()),
            notes: self.notes.as_ref().map(|n| SecretString::new(n.clone().into())),
            score: self.score.map(PasswordScore::new),
            created_at: self.created_at.clone(),
        }
    }
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 4: Verificare export.rs

- [ ] **Step 1: Verificare che export.rs non abbia costruzioni dirette di ExportablePassword**

Run: `grep -n "ExportablePassword" src/backend/export.rs`
Se non ci sono costruzioni dirette di ExportablePassword, nessuna modifica necessaria.

- [ ] **Step 2: Se necessario, aggiornare le costruzioni di ExportablePassword**

---

## Task 5: Commit delle modifiche

- [ ] **Step 1: Verificare compilazione completa**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 2: Commit delle modifiche**

```bash
git add src/backend/export_types.rs src/backend/export.rs
git commit -m "feat(export): add name and username fields to ExportablePassword

- Add name and username to ExportablePassword struct
- Update from_stored_raw() to extract name and username
- Update to_stored_raw() to include name and username
- Use serde(default) for backwards compatibility with old files

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Notes

### Serde attributes
- `#[serde(default)]` su `name` e `username`: se mancano nel file importato, vengono imposti a stringa vuota
- Questo permette di importare file esportati da versioni precedenti

### Ordine campi
I campi `name` e `username` sono all'inizio della struct per:
1. Importanza (identificano la password)
2. Compatibilità con la lettura umana del file JSON/CSV

---

## Verification Checklist

- [ ] `ExportablePassword` ha `name` e `username`
- [ ] `from_stored_raw()` estrae `name` e `username`
- [ ] `to_stored_raw()` include `name` e `username`
- [ ] `#[serde(default)]` applicato per backwards compatibility
- [ ] `cargo check` passa senza errori
- [ ] Commit effettuato
