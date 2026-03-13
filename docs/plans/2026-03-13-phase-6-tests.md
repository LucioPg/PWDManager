# Phase 6: Aggiornamento test password_utils_tests.rs

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aggiornare tutti i test che creano `StoredRawPassword` e chiamano `StoredPassword::new()` per includere `name` e `username`.

**Architecture:** I test verificano che i nuovi campi vengono gestiti correttamente in tutto il flusso encrypt/decrypt.

**Tech Stack:** Rust, tokio test, uuid

---

## Context

### Dipendenze
- **Richiede Phase 2** completata (funzioni encrypt/decrypt con name/username)

### Modifiche necessarie nei test
Ogni costruzione di `StoredRawPassword` deve includere:
- `name: String`
- `username: SecretString`

Ogni chiamata a `StoredPassword::new()` deve includere:
- `name: String`
- `username: SecretBox<[u8]>`
- `username_nonce: Vec<u8>`

---

## File Structure

### Files to Modify
- `src/backend/password_utils_tests.rs` - Test file

---

## Task 1: Aggiornare StoredRawPassword in test_encrypt_decrypt_password

**Files:**
- Modify: `src/backend/password_utils_tests.rs:96-105`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.to_string().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Test Service".to_string(),
    username: SecretString::new("testuser".into()),
    location: SecretString::new(location.to_string().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

---

## Task 2: Aggiornare StoredRawPassword in test_decrypt_invalid_nonce

**Files:**
- Modify: `src/backend/password_utils_tests.rs:357-366`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new("https://test.com".to_string().into()),
    password: raw_password,
    notes: None,
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Test Service".to_string(),
    username: SecretString::new("testuser".into()),
    location: SecretString::new("https://test.com".to_string().into()),
    password: raw_password,
    notes: None,
    score: None,
    created_at: None,
};
```

---

## Task 3: Aggiornare StoredPassword::new in test_decrypt_nonexistent_user

**Files:**
- Modify: `src/backend/password_utils_tests.rs:396-407`

**Current code:**
```rust
let stored_password = StoredPassword::new(
    None,
    99999, // User ID inesistente
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // encrypted location
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // location_nonce
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // password
    None,  // notes
    None,  // notes_nonce
    PasswordScore::new(38),
    None,
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // password_nonce
);
```

- [ ] **Step 1: Aggiungere name, username, username_nonce**

```rust
let stored_password = StoredPassword::new(
    None,
    99999, // User ID inesistente
    "Test Service".to_string(), // name
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // encrypted username
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // username_nonce
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // encrypted location
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // location_nonce
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // password
    None,  // notes
    None,  // notes_nonce
    PasswordScore::new(38),
    None,
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // password_nonce
);
```

---

## Task 4: Aggiornare StoredRawPassword in test_multiple_passwords_for_same_user

**Files:**
- Modify: `src/backend/password_utils_tests.rs:426-437`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(raw_pwd.to_string().into()),
    notes: None,
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: format!("Service {}", location),
    username: SecretString::new("testuser".into()),
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(raw_pwd.to_string().into()),
    notes: None,
    score: None,
    created_at: None,
};
```

---

## Task 5: Aggiornare StoredRawPassword in test_multiple_passwords_for_same_user_with_predefined_strength

**Files:**
- Modify: `src/backend/password_utils_tests.rs:492-503`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(raw_pwd.to_string().into()),
    notes: Some(SecretString::new("Ciao io sono una nota".into())),
    score: strength.score,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: location.to_string(),
    username: SecretString::new("testuser".into()),
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(raw_pwd.to_string().into()),
    notes: Some(SecretString::new("Ciao io sono una nota".into())),
    score: strength.score,
    created_at: None,
};
```

---

## Task 6: Aggiornare StoredRawPassword in test_encrypted_password_is_different_from_original

**Files:**
- Modify: `src/backend/password_utils_tests.rs:537-546`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new("https://encrypted.com".to_string().into()),
    password: SecretString::new(raw_password.into()),
    notes: None,
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Encrypted Service".to_string(),
    username: SecretString::new("encrypteduser".into()),
    location: SecretString::new("https://encrypted.com".to_string().into()),
    password: SecretString::new(raw_password.into()),
    notes: None,
    score: None,
    created_at: None,
};
```

---

## Task 7: Aggiornare StoredRawPassword in test_location_and_notes_are_encrypted

**Files:**
- Modify: `src/backend/password_utils_tests.rs:581-590`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: Some(SecretString::new(notes.clone().unwrap().into())),
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Secret Location Service".to_string(),
    username: SecretString::new("secretuser".into()),
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: Some(SecretString::new(notes.clone().unwrap().into())),
    score: None,
    created_at: None,
};
```

---

## Task 8: Aggiornare StoredRawPassword in test_decrypt_location_and_notes_roundtrip

**Files:**
- Modify: `src/backend/password_utils_tests.rs:633-642`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Roundtrip Service".to_string(),
    username: SecretString::new("roundtripuser".into()),
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

---

## Task 9: Aggiornare StoredRawPassword in test_password_migration_single_password

**Files:**
- Modify: `src/backend/password_utils_tests.rs:693-703`

**Current code:**
```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

