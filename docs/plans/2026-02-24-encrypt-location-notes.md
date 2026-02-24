# Encrypt Location and Notes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend AES-256-GCM encryption to `location` and `notes` fields in `StoredPassword`, currently only `password` is encrypted.

**Architecture:** Each encrypted field gets its own 12-byte nonce (unique constraint in DB). Same AES key derived from user's master password, but different nonces for each field. This is cryptographically safe for AES-GCM.

**Tech Stack:** Rust, AES-256-GCM (aes_gcm crate), Argon2 KDF, SQLite, sqlx-template

---

## Task 1: Update Database Schema

**Files:**
- Modify: `src/backend/init_queries.rs:47-57`

**Step 1: Update CREATE TABLE query**

Replace the passwords table definition:

```rust
"CREATE TABLE IF NOT EXISTS passwords (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    location BLOB NOT NULL,
    location_nonce BLOB NOT NULL UNIQUE,
    password BLOB NOT NULL,
    password_nonce BLOB NOT NULL UNIQUE,
    notes BLOB,
    notes_nonce BLOB UNIQUE,
    score INTEGER NOT NULL CHECK (0 <= score <= 100),
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
)",
```

**Step 2: Verify schema compiles**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/init_queries.rs
git commit -m "feat(db): update passwords schema for encrypted location and notes"
```

---

## Task 2: Update StoredPassword Struct AND All References

**Files:**
- Modify: `src/backend/password_types_helper.rs:187-237`
- Modify: `src/backend/password_utils.rs` (all references to `nonce` field)

**IMPORTANT**: This task must be done atomically - update both the struct AND all references to prevent compilation errors.

**Step 1: Update struct fields**

Replace the `StoredPassword` struct definition:

```rust
#[derive(sqlx::FromRow, Debug, Clone, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]
#[tp_upsert(by = "id")]
#[tp_select_builder]
pub struct StoredPassword {
    pub id: Option<i64>,
    pub user_id: i64,
    pub location: DbSecretVec,
    pub location_nonce: Vec<u8>,
    pub password: DbSecretVec,
    pub password_nonce: Vec<u8>,  // Renamed from 'nonce'
    pub notes: Option<DbSecretVec>,
    pub notes_nonce: Option<Vec<u8>>,
    pub score: PasswordScore,
    pub created_at: Option<String>,
}
```

**Step 2: Update constructor**

Replace `StoredPassword::new()`:

```rust
impl StoredPassword {
    pub fn new(
        id: Option<i64>,
        user_id: i64,
        location: SecretBox<[u8]>,
        location_nonce: Vec<u8>,
        password: SecretBox<[u8]>,
        notes: Option<SecretBox<[u8]>>,
        notes_nonce: Option<Vec<u8>>,
        score: PasswordScore,
        created_at: Option<String>,
        password_nonce: Vec<u8>,
    ) -> Self {
        let location: DbSecretVec = location.into();
        let password: DbSecretVec = password.into();
        let notes: Option<DbSecretVec> = notes.map(|n| n.into());

        StoredPassword {
            id,
            user_id,
            location,
            location_nonce,
            password,
            password_nonce,
            notes,
            notes_nonce,
            score,
            created_at,
        }
    }
}
```

**Step 3: Update ALL references to `nonce` field in password_utils.rs**

In `decrypt_bulk_stored_passwords` function (~line 302):
```rust
// Change:
let nonce = get_nonce_from_vec(&sp.nonce)?;
// To:
let nonce = get_nonce_from_vec(&sp.password_nonce)?;
```

In `decrypt_stored_password` function (~line 347):
```rust
// Change:
let nonce = get_nonce_from_vec(&stored_password.nonce)?;
// To:
let nonce = get_nonce_from_vec(&stored_password.password_nonce)?;
```

In `create_stored_passwords` function (~line 268):
```rust
// The nonce field is already correct as it creates new nonces
// No change needed here
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors (all references updated atomically)

**Step 5: Commit**

