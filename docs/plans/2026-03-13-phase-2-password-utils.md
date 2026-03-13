# Phase 2: Aggiornamento password_utils.rs

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aggiornare le funzioni di encryption/decryption per gestire i nuovi campi `name` e `username`.

**Architecture:** `username` segue lo stesso pattern di `location`: criptato con AES-256-GCM con nonce dedicato. `name` è testo in chiaro.

**Tech Stack:** Rust, pwd-crypto, rayon (parallel processing)

---

## Context

### Mapping campi
| Campo | Tipo DB | Tipo Rust | Criptato |
|-------|---------|-----------|----------|
| `name` | TEXT | `String` | ❌ No |
| `username` | BLOB | `SecretString` → `DbSecretVec` | ✅ Sì |
| `username_nonce` | BLOB | `Vec<u8>` | N/A |

### Pattern esistente per `location` (da replicare per `username`)
```rust
// Encryption
let (encrypted_location, location_nonce) =
    encrypt_string(srp.location.expose_secret(), &cipher)?;

// Decryption
let location_nonce = get_nonce_from_vec(&sp.location_nonce)?;
let location = decrypt_to_string(
    sp.location.expose_secret().as_ref(),
    &location_nonce,
    &cipher,
)?;
```

---

## File Structure

### Files to Modify
- `src/backend/password_utils.rs` - Funzioni encryption/decryption

---

## Task 1: Aggiornare create_stored_data_records() - aggiungere encryption username

**Files:**
- Modify: `src/backend/password_utils.rs:158-236`

**Obiettivo:** Criptare `username` con lo stesso pattern di `location`

- [ ] **Step 1: Aggiungere encryption per username (dopo encryption di location)**

Nel blocco `stored_raw_passwords.into_par_iter().map(|srp| {`, dopo la riga che cripta `location`:

```rust
// Cripta username
let (encrypted_username, username_nonce) =
    encrypt_string(srp.username.expose_secret(), &cipher)?;
```

- [ ] **Step 2: Aggiornare StoredPassword::new() con i nuovi parametri**

Sostituire la chiamata esistente:

```rust
Ok(StoredPassword::new(
    srp.id,
    user_auth.id,
    srp.name.clone(),                    // NUOVO: name in chiaro
    encrypted_username,                  // NUOVO: username criptato
    username_nonce.to_vec(),             // NUOVO: nonce username
    encrypted_location,
    location_nonce.to_vec(),
    encrypted_password,
    encrypted_notes,
    notes_nonce.map(|n| n.to_vec()),
    score_evaluation,
    created_at,
    password_nonce.to_vec(),
))
```

**Nota:** L'ordine dei parametri deve corrispondere alla firma di `StoredPassword::new()` in pwd-types:
```rust
pub fn new(
    id: Option<i64>,
    user_id: i64,
    name: String,
    username: SecretBox<[u8]>,
    username_nonce: Vec<u8>,
    location: SecretBox<[u8]>,
    location_nonce: Vec<u8>,
    password: SecretBox<[u8]>,
    notes: Option<SecretBox<[u8]>>,
    notes_nonce: Option<Vec<u8>>,
    score: PasswordScore,
    created_at: Option<String>,
    password_nonce: Vec<u8>,
) -> Self
```

- [ ] **Step 3: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 2: Aggiornare decrypt_bulk_stored_data() - aggiungere decryption username

**Files:**
- Modify: `src/backend/password_utils.rs:318-405`

**Obiettivo:** Decriptare `username` con lo stesso pattern di `location`

- [ ] **Step 1: Aggiungere decryption per username (dopo decryption di location)**

Nel blocco `stored_passwords.into_par_iter().map(|sp| {`, dopo la decryption di `location`:

```rust
// Decripta username
let username_nonce = get_nonce_from_vec(&sp.username_nonce)?;
let username = decrypt_to_string(
    sp.username.expose_secret().as_ref(),
    &username_nonce,
    &cipher,
)?;
```

- [ ] **Step 2: Aggiornare StoredRawPassword construction con i nuovi campi**

Sostituire la costruzione esistente:

```rust
Ok(StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: sp.id,
    user_id: user_auth.id,
    name: sp.name.clone(),               // NUOVO: name in chiaro
    username: SecretString::new(username.into()),  // NUOVO
    location: SecretString::new(location.into()),
    password: SecretString::new(password.into()),
    notes: notes.map(|n| SecretString::new(n.into())),
    score: Some(sp.score),
    created_at: sp.created_at,
})
```

- [ ] **Step 3: Verificare compilazione**

Run: `cargo check`
Expected: Nessun errore

---

## Task 3: Verifica completa password_utils.rs

- [ ] **Step 1: Eseguire cargo check**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 2: Commit delle modifiche**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): add username encryption/decryption support

- Update create_stored_data_records() to encrypt username
- Update decrypt_bulk_stored_data() to decrypt username
- Update StoredPassword::new() calls with name and username
- Update StoredRawPassword construction with name and username

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Notes

### Dipendenze
- Questo piano richiede che **Phase 1** sia completata (query DB aggiornate)
- `StoredPassword::new()` e `StoredRawPassword` devono essere già aggiornati in `pwd-types`

### Campo `name`
- Non viene criptato (è un identificativo leggibile)
- Viene passato direttamente da `srp.name` a `sp.name`

---

## Verification Checklist

- [ ] `create_stored_data_records()` cripta `username`
- [ ] `decrypt_bulk_stored_data()` decripta `username`
- [ ] `StoredPassword::new()` chiamato con tutti i nuovi parametri
- [ ] `StoredRawPassword` costruito con `name` e `username`
- [ ] `cargo check` passa senza errori
- [ ] Commit effettuato
