# Password Migration Progress Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implementare feedback visivo con progress bar per la migrazione password quando l'utente modifica la master
password.

**Architecture:** Canale mpsc Tokio per comunicare progresso dal backend (Rayon par_iter) al frontend (Dioxus Signals).
La pipeline esistente viene estesa con parametro opzionale per retrocompatibilità.

**Tech Stack:** Rust, Tokio mpsc, Rayon, Dioxus 0.7, DaisyUI 5

---

## Task 1: Creare tipi MigrationStage e ProgressMessage

**Files:**

- Create: `src/backend/migration_types.rs`
- Modify: `src/backend/mod.rs`

**Step 1: Creare il file migration_types.rs**

```rust
//! Tipi per il tracking del progresso della migrazione password.

use tokio::sync::mpsc::Sender;

/// Rappresenta lo stage corrente della migrazione password.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum MigrationStage {
    #[default]
    Idle,
    Decrypting,
    Encrypting,
    Finalizing,
    Completed,
    Failed,
}

/// Messaggio di progresso inviato tramite canale mpsc.
#[derive(Clone, Debug)]
pub struct ProgressMessage {
    pub stage: MigrationStage,
    pub current: usize,
    pub total: usize,
}

impl ProgressMessage {
    /// Crea un nuovo messaggio di progresso.
    pub fn new(stage: MigrationStage, current: usize, total: usize) -> Self {
        Self { stage, current, total }
    }

    /// Calcola la percentuale di completamento (0-100).
    pub fn percentage(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.current * 100) / self.total
        }
    }
}

/// Type alias per il sender del progresso.
pub type ProgressSender = Sender<ProgressMessage>;
```

**Step 2: Aggiungere il modulo in mod.rs**

Aggiungere in `src/backend/mod.rs`:

```rust
pub mod migration_types;
```

**Step 3: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita senza errori

**Step 4: Commit**

```bash
git add src/backend/migration_types.rs src/backend/mod.rs
git commit -m "feat: add MigrationStage and ProgressMessage types for progress tracking"
```

---

## Task 2: Aggiungere funzione restore_old_password in db_backend.rs

**Files:**

- Modify: `src/backend/db_backend.rs`

**Step 1: Aggiungere la funzione restore_old_password**

Aggiungere dopo la funzione `remove_temp_old_password` (circa riga 871):

```rust
/// Ripristina la vecchia password dalla colonna temp_old_password.
/// Utilizzato quando la migrazione fallisce per ripristinare lo stato precedente.
pub async fn restore_old_password(pool: &SqlitePool, user_id: i64) -> Result<(), DBError> {
    query(
        r#"
        UPDATE users
        SET password = temp_old_password,
            temp_old_password = NULL
        WHERE id = ?
        "#,
    )
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            DBError::new_update_error(format!("Failed to restore old password: {}", e))
        })?;

    Ok(())
}
```

**Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita senza errori

**Step 3: Commit**

```bash
git add src/backend/db_backend.rs
git commit -m "feat: add restore_old_password function for migration rollback"
```

---

## Task 3: Aggiornare decrypt_bulk_stored_data con progress tracking

**Files:**

- Modify: `src/backend/password_utils.rs`

**Step 1: Aggiungere import per migration_types**

Aggiungere all'inizio del file dopo gli use esistenti:

```rust
use crate::backend::migration_types::{MigrationStage, ProgressMessage, ProgressSender};
```

**Step 2: Modificare la signature di decrypt_bulk_stored_data**

Cercare la funzione `decrypt_bulk_stored_data` (circa riga 229) e modificare:

```rust
/// Decripta in parallelo un batch di StoredPassword.
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
    progress_tx: Option<Arc<ProgressSender>>,  // NUOVO PARAMETRO
) -> Result<Vec<StoredRawPassword>, DBError> {
```

**Step 3: Aggiungere progress tracking nel par_iter**

Modificare il body della funzione per includere progress tracking:

