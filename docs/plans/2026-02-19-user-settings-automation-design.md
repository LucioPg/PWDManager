# User Settings Automation - Design Document

Data: 2026-02-19

## Overview

Automatizzare la creazione dei settings utente durante la registrazione, con focus sui settings per la generazione delle password suggerite.

## Obiettivo

Quando un nuovo utente si registra, il sistema deve creare automaticamente:
1. Un record in `user_settings` (tabella padre)
2. Un record in `passwords_generation_settings` (tabella figlia) con preset GOD di default

## Struttura Database

### Tabella `user_settings`

```sql
CREATE TABLE IF NOT EXISTS user_settings (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
)
```

### Tabella `passwords_generation_settings`

```sql
CREATE TABLE IF NOT EXISTS passwords_generation_settings (
    id INTEGER PRIMARY KEY,
    settings_id INTEGER NOT NULL,
    length INTEGER NOT NULL CHECK (0 <= length <= 100),
    symbols INTEGER NOT NULL,
    numbers BOOLEAN NOT NULL,
    uppercase BOOLEAN NOT NULL,
    lowercase BOOLEAN NOT NULL,
    excluded_symbols TEXT,
    FOREIGN KEY(settings_id) REFERENCES user_settings(id) ON DELETE CASCADE,
    CHECK (symbols <= length)
)
```

**Nota:** `symbols` è INTEGER perché rappresenta il NUMERO di simboli, non un flag booleano.

## Preset per Generazione Password

Valori calcolati oggettivamente da `strength_utils::evaluate_password_strength` per garantire che ogni preset rientri nella fascia di strength corretta.

### Formula di Calcolo Score

```
score = (length * 0.5)                    // max 20 punti
      + (variety_count * 15)              // max 60 punti (4 categorie)
      + bonus_speciali_multipli           // +5 se symbols >= 2
      + bonus_length                      // +5 se >12, +10 se >16
      + bonus_entropia                    // +5 se >=12 unici, +10 se >=16 unici
```

### Preset (valori minimi per fascia)

| Preset | length | symbols | numbers | uppercase | lowercase | Score |
|--------|--------|---------|---------|-----------|-----------|-------|
| Medium | 8 | 2 | true | true | true | 69 |
| Strong | 12 | 2 | true | true | true | 81 |
| Epic | 16 | 2 | true | true | true | 93 |
| God | 26 | 2 | true | true | true | 98 |

**Default per nuovi utenti:** GOD

**Nota:** Il punteggio base minimo è 65 (60 varietà + 5 speciali multipli), quindi non è possibile generare password WEAK con questo sistema.

## Architettura

### Approccio Scelto: Funzione con Preset Enum

Vantaggi:
- Type-safe, niente stringhe magiche
- Facile da testare e estendere
- Riutilizzabile quando l'utente cambia preset dalla pagina settings

### Strutture Dati

**File:** `src/backend/settings_types.rs` (nuovo)

```rust
use sqlx::FromRow;
use sqlx_template::SqliteTemplate;

/// Preset per la generazione password
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordPreset {
    Medium,
    Strong,
    Epic,
    God,
}

impl PasswordPreset {
    /// Restituisce la configurazione per questo preset.
    /// I valori sono calcolati per garantire lo score minimo della fascia.
    pub fn to_config(&self) -> PasswordGenConfig {
        match self {
            Self::Medium => PasswordGenConfig { length: 8, symbols: 2, numbers: true, uppercase: true, lowercase: true },
            Self::Strong => PasswordGenConfig { length: 12, symbols: 2, numbers: true, uppercase: true, lowercase: true },
            Self::Epic => PasswordGenConfig { length: 16, symbols: 2, numbers: true, uppercase: true, lowercase: true },
            Self::God => PasswordGenConfig { length: 26, symbols: 2, numbers: true, uppercase: true, lowercase: true },
        }
    }
}

/// Configurazione per la generazione password (in memoria)
#[derive(Debug, Clone, Copy)]
pub struct PasswordGenConfig {
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
}

/// Settings generali utente
#[derive(Debug, Clone, FromRow, SqliteTemplate)]
#[table("user_settings")]
#[tp_upsert(by = "id")]
pub struct UserSettings {
    pub id: Option<i64>,
    pub user_id: i64,
}

/// Settings per generazione password
#[derive(Debug, Clone, FromRow, SqliteTemplate)]
#[table("passwords_generation_settings")]
#[tp_upsert(by = "id")]
pub struct PasswordsGenSettings {
    pub id: Option<i64>,
    pub settings_id: i64,
    pub length: i32,
    pub symbols: i32,
    pub numbers: bool,
    pub uppercase: bool,
    pub lowercase: bool,
    pub excluded_symbols: Option<String>,
}
```

### Funzione di Creazione Settings

**File:** `src/backend/db_backend.rs`

```rust
/// Crea i settings di default per un nuovo utente.
///
/// Usa una transazione per garantire atomicità tra i due INSERT.
/// Usa `RETURNING id` per ottenere l'id generato (SQLite 3.35+).
pub async fn create_user_settings(
    pool: &SqlitePool,
    user_id: i64,
    preset: PasswordPreset,
) -> Result<(), DBError> {
    // Inizia transazione
    let mut tx = pool.begin().await
        .map_err(|e| DBError::new_general_error(format!("Failed to start transaction: {}", e)))?;

    // 1. Inserisci user_settings e ottieni l'id con RETURNING
    let settings_id: i64 = sqlx::query_scalar(
        "INSERT INTO user_settings (user_id) VALUES (?) RETURNING id"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| DBError::new_save_error(format!("Failed to insert user_settings: {}", e)))?;

    // 2. Inserisci passwords_generation_settings
    let config = preset.to_config();
    sqlx::query(
        "INSERT INTO passwords_generation_settings
         (settings_id, length, symbols, numbers, uppercase, lowercase, excluded_symbols)
         VALUES (?, ?, ?, ?, ?, ?, NULL)"
    )
    .bind(settings_id)
    .bind(config.length)
    .bind(config.symbols)
    .bind(config.numbers)
    .bind(config.uppercase)
    .bind(config.lowercase)
    .execute(&mut *tx)
    .await
    .map_err(|e| DBError::new_save_error(format!("Failed to insert gen_settings: {}", e)))?;

    // Commit transazione
    tx.commit().await
        .map_err(|e| DBError::new_general_error(format!("Failed to commit transaction: {}", e)))?;

    Ok(())
}
```