- [ ] **Step 1: Aggiungere name e username**

```rust
let stored_raw_password = StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: "Migration Test Service".to_string(),
    username: SecretString::new("migrationuser".into()),
    location: SecretString::new(location.clone().into()),
    password: raw_password.clone(),
    notes: notes.clone(),
    score: None,
    created_at: None,
};
```

---

## Task 10: Aggiornare StoredRawPassword in test_password_migration_multiple_passwords

**Files:**
- Modify: `src/backend/password_utils_tests.rs:760-771`

**Current code:**
```rust
StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(pwd.to_string().into()),
    notes: note.map(|n| SecretString::new(n.into())),
    score: None,
    created_at: None,
}
```

- [ ] **Step 1: Aggiungere name e username**

```rust
StoredRawPassword {
    uuid: Uuid::new_v4(),
    id: None,
    user_id,
    name: location.to_string(),
    username: SecretString::new("multiuser".into()),
    location: SecretString::new(location.to_string().into()),
    password: SecretString::new(pwd.to_string().into()),
    notes: note.map(|n| SecretString::new(n.into())),
    score: None,
    created_at: None,
}
```

---

## Task 11: Aggiungere test roundtrip per name e username

**Files:**
- Add: `src/backend/password_utils_tests.rs` (nuovo test alla fine del file)

**Obiettivo:** Verificare esplicitamente che `name` e `username` vengono preservati correttamente attraverso il ciclo encrypt/decrypt.

- [ ] **Step 1: Aggiungere test roundtrip dedicato**

```rust
#[tokio::test]
async fn test_name_username_roundtrip() {
    let pool = setup_db().await;
    let user_id = setup_user(&pool).await;

    let original_name = "My Test Service".to_string();
    let original_username = "user@domain.com";

    let stored_raw_password = StoredRawPassword {
        uuid: Uuid::new_v4(),
        id: None,
        user_id,
        name: original_name.clone(),
        username: SecretString::new(original_username.into()),
        location: SecretString::new("https://test.com".into()),
        password: SecretString::new("testpassword123".into()),
        notes: None,
        score: None,
        created_at: None,
    };

    // Encrypt and save
    create_stored_data_pipeline_bulk(&pool, user_id, vec![stored_raw_password])
        .await
        .expect("Failed to encrypt password");

    // Fetch and decrypt
    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch passwords");

    let user_auth = get_user_auth(&pool, user_id)
        .await
        .expect("Failed to get user auth");

    let decrypted = decrypt_bulk_stored_data(stored_passwords, &user_auth)
        .expect("Failed to decrypt");

    // Verify roundtrip
    assert_eq!(decrypted.len(), 1);
    assert_eq!(decrypted[0].name, original_name);
    assert_eq!(decrypted[0].username.expose_secret(), original_username);
}
```

- [ ] **Step 2: Verificare che il test passi**

Run: `cargo test --lib password_utils_tests::test_name_username_roundtrip`
Expected: PASS

---

## Task 12: Verifica finale

- [ ] **Step 1: Eseguire cargo check**

Run: `cargo check`
Expected: Nessun errore

- [ ] **Step 2: Eseguire tutti i test**

Run: `cargo test --lib password_utils_tests`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/backend/password_utils_tests.rs
git commit -m "test(password): update all tests with name and username fields

- Update all StoredRawPassword constructions in tests
- Update StoredPassword::new call in test_decrypt_nonexistent_user
- Add name and username fields to all test fixtures
- Add explicit roundtrip test for name and username

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | File | Lines | Type |
|------|------|-------|------|
| 1 | password_utils_tests.rs | 96-105 | StoredRawPassword |
| 2 | password_utils_tests.rs | 357-366 | StoredRawPassword |
| 3 | password_utils_tests.rs | 396-407 | StoredPassword::new |
| 4 | password_utils_tests.rs | 426-437 | StoredRawPassword |
| 5 | password_utils_tests.rs | 492-503 | StoredRawPassword |
| 6 | password_utils_tests.rs | 537-546 | StoredRawPassword |
| 7 | password_utils_tests.rs | 581-590 | StoredRawPassword |
| 8 | password_utils_tests.rs | 633-642 | StoredRawPassword |
| 9 | password_utils_tests.rs | 693-703 | StoredRawPassword |
| 10 | password_utils_tests.rs | 760-771 | StoredRawPassword |
| 11 | password_utils_tests.rs | nuovo | Test roundtrip |

---

## Verification Checklist

- [ ] Tutte le costruzioni `StoredRawPassword` hanno `name` e `username`
- [ ] Chiamata `StoredPassword::new` aggiornata con nuovi parametri
- [ ] Test roundtrip `test_name_username_roundtrip` aggiunto e passa
- [ ] `cargo check` passa
- [ ] `cargo test --lib password_utils_tests` passa
- [ ] Commit effettuato
