# Password Migration Progress Feedback - Design Document

**Data:** 2026-03-03
**Stato:** Approvato
**Approccio:** Canale mpsc con struct ProgressMessage

## Obiettivo

Fornire feedback visivo all'utente durante la migrazione delle password quando modifica la propria master password. Il processo deve mostrare lo stadio corrente (decriptazione/criptazione) e la percentuale di avanzamento.

## Requisiti

1. **UI**: Progress bar esistente con messaggio e percentuale per stadio corrente
2. **Stadi**: Decrypting → Encrypting → Finalizing → Completed/Failed
3. **Error handling**: Toast di errore + rollback da `temp_old_password`
4. **Avvio**: Automatico al mount del componente (nessun pulsante)

## Architettura

### Tipi di Dati

File: `src/backend/migration_types.rs` (nuovo)

```rust
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

#[derive(Clone, Debug)]
pub struct ProgressMessage {
    pub stage: MigrationStage,
    pub current: usize,
    pub total: usize,
}

impl ProgressMessage {
    pub fn percentage(&self) -> usize {
        if self.total == 0 { 0 }
        else { (self.current * 100) / self.total }
    }

    pub fn new(stage: MigrationStage, current: usize, total: usize) -> Self {
        Self { stage, current, total }
    }
}
```

### Modifiche Backend

#### 1. Signature funzioni bulk (password_utils.rs)

```rust
pub async fn decrypt_bulk_stored_data(
    user_auth: UserAuth,
    stored_passwords: Vec<StoredPassword>,
    progress_tx: Option<Arc<Sender<ProgressMessage>>>,  // NUOVO
) -> Result<Vec<StoredRawPassword>, DBError>

pub async fn create_stored_data_records(
    cipher: Aes256Gcm,
    user_auth: UserAuth,
    stored_raw_passwords: Vec<StoredRawPassword>,
    progress_tx: Option<Arc<Sender<ProgressMessage>>>,  // NUOVO
) -> Result<Vec<StoredPassword>, DBError>
```

#### 2. Pattern per par_iter con progress

```rust
let completed = Arc::new(AtomicUsize::new(0));
let progress_tx_clone = progress_tx.clone();
let total = stored_passwords.len();

stored_passwords
    .into_par_iter()
    .enumerate()
    .map(|(idx, item)| {
        // ... elaborazione ...

        // Aggiorna progress
        if let Some(tx) = &progress_tx_clone {
            let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = tx.blocking_send(ProgressMessage {
                stage: MigrationStage::Decrypting,
                current,
                total,
            });
        }

        // ... resto della logica ...
    })
```

#### 3. Nuova funzione pipeline con progress

```rust
pub async fn stored_passwords_migration_pipeline_with_progress(
    pool: &SqlitePool,
    user_id: i64,
    old_password: String,
    progress_tx: Option<Arc<Sender<ProgressMessage>>>,
) -> Result<(), DBError>
```

#### 4. Funzione rollback (db_backend.rs)

```rust
pub async fn restore_old_password(pool: &SqlitePool, user_id: i64) -> Result<(), DBError> {
    query(r#"
        UPDATE users
        SET password = temp_old_password,
            temp_old_password = NULL
        WHERE id = ?
    "#)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| DBError::new_update_error(format!("Failed to restore old password: {}", e)))?;

    Ok(())
}
```

### Modifiche Frontend

#### ProgressMigrationChn (progress_migration.rs)

- **Rimozione**: Pulsante demo
- **Avvio**: Automatico via `use_effect` al mount
- **Stati**: `stage`, `progress`, `status_message` come Signal

```rust
use_effect(move || {
    let (tx, mut rx) = mpsc::channel(100);

    // Task ricezione progress
    spawn(async move {
        while let Some(msg) = rx.recv().await {
            stage.set(msg.stage.clone());
            progress.set(msg.percentage());
            status_message.set(format_stage_message(&msg.stage));
        }
    });

    // Task esecuzione migrazione
    spawn(async move {
        let result = stored_passwords_migration_pipeline_with_progress(...).await;
        if let Err(e) = result {
            show_toast_error(...);
            restore_old_password(&pool, user_id).await;
            on_failed.set(true);
        }
    });
});
```

#### UI

```rust
rsx! {
    div { class: "flex flex-col gap-4",
        p { class: "text-center font-medium", "{status_message}" }
        progress { class: "progress progress-primary w-full", value: "{progress}", max: "100" }
        p { class: "text-center text-sm opacity-70", "{progress}%" }
    }
}
```

## Flusso Dati

```
[Mount ProgressMigrationChn]
        ↓
[use_effect spawn]
        ↓
[crea mpsc channel]
    ↓           ↓
[ricevitore]  [mittente → pipeline]
    ↓                    ↓
[aggiorna    [par_iter con progress_tx]
 Signal]              ↓
    ↓           [invia ProgressMessage]
[UI re-render]        ↓
                 [completato/errore]
                        ↓
              [on_completed o rollback + on_failed]
```

## Gestione Errori

1. Errore durante migrazione → catch nel spawn async
2. Mostra toast con messaggio errore
3. Chiama `restore_old_password(pool, user_id)`
4. Imposta `on_failed = true`
5. UI mostra stato `Failed`

## File da Modificare/Creare

| File | Azione | Descrizione |
|------|--------|-------------|
| `src/backend/migration_types.rs` | Nuovo | Tipi MigrationStage, ProgressMessage |
| `src/backend/mod.rs` | Modifica | Aggiungere mod migration_types |
| `src/backend/password_utils.rs` | Modifica | Signature funzioni bulk + nuova pipeline |
| `src/backend/db_backend.rs` | Modifica | Nuova funzione restore_old_password |
| `src/components/features/progress_migration.rs` | Modifica | Rimozione pulsante, avvio automatico |

## Retrocompatibilità

- Le funzioni bulk accettano `Option<Arc<Sender<...>>>`
- Le chiamate esistenti passano `None` per disabilitare progress tracking
- `stored_passwords_migration_pipeline` originale può essere deprecated o wrapper della nuova