```bash
git add src/backend/password_types_helper.rs src/backend/password_utils.rs
git commit -m "feat(types): update StoredPassword with encrypted fields and all references"
```

---

## Task 3: Add Encryption Helper Functions

**Files:**
- Modify: `src/backend/password_utils.rs` (after line 100)

**Step 1: Add encrypt_string function**

Add after `get_nonce_from_vec` function:

```rust
/// Cripta una stringa con AES-256-GCM.
///
/// # Parametri
/// * `plaintext` - La stringa in chiaro da criptare
/// * `cipher` - Il cipher AES-256-GCM inizializzato
///
/// # Valore Restituito
/// Tupla (encrypted_bytes, nonce)
fn encrypt_string(
    plaintext: &str,
    cipher: &Aes256Gcm,
) -> Result<(SecretBox<[u8]>, Nonce<Aes256Gcm>), DBError> {
    let nonce = create_nonce();
    let encrypted = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| DBError::new_cipher_encryption_error(e.to_string()))?;
    Ok((SecretBox::new(encrypted.into()), nonce))
}

/// Cripta una stringa opzionale con AES-256-GCM.
fn encrypt_optional_string(
    plaintext: Option<&str>,
    cipher: &Aes256Gcm,
) -> Result<(Option<SecretBox<[u8]>>, Option<Nonce<Aes256Gcm>>), DBError> {
    match plaintext {
        Some(text) => {
            let (encrypted, nonce) = encrypt_string(text, cipher)?;
            Ok((Some(encrypted), Some(nonce)))
        }
        None => Ok((None, None)),
    }
}
```

**Step 2: Add decrypt helper functions**

Add after encrypt functions:

```rust
/// Decripta bytes in una stringa UTF-8.
fn decrypt_to_string(
    encrypted: &[u8],
    nonce: &Nonce<Aes256Gcm>,
    cipher: &Aes256Gcm,
) -> Result<String, DBError> {
    let plaintext_bytes = cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
    String::from_utf8(plaintext_bytes)
        .map_err(|e| DBError::new_password_conversion_error(e.to_string()))
}

/// Decripta bytes opzionali in una stringa opzionale.
fn decrypt_optional_to_string(
    encrypted: Option<&[u8]>,
    nonce: Option<&Nonce<Aes256Gcm>>,
    cipher: &Aes256Gcm,
) -> Result<Option<String>, DBError> {
    match (encrypted, nonce) {
        (Some(enc), Some(n)) => {
            let decrypted = decrypt_to_string(enc, n, cipher)?;
            Ok(Some(decrypted))
        }
        _ => Ok(None),
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): add encrypt/decrypt helper functions for strings"
```

---

## Task 4: Rename Functions from "password" to "data"

**Files:**
- Modify: `src/backend/password_utils.rs`

**Step 1: Rename create_stored_password_pipeline**

Find function at line ~185, rename to `create_stored_data_pipeline`:

```rust
pub async fn create_stored_data_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    score: Option<PasswordScore>,
) -> Result<(), DBError> {
```

Keep old name as alias for backward compatibility:

```rust
/// Deprecated: Use `create_stored_data_pipeline` instead
pub async fn create_stored_password_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    score: Option<PasswordScore>,
) -> Result<(), DBError> {
    create_stored_data_pipeline(pool, user_id, location, raw_password, notes, score).await
}
```

**Step 2: Rename decrypt_bulk_stored_passwords**

Find function at line ~291, rename to `decrypt_bulk_stored_data`:

```rust
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
```

Add alias:

```rust
/// Deprecated: Use `decrypt_bulk_stored_data` instead
pub async fn decrypt_bulk_stored_passwords(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    decrypt_bulk_stored_data(user_auth, stored_passwords).await
}
```

**Step 3: Rename create_stored_passwords**

Find function at line ~229, rename to `create_stored_data_records`:

```rust
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
```

Add alias:

```rust
/// Deprecated: Use `create_stored_data_records` instead
pub async fn create_stored_passwords(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
    create_stored_data_records(cipher, user_auth, stored_raw_passwords).await
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors (aliases maintain compatibility)

**Step 5: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "refactor(crypto): rename functions from password to data"
```

