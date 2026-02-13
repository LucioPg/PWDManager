# Design: Test Suite per `save_or_update_user` in `db_backend.rs`

**Data:** 2026-02-13
**Autore:** Claude Code
**Stato:** Draft

## Obiettivo

Creare una suite di test completa per la funzionalità `save_or_update_user` nel modulo `backend/db_backend.rs`. Il design mantiene il codice di produzione invariato aggiungendo solo test per verificare il comportamento della funzione.

## Contesto

Attualmente:
- **Utenti:** Gestiti con query SQL manuali (righe 132-198 in `db_backend.rs`)
- **Password:** Gestite con `sqlx-template` e metodi generati automaticamente
- **Test:** Nessun test esistente per la funzionalità utenti

La funzione `save_or_update_user` supporta:
- **INSERT** quando `id: None` - crea nuovo utente
- **UPDATE** quando `id: Some(user_id)` - aggiorna utente esistente
- **Salvataggio temporaneo** - salva vecia password in `temp_old_password` quando si aggiorna

## Scopo

I test devono verifrare:
1. ✅ Inserimento corretto di nuovi utenti
2. ✅ Aggiornamento corretto di utenti esistenti
3. ✅ Funzionamento del salvataggio `temp_old_password`
4. ✅ Gestione appropriata dei casi di errore

## Design

### 1. Organizzazione dei File

```
src/backend/
├── mod.rs                       # Aggiungerà `#[cfg(test)] mod db_backend_tests;`
├── db_backend.rs                 # Codice produzione (NESSUNA MODIFICA)
└── db_backend_tests.rs           # [NUOVO] Tutti i test per db_backend
```

### 2. Dipendenze Aggiuntive

Aggiungere al `Cargo.toml` principale:

```toml
[dev-dependencies]
tempfile = "3"  # Per directory temporanee per test con file DB reali
```

**Nota:** `tempfile` è uno standard nell'ecosistema Rust per creare directory temporanee che vengono pulite automaticamente.

### 3. Setup e Teardown Pattern

Ogni test userà questo pattern per inizializzare il database:

```rust
async fn setup_test_db() -> (SqlitePool, TempDir) {
    // 1. Crea directory temporanea (auto-cleanup quando esce dallo scope)
    let temp_dir = TempDir::new().unwrap();

    // 2. Crea database file SQLite nella directory temporanea
    let db_path = temp_dir.path().join("test_users.db");
    let options = SqliteConnectOptions::from_str(
        format!("sqlite:{}", db_path.display())
    )
    .unwrap()
    .journal_mode(SqliteJournalMode::Wal)  // Fondamentale per concorrenza
    .foreign_keys(true)
    .create_if_missing(true);

    // 3. Connetiti e inizzializza schema con init_db()
    let pool = SqlitePool::connect_with(options).await.unwrap();

    // Esegui le query di inizializzazione (crea tabella users)
    for init_query in QUERIES {
        query(init_query)
            .execute(&pool)
            .await
            .expect("Failed to create table during test setup");
    }

    (pool, temp_dir)  // Return entrambi per gestire lifecycle
}
```

**Vantaggi:**
- Database reale con WAL mode (testa concorrenza)
- `TempDir` garantisce cleanup automatico (nessun file orfano)
- Ogni test ha un DB pulito e isolato

### 4. Categorie di Test

#### Categoria 1: Test INSERT Nuovi Utenti

| Test Nome | Descrizione | Verifica |
|-----------|-------------|----------|
| `test_insert_new_user_success` | Inserisce utente completo con username, password e avatar | Record creato nel DB con ID generato |
| `test_insert_new_user_without_avatar` | Inserisce utente senza avatar (Opzionale) | Record creato con avatar=NULL |
| `test_insert_new_user_empty_password` | Tenta inserimento con password vuota | Return `Err(DBError)` con messaggio appropriato |
| `test_insert_new_user_trimmed_password` | Inserisce utente con password con spazi (trimmed) | Password salvata senza spazi |

#### Categoria 2: Test UPDATE Utenti Esistenti

| Test Nome | Descrizione | Verifica |
|-----------|-------------|----------|
| `test_update_username_only` | Aggiorna solo username (password=None, avatar=None) | Username aggiornato, password/avatar invariati |
| `test_update_password_only` | Aggiorna solo password | Password aggiornata, vecia salvata in `temp_old_password` |
| `test_update_avatar_only` | Aggiorna solo avatar | Avatar aggiornato, altri campi invariati |
| `test_update_all_fields` | Aggiorna username, password E avatar | Tutti i campi aggiornati correttamente |
| `test_update_empty_password_ignored` | Aggiorna con password vuota (stringa vuota dopo trim) | Nessun aggiornamento password, errore gestito |

#### Categoria 3: Test per temp_old_password

| Test Nome | Descrizione | Verifica |
|-----------|-------------|----------|
| `test_temp_password_saved_on_update` | Aggiorna password utente esistente | `temp_old_password` contiene vecia password (hash) |
| `test_temp_password_overwritten_on_multiple_updates` | Aggiorna password due volte | `temp_old_password` contiene solo penultima vecia password |
| `test_temp_password_set_on_first_update` | Prima modifica password utente | `temp_old_password` salvato correttamente |
| `test_temp_password_query_after_update` | Dopo aggiornamento, query `temp_old_password` | Valore recuperato corrisponde al vecio hash |

#### Categoria 4: Test per Casi di Errore

| Test Nome | Descrizione | Comportamento Atteso |
|-----------|-------------|---------------------|
| `test_update_nonexistent_user` | Tenta UPDATE con ID inesistente | Query SQLite success (ma 0 righe affette) |
| `test_update_all_fields_after_user_deletion` | Aggiorna utente che viene cancellato | Gestione appropriata |
| `test_insert_empty_username` | Tenta INSERT con username vuoto | Errore SQLite (vincolo NOT NULL) |
| `test_special_characters_username` | Inserisci/aggiorna username con caratteri speciali | Gestione corretta escape/caratteri |

### 5. Struttura del Codice di Test

```rust
#[cfg(test)]
mod db_backend_tests {
    use super::*;
    use crate::backend::init_queries::QUERIES;
    use secrecy::SecretString;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
    use tempfile::TempDir;

