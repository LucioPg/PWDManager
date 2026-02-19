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

Basati su `docs/password_strength_levels.md`:

| Preset | length | symbols | numbers | uppercase | lowercase | Score Atteso |
|--------|--------|---------|---------|-----------|-----------|--------------|
| Medium | 10 | 1 | true | true | true | ~65 |
| Strong | 14 | 2 | true | true | true | ~75-82 |
| Epic | 18 | 3 | true | true | true | ~89+ |
| God | 24 | 5 | true | true | true | ~100 |

**Default per nuovi utenti:** GOD

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
    pub fn to_config(&self) -> PasswordGenConfig {
        match self {
            Self::Medium => PasswordGenConfig { length: 10, symbols: 1, numbers: true, uppercase: true, lowercase: true },
            Self::Strong => PasswordGenConfig { length: 14, symbols: 2, numbers: true, uppercase: true, lowercase: true },
            Self::Epic => PasswordGenConfig { length: 18, symbols: 3, numbers: true, uppercase: true, lowercase: true },
            Self::God => PasswordGenConfig { length: 24, symbols: 5, numbers: true, uppercase: true, lowercase: true },
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
pub async fn create_user_settings(
    pool: SqlitePool,
    user_id: i64,
    preset: PasswordPreset,
) -> Result<(), DBError> {
    // 1. Crea record in user_settings
    let user_settings = UserSettings {
        id: None,
        user_id,
    };
    let inserted = UserSettings::upsert_by_id(&user_settings, &pool)
        .await
        .map_err(|e| DBError::Save(Box::new(e)))?;

    let settings_id = inserted.id.ok_or_else(||
        DBError::Save("Failed to get user_settings id".into())
    )?;

    // 2. Crea record in passwords_generation_settings
    let config = preset.to_config();
    let gen_settings = PasswordsGenSettings {
        id: None,
        settings_id,
        length: config.length,
        symbols: config.symbols,
        numbers: config.numbers,
        uppercase: config.uppercase,
        lowercase: config.lowercase,
        excluded_symbols: None,
    };

    PasswordsGenSettings::upsert_by_id(&gen_settings, &pool)
        .await
        .map_err(|e| DBError::Save(Box::new(e)))?;

    Ok(())
}
```

### Integrazione nel Flusso di Registrazione

**File:** `src/components/features/upsert_user.rs`

```rust
match save_or_update_user(&pool, user_id, u, password_to_save, a).await {
    Ok(saved_user_id) => {
        // Se è un nuovo utente, crea i settings di default
        if user_id.is_none() {
            if let Err(e) = create_user_settings(pool.clone(), saved_user_id, PasswordPreset::God).await {
                tracing::error!("Failed to create user settings: {:?}", e);
                // Non blocchiamo la registrazione
            }
        }

        // ... resto del flusso esistente
    }
    // ... error handling esistente
}
```

**Nota:** `save_or_update_user` deve essere modificata per restituire `i64` (user_id) invece di `()`.

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
│ → restituisce user_id   │
└─────────────────────────┘
        │
        ▼ (solo nuovi utenti)
┌─────────────────────────┐
│ create_user_settings()  │
│ → INSERT in             │
│   user_settings         │
│ → INSERT in             │
│   passwords_gen_settings│
│   (preset GOD)          │
└─────────────────────────┘
        │
        ▼
┌─────────────────────────┐
│ Redirect a /login       │
└─────────────────────────┘
```

## Gestione Errori

- La creazione dei settings non blocca la registrazione
- Gli errori vengono loggati con `tracing::error!`
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