```rust
/// Decripta in parallelo un batch di StoredPassword.
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<Vec<StoredRawPassword>, DBError> {
    if stored_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    let cipher = Arc::new(cipher);
    let total = stored_passwords.len();
    let completed = Arc::new(AtomicUsize::new(0));
    let progress_tx_clone = progress_tx.clone();

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

                // Aggiorna progress
                if let Some(tx) = &progress_tx_clone {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = tx.blocking_send(ProgressMessage::new(
                        MigrationStage::Decrypting,
                        current,
                        total,
                    ));
                }

                Ok(StoredRawPassword {
                    uuid: Uuid::new_v4(),
                    id: sp.id,
                    user_id: user_auth.id,
                    location: SecretString::new(location.into()),
                    password: SecretString::new(password.into()),
                    notes: notes.map(|n| SecretString::new(n.into())),
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

**Step 4: Verificare compilazione**

Run: `cargo check`
Expected: Errori nelle chiamate esistenti (manca il nuovo parametro)

**Step 5: Aggiornare le chiamate esistenti**

Cercare e aggiornare `get_stored_raw_passwords` (circa riga 214):

```rust
pub async fn get_stored_raw_passwords(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<StoredRawPassword>, DBError> {
    let stored_passwords: Vec<StoredPassword> =
        fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let stored_raw_passwords = decrypt_bulk_stored_data(
        fetch_user_auth_from_id(pool, user_id).await?,
        stored_passwords,
        None,  // Nessun progress tracking
    )
        .await?;
    Ok(stored_raw_passwords)
}
```

**Step 6: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita

**Step 7: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat: add progress tracking to decrypt_bulk_stored_data"
```

---

## Task 4: Aggiornare create_stored_data_records con progress tracking

**Files:**

- Modify: `src/backend/password_utils.rs`

**Step 1: Modificare la signature di create_stored_data_records**

Cercare la funzione `create_stored_data_records` (circa riga 155):

```rust
/// Crea record StoredPassword criptando location, password e notes in parallelo.
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
    progress_tx: Option<Arc<ProgressSender>>,  // NUOVO PARAMETRO
) -> Result<Vec<StoredPassword>, DBError> {
```

**Step 2: Aggiungere progress tracking nel par_iter**

Modificare il body della funzione:

```rust
/// Crea record StoredPassword criptando location, password e notes in parallelo.
pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<Vec<StoredPassword>, DBError> {
    if stored_raw_passwords.is_empty() {
        return Ok(Vec::new());
    }

    let cipher = Arc::new(cipher);
    let user_auth = Arc::new(user_auth);
    let total = stored_raw_passwords.len();
    let completed = Arc::new(AtomicUsize::new(0));
    let progress_tx_clone = progress_tx.clone();

    task::spawn_blocking(move || {
        stored_raw_passwords
            .into_par_iter()
            .map(|srp| {
                // Cripta location
                let (encrypted_location, location_nonce) =
                    encrypt_string(srp.location.expose_secret(), &cipher)?;

                // Cripta password
                let password_nonce = create_nonce();
                let encrypted_password =
                    create_password_with_cipher_sync(&srp.password, &password_nonce, &cipher)
                        .map_err(|_| {
                            DBError::new_cipher_encryption_error("Cipher error".to_string())
                        })?;

                // Cripta notes
                let notes_str = srp.notes.as_ref().map(|n| n.expose_secret().to_string());
                let (encrypted_notes, notes_nonce) =
                    encrypt_optional_string(notes_str.as_deref(), &cipher)?;

                // Calcola score
                let score_evaluation: PasswordScore = srp.score.unwrap_or_else(|| {
                    evaluate_password_strength(&srp.password, None)
                        .score
                        .unwrap_or(PasswordScore::new(0))
                });

                // Aggiorna progress
                if let Some(tx) = &progress_tx_clone {
                    let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = tx.blocking_send(ProgressMessage::new(
                        MigrationStage::Encrypting,
                        current,
                        total,
                    ));
                }

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

**Step 3: Aggiornare la chiamata in create_stored_data_pipeline_bulk**

Cercare `create_stored_data_pipeline_bulk` (circa riga 136):

```rust
/// Pipeline completa per salvare le passwords nel database in bulk/batch.
pub async fn create_stored_data_pipeline_bulk(
    pool: &SqlitePool,
    user_id: i64,
    stored_raw_passwords: Vec<StoredRawPassword>,
) -> Result<(), DBError> {
    // 1. Recupero credenziali e setup crittografico
    let user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&user_auth.password);
    let cipher = create_cipher(&salt, &user_auth)?;
    // 2. Creazione StoredPassword
    let stored_passwords =
        create_stored_data_records(cipher, user_auth, stored_raw_passwords, None).await?;  // None = no progress
    // 3. Salvataggio in batch
    upsert_stored_passwords_batch(&pool, stored_passwords).await?;

    Ok(())
}
```

**Step 4: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita

**Step 5: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat: add progress tracking to create_stored_data_records"
```