### Modifica a save_or_update_user

**File:** `src/backend/db_backend.rs`

La funzione deve restituire `i64` (user_id) invece di `()`:

```rust
pub async fn save_or_update_user(
    pool: &SqlitePool,
    id: Option<i64>,
    username: String,
    password: Option<SecretString>,
    avatar: Option<Vec<u8>>,
) -> Result<i64, DBError> {  // Cambiato da Result<(), DBError>
    match id {
        Some(user_id) => {
            // ... logica UPDATE esistente ...
            Ok(user_id)  // Restituisci l'id esistente
        }
        None => {
            // ... cripta password ...
            let user_id: i64 = sqlx::query_scalar(
                "INSERT INTO users (username, password, avatar) VALUES (?, ?, ?) RETURNING id"
            )
            .bind(&username)
            .bind(&hash_password)
            .bind(&avatar)
            .fetch_one(pool)
            .await
            .map_err(|e| DBError::new_save_error(format!("Insert failed: {}", e)))?;

            Ok(user_id)  // Restituisci il nuovo id
        }
    }
}
```

### Integrazione nel Flusso di Registrazione

**File:** `src/components/features/upsert_user.rs`

```rust
match save_or_update_user(&pool, user_id, u, password_to_save, a).await {
    Ok(saved_user_id) => {
        // Se è un nuovo utente, crea i settings di default
        if user_id.is_none() {
            if let Err(e) = create_user_settings(&pool, saved_user_id, PasswordPreset::God).await {
                tracing::warn!("Failed to create user settings for user {}: {}", saved_user_id, e);
                // Non blocchiamo la registrazione
            }
        }

        auth_state.logout();
        let message = if is_updating {
            "User Updated successfully!"
        } else {
            "User Registered successfully!"
        };
        schedule_toast_success(message.to_string(), toast);
        nav.push("/login");
    }
    Err(e) => error.set(Some(e.to_string())),
}
```

## Flusso Completo

```
Registrazione Utente
        │
        ▼
┌─────────────────────────┐
│ upsert_user.rs          │
│ on_submit()             │
└─────────────────────────┘
        │
        ▼
┌─────────────────────────┐
│ save_or_update_user()   │
│ → INSERT in users       │
│ → RETURNING id          │
│ → restituisce user_id   │
└─────────────────────────┘
        │
        ▼ (solo nuovi utenti)
┌─────────────────────────┐
│ create_user_settings()  │
│ → BEGIN TRANSACTION     │
│ → INSERT user_settings  │
│   RETURNING id          │
│ → INSERT gen_settings   │
│ → COMMIT                │
└─────────────────────────┘
        │
        ▼
┌─────────────────────────┐
│ Redirect a /login       │
└─────────────────────────┘
```

## Note Tecniche Importanti

### Perché non usare sqlx-template upsert_by_id?

Il metodo `upsert_by_id()` generato da sqlx-template restituisce `Result<(), Error>`, **non** la struct con l'id generato. Per ottenere l'id, usiamo `RETURNING id` con una query diretta.

### Perché usare una transazione?

Senza transazione, se il secondo INSERT fallisce, avremmo un record orfano in `user_settings` senza il corrispondente `passwords_generation_settings`. La transazione garantisce atomicità.

### Perché &SqlitePool invece di SqlitePool?

Passare per reference evita clonazioni non necessarie del pool. Il pool è internamente un `Arc`, quindi il clone è comunque economico, ma la reference è più idiomatica.

## Gestione Errori

- La creazione dei settings non blocca la registrazione
- Gli errori vengono loggati con `tracing::warn!`
- L'utente può configurare i settings in seguito dalla pagina dedicata

## File da Creare/Modificare

| File | Azione |
|------|--------|
| `src/backend/settings_types.rs` | **Creare** |
| `src/backend/mod.rs` | **Modificare** - export nuovi tipi |
| `src/backend/db_backend.rs` | **Modificare** - aggiungere `create_user_settings()`, modificare `save_or_update_user()` |
| `src/components/features/upsert_user.rs` | **Modificare** - chiamare `create_user_settings()` |

## Estensibilita

Il design permette future espansioni:
- Nuovi preset possono essere aggiunti all'enum `PasswordPreset`
- Nuove tabelle settings possono essere aggiunte seguendo lo stesso pattern
- Il campo `excluded_symbols` permette personalizzazione per siti con vincoli

## Decisioni Prese

1. **Funzione separata vs Trigger DB**: Scelto funzione separata per maggiore flessibilita e testabilita
2. **Preset GOD come default**: Massima sicurezza per un password manager
3. **Tabelle separate**: Per separare le responsabilita e permettere future espansioni
4. **Non bloccare registrazione**: Graceful degradation in caso di errori nella creazione settings
5. **Transazione per atomicità**: Garantisce consistenza dei dati
6. **RETURNING id invece di last_insert_rowid**: Più atomico e sicuro in contesti concorrenti
