# Guida a sqlx-template

Questa guida spiega come usare `sqlx-template` nel progetto PWDManager per generare automaticamente funzioni CRUD (Create, Read, Update, Delete, Upsert).

## Cos'è sqlx-template?

`sqlx-template` è un crate che genera automaticamente funzioni per interrogare il database usando macro derive. Invece di scrivere SQL manualmente, definisci la struct e le funzioni vengono generate per te!

**Vantaggi:**
- ✅ Meno codice boilerplate
- ✅ Query type-safe verificate a compile-time
- ✅ Sintassi coerente per tutte le operazioni
- ✅ Supporta builder pattern per query dinamiche

## Setup

Nel tuo `Cargo.toml` hai già:

```toml
[dependencies]
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls"] }
sqlx-template = "0.2"
secrecy = "0.10"
futures = "0.3"      # ← Richiesto da SqlxTemplate per builder pattern
```

## Sintassi Base

### 1. Derivare la macro - SqliteTemplate vs SqlxTemplate

**Per operazioni CRUD base** (upsert, delete, select semplice):
```rust
use sqlx_template::SqliteTemplate;
use sqlx::FromRow;

#[derive(FromRow, Debug, SqliteTemplate)]
#[table("nome_tabella")]
#[tp_upsert(by = "id")]
pub struct MiaStruct {
    pub id: Option<i64>,
    pub campo1: String,
    pub campo2: i32,
}
```

**Per builder pattern e query avanzate**:
```rust
use sqlx_template::SqlxTemplate;  // ← Nota: SqlxTemplate, non SqliteTemplate!
use sqlx::FromRow;

#[derive(FromRow, Debug, SqlxTemplate)]
#[table("nome_tabella")]
#[db("sqlite")]           // ← Specifica il database
#[tp_upsert(by = "id")]
#[tp_select_builder]      // ← Abilita il builder pattern
pub struct MiaStruct {
    pub id: Option<i64>,
    pub campo1: String,
    pub campo2: i32,
}
```

| Macro | Quando usarla | Richiede futures? |
|-------|---------------|-------------------|
| `SqliteTemplate` | CRUD base semplice | No |
| `SqlxTemplate` | Builder pattern, query avanzate | **Sì** |

### 2. Attributi principali

| Attributo | Scopo | Esempio |
|-----------|---------|-----------|
| `#[table("...")]` | Nome della tabella DB | `#[table("passwords")]` |
| `#[tp_upsert(by = "...")]` | Genera upsert by campo | `#[tp_upsert(by = "id")]` |
| `#[tp_delete(by = "...")]` | Genera delete by campo | `#[tp_delete(by = "id")]` |
| `#[tp_select_all(by = "...")]` | Genera select by | `#[tp_select_all(by = "id")]` |
| `#[tp_update(by = "...")]` | Genera update by | `#[tp_update(by = "id")]` |

### 3. Attributi opzionali

| Attributo | Scopo |
|-----------|---------|
| `order = "campo desc"` | Ordinamento risultati |
| `fn_name = "nome_custom"` | Rinomina funzione generata |
| `returning = true` | Return record inserito (Postgres-only) |

## Esempio Pratico: StoredPassword

Questo è il codice attuale nel progetto (nota: usa `SqlxTemplate` per il builder pattern):

```rust
use sqlx_template::SqlxTemplate;  // ← Nota: SqlxTemplate, non SqliteTemplate!

#[derive(sqlx::FromRow, Debug, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]           // Specifica il database per ottimizzazioni
#[tp_upsert(by = "id")]
#[tp_select_builder]      // Abilita il builder per query dinamiche
pub struct StoredPassword {
    pub id: Option<i64>,            // INTEGER PRIMARY KEY,
    pub user_id: i64,               // INTEGER NOT NULL,
    pub location: String,           // TEXT NOT NULL,
    pub password: DbSecretVec,      // BLOB NOT NULL,
    pub notes: Option<String>,      // TEXT,
    pub strength: PasswordStrength, // TEXT NOT NULL,
    pub created_at: Option<String>, // TEXT,
    pub nonce: Vec<u8>,             // BLOB NOT NULL UNIQUE,
}
```

### Funzioni generate automaticamente

La macro `SqlxTemplate` genera:

```rust
impl StoredPassword {
    // UPSERT - Insert o Replace in base a id
    pub async fn upsert_by_id(
        &self,
        pool: &SqlitePool
    ) -> Result<(), sqlx::Error>;

    // BUILDER SELECT - Costruisce query dinamiche
    pub fn builder_select() -> StoredPasswordSelectBuilder;

    // E molte altre in base agli attributi tp_*...
}
```