---

## Task 5: Creare stored_passwords_migration_pipeline_with_progress

**Files:**

- Modify: `src/backend/password_utils.rs`

**Step 1: Verificare gli import in password_utils.rs**

Gli import necessari dovrebbero già includere:

```rust
use crate::backend::db_backend::{
    fetch_all_stored_passwords_for_user, fetch_user_auth_from_id, remove_temp_old_password,
    upsert_stored_passwords_batch,
};
```

Nota: `restore_old_password` serve solo nel frontend, non nella pipeline.

**Step 2: Aggiungere la nuova funzione pipeline**

Aggiungere dopo `stored_passwords_migration_pipeline` (circa riga 312):

```rust
/// Pipeline di migrazione password con feedback di progresso.
/// Invia aggiornamenti tramite il canale mpsc fornito.
pub async fn stored_passwords_migration_pipeline_with_progress(
    pool: &SqlitePool,
    user_id: i64,
    old_password: String,
    progress_tx: Option<Arc<ProgressSender>>,
) -> Result<(), DBError> {
    // Invia stato iniziale
    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Decrypting, 0, 0)).await;
    }

    // 1. Fetch tutte le password salvate
    let data = fetch_all_stored_passwords_for_user(pool, user_id).await?;
    let total = data.len();

    // 2. Prepara UserAuth con vecchia password
    let old_password = SecretString::new(old_password.into());
    let user_auth = UserAuth {
        id: user_id,
        password: old_password.into(),
    };

    // 3. Decrypt con progress tracking
    let decrypted_data = decrypt_bulk_stored_data(user_auth, data, progress_tx.clone()).await?;

    // Invia cambio stage
    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Encrypting, 0, total)).await;
    }

    // 4. Recupera cipher con NUOVA password (dal DB aggiornato)
    let new_user_auth = fetch_user_auth_from_id(pool, user_id).await?;
    let salt = get_salt(&new_user_auth.password);
    let cipher = create_cipher(&salt, &new_user_auth)?;

    // 5. Encrypt con progress tracking
    let encrypted_data =
        create_stored_data_records(cipher, new_user_auth, decrypted_data, progress_tx.clone())
            .await?;

    // Invia finalizzazione
    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Finalizing, 0, 0)).await;
    }

    // 6. Salvataggio in batch
    upsert_stored_passwords_batch(pool, encrypted_data).await?;

    // 7. Rimuovi temp_old_password
    remove_temp_old_password(pool, user_id).await?;

    // Invia completamento
    if let Some(tx) = &progress_tx {
        let _ = tx.send(ProgressMessage::new(MigrationStage::Completed, 100, 100)).await;
    }

    Ok(())
}
```

**Step 3: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita

**Step 4: Commit**

```bash
git add src/backend/password_utils.rs
git commit -m "feat: add stored_passwords_migration_pipeline_with_progress"
```

---

## Task 6: Aggiornare ProgressMigrationChn - Rimuovere demo e aggiungere avvio automatico

**Files:**

- Modify: `src/components/features/progress_migration.rs`

**Step 1: Sostituire completamente il contenuto del file**