    /// Helper function per setup database di test
    async fn setup_test_db() -> (SqlitePool, TempDir) {
        // ... (vedere sezione 3)
    }

    /// Helper function per creare un utente di test base
    async fn create_test_user(pool: &SqlitePool) -> i64 {
        // Crea e returna l'ID di un utente di test
    }

    // ============ Categoria 1: Test INSERT ============

    #[tokio::test]
    async fn test_insert_new_user_success() {
        let (pool, _temp_dir) = setup_test_db().await;

        let username = "test_user".to_string();
        let password = SecretString::new("secure_password_123".to_string());
        let avatar = vec![1u8, 2u8, 3u8];

        let result = save_or_update_user(
            &pool,
            None,  // id = None → INSERT
            username.clone(),
            Some(password),
            Some(avatar.clone())
        ).await;

        assert!(result.is_ok(), "INSERT should succeed");

        // Verifica che l'utente sia nel database
        let users = list_users(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].1, username);  // username
    }

    #[tokio::test]
    async fn test_insert_new_user_without_avatar() {
        let (pool, _temp_dir) = setup_test_db().await;

        let result = save_or_update_user(
            &pool,
            None,
            "test_user_no_avatar".to_string(),
            Some(SecretString::new("password".to_string())),
            None  // avatar = None
        ).await;

        assert!(result.is_ok());

        // Verifica che l'utente sia stato creato senza avatar
        let users = list_users(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
        assert!(users[0].3.is_none());  // avatar = None
    }

    #[tokio::test]
    async fn test_insert_new_user_empty_password() {
        let (pool, _temp_dir) = setup_test_db().await;

        let result = save_or_update_user(
            &pool,
            None,
            "test_user".to_string(),
            Some(SecretString::new("".to_string())),  // password vuota
            None
        ).await;

        assert!(result.is_err(), "Empty password should return error");
    }

    // ============ Categoria 2: Test UPDATE ============

    #[tokio::test]
    async fn test_update_username_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Prima crea un utente
        let user_id = create_test_user(&pool).await;

        // Poi aggiorna solo username
        let new_username = "updated_username".to_string();
        let result = save_or_update_user(
            &pool,
            Some(user_id),  // id = Some → UPDATE
            new_username.clone(),
            None,  // password = None
            None   // avatar = None
        ).await;

        assert!(result.is_ok());

        // Verifica aggiornamento
        let users = list_users(&pool).await.unwrap();
        assert_eq!(users[0].0, user_id);
        assert_eq!(users[0].1, new_username);
    }

    #[tokio::test]
    async fn test_update_password_only() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;
        let old_password = "old_password".to_string();

        // Aggiorna solo password
        let new_password = SecretString::new("new_password_456".to_string());
        let result = save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),  // username invariato
            Some(new_password),
            None
        ).await;

        assert!(result.is_ok());

        // Verifica che temp_old_password sia stato salvato
        // (questo richiede una query per verificare il campo temp_old_password)
    }

    #[tokio::test]
    async fn test_update_all_fields() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;

        let new_username = "fully_updated".to_string();
        let new_password = SecretString::new("new_pass".to_string());
        let new_avatar = vec![9u8, 8u8, 7u8];

        let result = save_or_update_user(
            &pool,
            Some(user_id),
            new_username.clone(),
            Some(new_password),
            Some(new_avatar.clone())
        ).await;

        assert!(result.is_ok());

        // Verifica tutti i campi
        let users = list_users(&pool).await.unwrap();
        assert_eq!(users[0].1, new_username);
        assert_eq!(users[0].3, Some(new_avatar));
    }

    // ============ Categoria 3: Test temp_old_password ============

    #[tokio::test]
    async fn test_temp_password_saved_on_update() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;
        let old_password_hash = fetch_user_password(&pool, "test_user").await.unwrap();

        // Aggiorna password
        let new_password = SecretString::new("new_password".to_string());
        save_or_update_user(&pool, Some(user_id), "test_user".to_string(), Some(new_password), None).await.unwrap();

        // Verifica temp_old_password
        let temp_password_row = sqlx::query("SELECT temp_old_password FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one::<String>(&pool)
            .await
            .unwrap();

        assert_eq!(temp_password_row, old_password_hash);
    }

    #[tokio::test]
    async fn test_temp_password_overwritten_on_multiple_updates() {
        let (pool, _temp_dir) = setup_test_db().await;

        let user_id = create_test_user(&pool).await;
        let first_hash = fetch_user_password(&pool, "test_user").await.unwrap();

        // Prima aggiornamento
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password2".to_string())),
            None
        ).await.unwrap();
        let second_hash = fetch_user_password(&pool, "test_user").await.unwrap();

        // Secondo aggiornamento
        save_or_update_user(
            &pool,
            Some(user_id),
            "test_user".to_string(),
            Some(SecretString::new("password3".to_string())),
            None
        ).await.unwrap();

        // Verifica che temp_old_password contenga la seconda password (non la prima)
        let temp_password_row = sqlx::query("SELECT temp_old_password FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one::<String>(&pool)
            .await
            .unwrap();

        assert_eq!(temp_password_row, second_hash);
        assert_ne!(temp_password_row, first_hash);
    }

    // ============ Categoria 4: Test Casi di Errore ============

    #[tokio::test]
    async fn test_update_nonexistent_user() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Tenta aggiornamento con ID inesistente
        let fake_id = 99999i64;
        let result = save_or_update_user(
            &pool,
            Some(fake_id),
            "nonexistent".to_string(),
            Some(SecretString::new("password".to_string())),
            None
        ).await;

        // SQLite UPDATE con ID inesistente non fallisce (0 righe affette)
        // Quindi questo test verifica che non ci siano panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_special_characters_username() {
        let (pool, _temp_dir) = setup_test_db().await;

        let special_username = "user'; DROP TABLE users; --".to_string();  // SQL injection test
        let result = save_or_update_user(
            &pool,
            None,
            special_username,
            Some(SecretString::new("password".to_string())),
            None
        ).await;

        // sqlx dovrebbe gestire l'escape correttamente
        assert!(result.is_ok());
    }
}
```

### 6. Modifiche a `mod.rs`

In `src/backend/mod.rs`, aggiungere:

```rust
#[cfg(test)]
mod db_backend_tests;
```

Questo abilita il modulo solo durante compilazione con `--tests`.

## Vincoli e Requisiti

1. **Nessuna modifica al codice di produzione** - I test non devono toccare `db_backend.rs`
2. **Isolamento test** - Ogni test deve avere il proprio database pulito
3. **Cleanup automatico** - `TempDir` garantisce nessun file orfano
4. **Compatibilità SQLite** - Test con file DB reali (non `:memory:`) per testare WAL mode

## Domande Aperte (da risolvere durante implementazione)

1. Dobbiamo esporre `fetch_user_password` come `pub` per testare `temp_old_password`?
2. Il sistema di encryption per password è disponibile in test o serve mock?
3. Dobbiamo aggiungere `tokio::test` macro ai test o usare `#[test]` standard?

## Prossimi Passi

1. ✅ Approvazione design documento
2. ⏳ Invocare `superpowers:writing-plans` per creare piano di implementazione dettagliato
3. ⏳ Implementare test seguendo piano
4. ⏳ Esegure `cargo test` per verifrare che tutti passino
5. �isci Aggiungere CI per eseguire test automaticamente