### Builder generato da tp_select_builder

```rust
// Il builder ha metodi per ogni campo:
impl StoredPasswordSelectBuilder {
    // Filtri esatti
    pub fn user_id(&mut self, val: &i64) -> Result<&mut Self, sqlx::Error>;
    pub fn location(&mut self, val: &str) -> Result<&mut Self, sqlx::Error>;
    pub fn strength(&mut self, val: &PasswordStrength) -> Result<&mut Self, sqlx::Error>;

    // LIKE e varianti stringa
    pub fn location_like(&mut self, pattern: &str) -> Result<&mut Self, sqlx::Error>;
    pub fn location_start_with(&mut self, prefix: &str) -> Result<&mut Self, sqlx::Error>;
    pub fn location_end_with(&mut self, suffix: &str) -> Result<&mut Self, sqlx::Error>;

    // Comparazioni numeriche
    pub fn id_gt(&mut self, val: &i64) -> Result<&mut Self, sqlx::Error>;
    pub fn id_lt(&mut self, val: &i64) -> Result<&mut Self, sqlx::Error>;

    // Ordinamenti
    pub fn order_by_created_at_asc(&mut self) -> Result<&mut Self, sqlx::Error>;
    pub fn order_by_created_at_desc(&mut self) -> Result<&mut Self, sqlx::Error>;

    // Esecuzione
    pub async fn find_all(self, pool: &SqlitePool) -> Result<Vec<StoredPassword>, sqlx::Error>;
    pub async fn find_one(self, pool: &SqlitePool) -> Result<Option<StoredPassword>, sqlx::Error>;
    pub async fn find_page(self, (offset, limit, count): (usize, usize, bool), pool: &SqlitePool)
        -> Result<(Vec<StoredPassword>, Page, u64), sqlx::Error>;
}
```

## Upsert: Come funziona

L'**upsert** è l'operazione chiave per gestire sia INSERT che UPDATE in una sola chiamata.

### Comportamento

```rust
let password = StoredPassword { ... };

// CASO 1: id è None → INSERT nuovo record
password.id = None;
StoredPassword::upsert_by_id(&password, &pool).await?;
// SQL: INSERT INTO passwords (...) VALUES (...)

// CASO 2: id è Some(x) → REPLACE esistente
password.id = Some(123);
StoredPassword::upsert_by_id(&password, &pool).await?;
// SQL: INSERT OR REPLACE INTO passwords (...) VALUES (...)
```

### Perché `INSERT OR REPLACE`?

SQLite usa `INSERT OR REPLACE` per upsert:
- Se esiste una riga con lo stesso `id` → la **elimina e re-inserisce**
- Se non esiste → fa un normale **INSERT**

⚠️ **Attenzione:** `INSERT OR REPLACE` resetta TUTTI i campi, non solo quelli specificati.

## Builder Pattern con tp_select_builder

Il **Builder Pattern** è il modo più flessibile per eseguire query SELECT con sqlx-template. A differenza di `tp_select_all` che genera un metodo fisso, il builder ti permette di comporre query dinamiche passo dopo passo.

### Attenzione: SqlxTemplate vs SqliteTemplate

⚠️ **IMPORTANTE:** Per usare `#[tp_select_builder]`, devi usare `SqlxTemplate` (non `SqliteTemplate`) e aggiungere `futures` alle dipendenze:

```toml
[dependencies]
sqlx-template = "0.2"
futures = "0.3"  # Richiesto da SqlxTemplate
```

### Abilitare il Builder

Aggiungi `#[tp_select_builder]` alla tua struct:

```rust
use sqlx_template::SqlxTemplate;  // Nota: SqlxTemplate, non SqliteTemplate!

#[derive(sqlx::FromRow, Debug, SqlxTemplate)]
#[table("passwords")]
#[db("sqlite")]           // Specifica il database
#[tp_upsert(by = "id")]
#[tp_select_builder]      // ← Abilita il builder pattern
pub struct StoredPassword {
    pub id: Option<i64>,
    pub user_id: i64,
    pub location: String,
    pub password: DbSecretVec,
    pub notes: Option<String>,
    pub strength: PasswordStrength,
    pub created_at: Option<String>,
    pub nonce: Vec<u8>,
}
```

### Metodi Generati Automaticamente