```rust
use crate::backend::db_backend::restore_old_password;
use crate::backend::migration_types::{MigrationStage, ProgressMessage};
use crate::backend::password_utils::stored_passwords_migration_pipeline_with_progress;
use crate::components::{MigrationData, show_toast_error, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI.
fn format_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing migration...".to_string(),
        MigrationStage::Decrypting => "Decrypting passwords...".to_string(),
        MigrationStage::Encrypting => "Encrypting with new password...".to_string(),
        MigrationStage::Finalizing => "Finalizing...".to_string(),
        MigrationStage::Completed => "Migration completed!".to_string(),
        MigrationStage::Failed => "Migration failed".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ProgressMigrationChn(
    /// Callback quando la migrazione è completata con successo
    on_completed: Signal<bool>,
    /// Callback quando la migrazione fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    let mut status_message = use_signal(|| String::new());
    let mut migration_started = use_signal(|| false);  // Flag per evitare doppi avvii
    let context = use_context::<Signal<MigrationData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia migrazione automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio della migrazione
        if migration_started() {
            return;
        }
        migration_started.set(true);

        let context = context.clone();
        let pool = pool.clone();
        let mut on_completed = on_completed.clone();
        let mut on_failed = on_failed.clone();
        let toast = toast.clone();

        let (tx, mut rx) = mpsc::channel(100);

        // Task per ricevere progress updates
        spawn(async move {
            while let Some(msg) = rx.recv().await {
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(format_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed.set(true);
                }
            }
        });

        // Task per eseguire la migrazione
        spawn(async move {
            let user_id = context.read().user_id;
            let old_password = context.read().old_password.clone();

            match (user_id, old_password) {
                (Some(uid), Some(pwd)) => {
                    let result = stored_passwords_migration_pipeline_with_progress(
                        &pool,
                        uid,
                        pwd,
                        Some(Arc::new(tx)),
                    )
                        .await;

                    if let Err(e) = result {
                        // Mostra toast errore
                        show_toast_error(
                            format!("Migration failed: {}", e),
                            toast,
                        );

                        // Rollback password
                        let _ = restore_old_password(&pool, uid).await;

                        // Imposta stato fallito
                        stage.set(MigrationStage::Failed);
                        on_failed.set(true);
                    }
                }
                _ => {
                    show_toast_error(
                        "Migration failed: missing user data".to_string(),
                        toast,
                    );
                    stage.set(MigrationStage::Failed);
                    on_failed.set(true);
                }
            }
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            // Messaggio stato
            p { class: "text-center font-medium text-base-content",
                "{status_message}"
            }

            // Progress bar DaisyUI
            progress {
                class: "progress progress-primary w-full",
                value: "{progress}",
                max: "100",
            }

            // Percentuale
            p { class: "text-center text-sm opacity-70",
                "{progress}%"
            }
        }
    }
}
```

**Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita

**Step 3: Commit**

```bash
git add src/components/features/progress_migration.rs
git commit -m "feat: replace demo with automatic migration in ProgressMigrationChn"
```

---

## Task 7: Aggiornare mod.rs per export migration_types

**Files:**

- Modify: `src/components/features/mod.rs`

**Step 1: Verificare che progress_migration sia esportato correttamente**

Verificare che `mod.rs` contenga:

```rust
pub mod progress_migration;
```

E che il componente `ProgressMigrationChn` sia re-exportato se necessario.

**Step 2: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita

**Step 3: Commit (se modifiche necessarie)**

```bash
git add src/components/features/mod.rs
git commit -m "chore: update mod.rs exports for progress_migration"
```

---

## Task 8: Test manuale e verifica finale

**Step 1: Check completo**

Run: `cargo check`
Expected: check riuscito senza warning critici

**Step 2: Verificare che i test esistenti passino**

Run: `cargo test`
Expected: Tutti i test passano

**Step 3: Commit finale**

```bash
git add -A
git commit -m "feat: complete password migration progress feedback implementation"
```

---

## Riepilogo File Modificati

| File                                            | Azione   |
|-------------------------------------------------|----------|
| `src/backend/migration_types.rs`                | Nuovo    |
| `src/backend/mod.rs`                            | Modifica |
| `src/backend/db_backend.rs`                     | Modifica |
| `src/backend/password_utils.rs`                 | Modifica |
| `src/components/features/progress_migration.rs` | Modifica |
| `src/components/features/mod.rs`                | Verifica |

---

## Note per Testing Manuale

1. Creare un utente con alcune password salvate
2. Andare in Account Settings
3. Cambiare la password
4. Verificare che:
    - Il dialog di migrazione appare
    - La progress bar mostra gli stadi corretti
    - I messaggi cambiano (Decrypting → Encrypting → Finalizing)
    - Al completamento, l'utente viene reindirizzato al login
5. Testare scenario errore:
    - Simulare un errore durante migrazione
    - Verificare toast di errore
    - Verificare rollback della password