---

## Task 5: Update create_stored_data_pipeline with Encryption

**Files:**
- Modify: `src/backend/password_utils.rs` (function starting at ~185)

**Step 1: Implement full encryption in pipeline**

Replace the function body:

```rust
pub async fn create_stored_data_pipeline(
    pool: &SqlitePool,
    user_id: i64,
    location: String,
    raw_password: SecretString,
    notes: Option<String>,
    score: Option<PasswordScore>,
) -> Result<(), DBError> {
    // 1. Recupero credenziali e setup crittografico
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;

    // 2. Cripta location
    let (encrypted_location, location_nonce) = encrypt_string(&location, &cipher)?;

    // 3. Cripta password
    let password_nonce = create_nonce();
    let encrypted_password = create_password_with_cipher(&raw_password, &password_nonce, &cipher)
        .await
        .map_err(|_| DBError::new_password_save_error("Errore durante la criptazione".into()))?;

    // 4. Cripta notes
    let (encrypted_notes, notes_nonce) = encrypt_optional_string(notes.as_deref(), &cipher)?;

    // 5. Determinazione del punteggio
    let password_score = score.unwrap_or_else(|| {
        evaluate_password_strength(&raw_password, None)
            .score
            .unwrap_or(PasswordScore::new(0))
    });

    // 6. Creazione della struct
    let stored_password = StoredPassword::new(
        None,
        user_id,
        encrypted_location,
        location_nonce.to_vec(),
        encrypted_password,
        encrypted_notes,
        notes_nonce.map(|n| n.to_vec()),
        password_score,
        None,
        password_nonce.to_vec(),
    );

    // 7. Persistenza
    save_or_update_stored_password(pool, stored_password).await?;

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): encrypt location and notes in pipeline"
```

---

## Task 6: Update decrypt_bulk_stored_data with Decryption

**Files:**
- Modify: `src/backend/password_utils.rs` (function starting at ~291)

**Step 1: Implement full decryption**

Replace the function body:

```rust
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    let cipher = Arc::new(cipher);

    task::spawn_blocking(move || {
        stored_passwords
            .into_par_iter()
            .map(|sp| {
                // Decripta location
                let location_nonce = get_nonce_from_vec(&sp.location_nonce)?;
                let location = decrypt_to_string(
                    sp.location.expose_secret().as_ref(),
                    &location_nonce,
                    &cipher,
                )?;

                // Decripta password
                let password_nonce = get_nonce_from_vec(&sp.password_nonce)?;
                let password_bytes = cipher
                    .decrypt(&password_nonce, sp.password.expose_secret().as_ref())
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;
                let password = String::from_utf8(password_bytes)
                    .map_err(|e| DBError::new_password_conversion_error(e.to_string()))?;

                // Decripta notes
                let notes = match (&sp.notes, &sp.notes_nonce) {
                    (Some(enc_notes), Some(nn)) => {
                        let notes_nonce = get_nonce_from_vec(nn)?;
                        decrypt_optional_to_string(
                            Some(enc_notes.expose_secret().as_ref()),
                            Some(&notes_nonce),
                            &cipher,
                        )?
                    }
                    _ => None,
                };

                Ok(StoredRawPassword {
                    id: sp.id,
                    user_id: user_auth.id,
                    location,
                    password: SecretString::new(password.into()),
                    notes,
                    score: Some(sp.score),
                    created_at: sp.created_at,
                })
            })
            .collect::<Result<Vec<StoredRawPassword>, DBError>>()
    })
    .await
    .map_err(|e| DBError::new_password_conversion_error(format!("Join error: {}", e)))?
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): decrypt location and notes in bulk function"
```

---

## Task 7: Update create_stored_data_records (Bulk Creation)

**Files:**
- Modify: `src/backend/password_utils.rs` (function starting at ~229)

**Step 1: Implement full encryption in bulk**

Replace the function body:

```rust
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<Vec<StoredPassword>, DBError> {
    if stored_raw_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let cipher = Arc::new(cipher);
    let user_auth = Arc::new(user_auth);

    task::spawn_blocking(move || {
        stored_raw_passwords
            .into_par_iter()
            .map(|srp| {
                // Cripta location
                let (encrypted_location, location_nonce) =
                    encrypt_string(&srp.location, &cipher)?;

                // Cripta password
                let password_nonce = create_nonce();
                let encrypted_password = create_password_with_cipher_sync(
                    &srp.password, &password_nonce, &cipher
                ).map_err(|_| {
                    DBError::new_cipher_encryption_error("Cipher error".to_string())
                })?;

                // Cripta notes
                let (encrypted_notes, notes_nonce) = encrypt_optional_string(
                    srp.notes.as_deref(), &cipher
                )?;

                // Calcola score
                let score_evaluation: PasswordScore = srp.score.unwrap_or_else(|| {
                    evaluate_password_strength(&srp.password, None)
                        .score
                        .unwrap_or(PasswordScore::new(0))
                });

                Ok(StoredPassword::new(
                    srp.id,
                    user_auth.id,
                    encrypted_location,
                    location_nonce.to_vec(),
                    encrypted_password,
                    encrypted_notes,
                    notes_nonce.map(|n| n.to_vec()),
                    score_evaluation,
                    None,
                    password_nonce.to_vec(),
                ))
            })
            .collect::<Result<Vec<StoredPassword>, DBError>>()
    })
    .await
    .map_err(|e| DBError::new_password_save_error(format!("Join error: {}", e)))?
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat(crypto): encrypt location and notes in bulk creation"
```

---

## Task 8: Update Tests

**Files:**
- Modify: `src/backend/password_utils_tests.rs`

**Step 1: Update test_encrypt_decrypt_password**

Update assertions for encrypted location:

```rust
// Remove these lines (location is now encrypted):
// assert_eq!(stored_password.location, location);
// assert_eq!(stored_password.notes, notes);

// Add check that location is encrypted (not plaintext):
let location_bytes = stored_password.location.expose_secret();
assert_ne!(
    location_bytes.as_ref(),
    location.as_bytes(),
    "Location should be encrypted"
);

// Add nonce checks:
assert_eq!(stored_password.location_nonce.len(), 12, "Location nonce should be 12 bytes");
assert_eq!(stored_password.password_nonce.len(), 12, "Password nonce should be 12 bytes");
```

**Step 2: Update test_encrypt_decrypt_password_rayon**

Same changes as Step 1 for the rayon test.

**Step 3: Update test_decrypt_invalid_nonce**

Change `stored_password.nonce` to `stored_password.password_nonce`:

```rust
stored_password.password_nonce = vec![1, 2, 3]; // Solo 3 byte invece di 12
```

**Step 4: Update test_decrypt_nonexistent_user**

Update StoredPassword::new call:

```rust
let stored_password = StoredPassword::new(
    None,
    99999,
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // encrypted location
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // location_nonce
    SecretBox::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into()), // password
    None, // notes
    None, // notes_nonce
    PasswordScore::new(38),
    None,
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], // password_nonce
);
```

**Step 5: Update test_multiple_passwords_for_same_user**

Remove location comparison (now encrypted):

```rust
// Remove: assert_eq!(stored.location, *expected_location);
// Location will need to be decrypted first, or just verify count
```

**Step 6: Update test_multiple_passwords_for_same_user_with_predefined_strength**

Same changes as Step 5.

**Step 7: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 8: Commit**

```bash
git add src/backend/password_utils_tests.rs
git commit -m "test(crypto): update tests for encrypted location and notes"
```

---

## Task 9: Add New Test for Location/Notes Encryption

**Files:**
- Modify: `src/backend/password_utils_tests.rs` (append to file)

**Step 1: Add test for encrypted location/notes**