Il builder genera automaticamente metodi per **ogni campo** della struct:

| Campo della struct | Metodo generato | Esempio |
|--------------------|-----------------|---------|
| `user_id: i64` | `.user_id(&i64)` | `.user_id(&123)` |
| `location: String` | `.location("text")` | `.location("github.com")` |
| `strength: PasswordStrength` | `.strength(&enum)` | `.strength(&PasswordStrength::STRONG)` |
| `created_at: String` | `.created_at("date")` | `.created_at("2024-01-15")` |

Oltre ai metodi per campo, hai anche:

| Metodo | Descrizione |
|--------|-------------|
| `.field_like("pattern")` | LIKE per stringhe |
| `.field_start_with("prefix")` | STARTS WITH |
| `.field_end_with("suffix")` | ENDS WITH |
| `.field_gt(&val)` | Greater than (numeri) |
| `.field_gte(&val)` | Greater or equal |
| `.field_lt(&val)` | Less than |
| `.field_lte(&val)` | Less or equal |
| `.order_by_field_asc()` | Ordinamento crescente |
| `.order_by_field_desc()` | Ordinamento decrescente |

### Esempio Pratico: Fetch by user_id

Questo è il codice attuale in `db_backend.rs`:

```rust
#[instrument(skip(pool))]
pub async fn get_all_passwords_for_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<StoredPassword>, DBError> {
    debug!("Fetching all passwords for user_id: {}", user_id);

    // Builder: 1. Crea SELECT  2. Filtra per user_id  3. Ordina per created_at DESC
    let builder = StoredPassword::builder_select()
        .user_id(&user_id)
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?
        .order_by_created_at_desc()
        .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;

    builder
        .find_all(pool)
        .await
        .map_err(|e| DBError::new_list_error(format!("Failed to fetch passwords: {}", e)))
}
```

**SQL generato:**
```sql
SELECT * FROM passwords WHERE user_id = ? ORDER BY created_at DESC
```

### Altri Esempi di Query

#### Query con condizioni multiple

```rust
// Tutte le password forti dell'utente 123, ordinate per location
let results = StoredPassword::builder_select()
    .user_id(&123)?
    .strength(&PasswordStrength::STRONG)?
    .order_by_location_asc()?
    .find_all(pool)
    .await?;
```

#### Query con LIKE

```rust
// Tutte le password con location che contiene "github"
let results = StoredPassword::builder_select()
    .location_like("%github%")?
    .find_all(pool)
    .await?;
```

#### Query con comparazioni numeriche

```rust
// Tutte le password con ID > 100
let results = StoredPassword::builder_select()
    .id_gt(&100)?
    .find_all(pool)
    .await?;
```

#### Query con paginazione

```rust
// Prima pagina (5 risultati), con conteggio totale
let (results, total, page) = StoredPassword::builder_select()
    .user_id(&123)?
    .order_by_created_at_desc()?
    .find_page((0, 5, true), pool)  // (offset, limit, count)
    .await?;
```

### Metodi di Esecuzione

| Metodo | Return | Descrizione |
|--------|--------|-------------|
| `.find_all(pool)` | `Vec<T>` | Tutti i risultati |
| `.find_one(pool)` | `Option<T>` | Un solo risultato |
| `.find_page((offset, limit, count), pool)` | `(Vec<T>, Page, u64)` | Paginazione |
| `.stream(pool)` | `Stream` | Stream per grandi dataset |
| `.count(pool)` | `u64` | Solo conteggio |

### Gestione Errori

Ogni metodo del builder restituisce `Result<_, sqlx::Error>`. Per convertire in `DBError`:

```rust
let builder = StoredPassword::builder_select()
    .user_id(&user_id)
    .map_err(|e| DBError::new_list_error(format!("Builder error: {}", e)))?;  // ← unwrap con conversione
```

### Perché usare il Builder invece di tp_select_all?

| Caratteristica | `tp_select_all` | `tp_select_builder` |
|----------------|-----------------|---------------------|
| Query fisse | ✅ Perfetto | ❌ Overkill |
| Query dinamiche | ❌ Impossibile | ✅ Ideale |
| Ordinamento flessibile | ❌ Solo specificato nell'attributo | ✅ A runtime |
| Filtri opzionali | ❌ Tutti obbligatori | ✅ Puoi ometterli |
| Combinazioni complesse | ❌ Non supportato | ✅ Più condizioni |
| Semplicità | ✅ Una chiamata | ⚠️ Più verboso |