```rust
#[tokio::test]
async fn test_location_and_notes_are_encrypted() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_enc", "MasterPass123!").await;

    let raw_password = SecretString::new("MySecurePassword456".into());
    let location = "https://secret-location.com".to_string();
    let notes = Some("Confidential notes".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt data");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch stored passwords");

    let stored = &stored_passwords[0];

    // Verify location is NOT plaintext
    let location_bytes = stored.location.expose_secret();
    assert_ne!(
        location_bytes.as_ref(),
        location.as_bytes(),
        "Location should be encrypted"
    );

    // Verify notes are NOT plaintext
    if let Some(notes_enc) = &stored.notes {
        assert_ne!(
            notes_enc.expose_secret().as_ref(),
            notes.as_ref().unwrap().as_bytes(),
            "Notes should be encrypted"
        );
    }

    // Verify nonces are 12 bytes
    assert_eq!(stored.location_nonce.len(), 12);
    assert_eq!(stored.password_nonce.len(), 12);
    if let Some(nn) = &stored.notes_nonce {
        assert_eq!(nn.len(), 12);
    }
}
```

**Step 2: Add test for decryption roundtrip**

```rust
#[tokio::test]
async fn test_decrypt_location_and_notes_roundtrip() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "testuser_rt", "MasterPass123!").await;

    let raw_password = SecretString::new("MyPassword".into());
    let location = "MySecretService".to_string();
    let notes = Some("My secret notes".to_string());

    create_stored_password_pipeline(
        &pool,
        user_id,
        location.clone(),
        raw_password.clone(),
        notes.clone(),
        None,
    )
    .await
    .expect("Failed to encrypt");

    let stored_passwords = fetch_all_stored_passwords_for_user(&pool, user_id)
        .await
        .expect("Failed to fetch");

    let user_auth = UserAuth {
        id: user_id,
        password: DbSecretString(SecretString::from("MasterPass123!")),
    };

    let decrypted = decrypt_bulk_stored_data(user_auth, stored_passwords)
        .await
        .expect("Failed to decrypt");

    assert_eq!(decrypted.len(), 1);
    assert_eq!(decrypted[0].location, location);
    assert_eq!(decrypted[0].notes, notes);
    assert_eq!(decrypted[0].password.expose_secret(), raw_password.expose_secret());
}
```

**Step 3: Run new tests**

Run: `cargo test test_location_and_notes_are_encrypted test_decrypt_location_and_notes_roundtrip`
Expected: Both tests pass

**Step 4: Commit**

```bash
git add src/backend/password_utils_tests.rs
git commit -m "test(crypto): add tests for location and notes encryption"
```

---

## Task 10: Final Verification

**Files:**
- All modified files

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Build the application**

Run: `cargo build`
Expected: Build succeeds

**Step 3: Run the application**

Run: `dx serve --desktop`
Expected: App starts, can create/view passwords

**Step 4: Manual verification**

1. Create a new password entry with location and notes
2. Close the app
3. Open the SQLite database directly with a tool (e.g., DB Browser for SQLite)
4. Verify `location`, `password`, and `notes` columns contain BLOB data that is NOT readable text
5. Reopen the app and verify the data is displayed correctly (decrypted)

---

## Summary of Changes

| File | Changes |
|------|---------|
| `init_queries.rs` | Schema: location/notes become BLOB, add nonce columns |
| `password_types_helper.rs` | StoredPassword: location → DbSecretVec, add nonce fields |
| `password_utils.rs` | Add encrypt_string/decrypt_to_string helpers |
| `password_utils.rs` | Rename functions: password → data |
| `password_utils.rs` | Update pipeline to encrypt location + notes |
| `password_utils.rs` | Update decrypt_bulk to decrypt all fields |
| `password_utils.rs` | Update all nonce field references (Task 2, atomic) |
| `password_utils_tests.rs` | Update all tests for new structure |

---

## Notes

- **User handles DB drop**: The user will manually drop the database before running the new code
- **Backward compatibility aliases**: Old function names work via aliases during transition
- **Nonce uniqueness**: Each nonce column has UNIQUE constraint to guarantee AES-GCM security