**Quando usare l'uno o l'altro:**
- Query semplice e fissa → `tp_select_all`
- Query con filtri dinamici, ordinamento variabile, o combinazioni complesse → `tp_select_builder`

### Custom Conditions

Per condizioni SQL personalizzate che non corrispondono a un campo:

```rust
#[tp_select_builder(
    with_email_domain = "email LIKE :domain$String",
    with_score_range = "score BETWEEN :min$i32 AND :max$i32"
)]
pub struct User { ... }
```

Uso:
```rust
let results = User::builder_select()
    .with_email_domain("@gmail.com")?
    .with_score_range(10, 100)?
    .find_all(pool)
    .await?;
```

## Confronto Vecchia → Nuova Sintassi

⚠️ **IMPORTANTE:** La sintassi vecchia è deprecata!

| Vecchia (Deprecata) | Nuova (Corretta) |
|---------------------|-------------------|
| `#[sqlx(table_name = "...")]` | `#[table("...")]` |
| `#[sqlx(upsert_by = "...")]` | `#[tp_upsert(by = "...")]` |
| Non supportata | `#[tp_delete(by = "...")]` |

## Metodi Statici vs Metodi di Istanza

**SBAGLIATO:**
```rust
let password = StoredPassword { ... };
password.upsert(&pool).await?;  // ❌ Non esiste!
```

**CORRETTO:**
```rust
let password = StoredPassword { ... };
StoredPassword::upsert_by_id(&password, &pool).await?;  // ✅ Metodo statico
```

Tutti i metodi generati da `sqlx-template` sono **statici** e si chiamano con `NomeStruct::metodo()`.

## Esempio Completo in db_backend.rs

```rust
pub async fn save_or_update_stored_password(
    pool: &SqlitePool,
    stored_password: StoredPassword,
) -> Result<(), DBError> {
    debug!("Attempting to save/update user password");

    // Validazione
    if stored_password.password.expose_secret().is_empty()
        || stored_password.location.trim().is_empty()
    {
        return Err(DBError::new_password_save_error(
            "Password and location cannot be empty".into()
        ));
    }

    // Upsert gestisce entrambi i casi:
    // - id None → INSERT
    // - id Some(id) → UPDATE (via INSERT OR REPLACE)
    StoredPassword::upsert_by_id(&stored_password, pool)
        .await
        .map_err(|e| DBError::new_password_save_error(format!("Upsert failed: {}", e)))?;

    Ok(())
}
```

## Troubleshooting

### Errore: "no method named `upsert`"

**Problema:** Stai chiamando un metodo di istanza che non esiste.

**Soluzione:** Usa il metodo statico generato:
```rust
// ❌ Errato
stored_password.upsert(&pool)

// ✅ Corretto
StoredPassword::upsert_by_id(&stored_password, &pool)
```

### Errore: "cannot find attribute `db`"

**Problema:** Stai usando una sintassi non supportata dalla versione del crate.

**Soluzione:** Rimuovi l'attributo `#[db("sqlite")]` dalla struct.

### Errore: "trait bound &&Pool is not satisfied"

**Problema:** Stai passando `&pool` quando il metodo si aspetta `pool`.

**Soluzione:** Rimuovi il riferimento:
```rust
// ❌
StoredPassword::upsert_by_id(&password, &pool)

// ✅
StoredPassword::upsert_by_id(&password, pool)
```

### Tipo custom non mappato

Se hai tipi custom come `DbSecretVec`, devi implementare i trait SQLx:

```rust
impl Type<Sqlite> for DbSecretVec { ... }
impl Encode<'q, Sqlite> for DbSecretVec { ... }
impl Decode<'r, Sqlite> for DbSecretVec { ... }
```

## Riferimenti

- [sqlx-template su GitHub](https://github.com/hn63wospuvy/sqlx-template)
- [sqlx-template su crates.io](https://crates.io/crates/sqlx-template)
- [SQLx Documentation](https://docs.rs/sqlx/)

## Checklist Rapida

- [ ] Struct deriva `SqliteTemplate`
- [ ] Attributo `#[table("nome_tabella")]` presente
- [ ] Attributo `#[tp_upsert(by = "campo")]` per upsert
- [ ] Campi della struct corrispondono alle colonne DB
- [ ] Tipi custom implementano `Type`, `Encode`, `Decode`
- [ ] Metodo chiamato come `NomeStruct::metodo_statico()`
- [ ] Pool passato senza riferimento aggiuntivo
