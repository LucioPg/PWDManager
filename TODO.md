## CRITICAL

- [ ] non è possibile modificare il campo password dopo che è stato usato il pulsante suggest

## Generali

- [ ] rimuovere possibilità di aprire in devtools in release
- [ ] rimuovere menu contestuali in release
- [ ] rimuovere menu finestra (windows, help etc...)
- [ ] aggiungere password al database tramite sqlcipher:

```toml
[dependencies]
# Disabilita le feature di default e aggiungi il supporto a sqlcipher
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "macros"] }
# È spesso necessario linkare la libreria sqlcipher nativa
libsqlite3-sys = { version = "0.27", features = ["sqlcipher"] }
```

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::str::FromStr;

let options = SqliteConnectOptions::from_str("sqlite:mio_database.db") ?
.create_if_missing(true)
// Imposta la password qui
.pragma("key", "tua_password_segreta")
.journal_mode(SqliteJournalMode::Wal);

let pool = sqlx::SqlitePool::connect_with(options).await?;

```

- [ ] la password db deve essere offuscata tramite il crate obfstr:

```rust
let key = obfstr::obfstr!("password_molto_complessa");
let options = SqliteConnectOptions::new()
.pragma("key", key);

```

- [ ] oppure usare il keyring del sistema operativo per generare la password per ogni installazione:

```toml
[dependencies]
# Per gestire le chiavi nel portachiavi di sistema
keyring = "2.3"
# Per generare una chiave sicura e casuale
rand = "0.8"
# SQLx con supporto SQLCipher
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "macros"] }
libsqlite3-sys = { version = "0.27", features = ["sqlcipher"] }
```

```rust
use keyring::Entry;
use rand::{distributions::Alphanumeric, Rng};

fn get_or_create_db_key() -> Result<String, Box<dyn std::error::Error>> {
    let app_name = "mio_progetto_dioxus";
    let username = "default_user"; // Identificativo locale
    let entry = Entry::new(app_name, username)?;

    match entry.get_password() {
        Ok(existing_key) => Ok(existing_key),
        Err(_) => {
            // Se non esiste, genera una stringa casuale di 64 caratteri
            let new_key: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();

            // Salvala nel portachiavi di sistema (Windows Credential Manager / macOS Keychain)
            entry.set_password(&new_key)?;
            Ok(new_key)
        }
    }
}


// INTEGRAZIONE CON SQLX
pub async fn setup_database() -> Result<sqlx::SqlitePool, Box<dyn std::error::Error>> {
    let db_key = get_or_create_db_key()?;

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename("mio_database.db")
        .create_if_missing(true)
        // SQLCipher userà questa chiave per criptare/decriptare il file
        .pragma("key", &db_key)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = sqlx::SqlitePool::connect_with(options).await?;
    Ok(pool)
}

```

In questo caso è necessario provvedere alla creazione di una funzione di backup della chiave nel caso l'utente formatti
il pc

## Registrazione

- [x] Verificare che il nome utente non sia già usato
- [x] Verificare i campi username e password siano pieni
- [x] Verificare che i campi password e re-type-password siano uguali

## Gestione utenti

- [x] Eliminare utente
- [x] Modificare password
- [x] Modificare nome utente
- [ ] Preferenze utente:
    - [x] Creazione tabella settings
    - [x] Creazione di un form per la modifica delle preferenze utente per le password
    - [x] Creazione trigger creazione di settings per ogni nuovo utente

## UI e UX

- [x] stilizzare la navbar
- [x] stilizzare il form di registrazione
- [x] stilizzare il form di login
- [x] stilizzare la pagina di gestione utenti (settings)
- [x] stilizzare la pagina di dashboard
- [x] stilizzare la pagina di landing
- [x] stilizzare i componenti base (pulsanti etc...)
- [x] aggiungere uno spinner per lo scaling dell'avatar quando selezionato
- [x] aggiungere uno spinner all'avvio dell'applicazione
- [ ] aggiungere toggle per tema scuro in settings
- [x] aggiungere toast per errori e informazioni
- [x] modificare table row perché la password non sia immediatamente visibile
- [x] modificare table row perché ci siano i pulsanti per copiare location e passwords
- [x] decidere se offuscare e criptare anche location e note -- Fatto
- [x] lo sfondo scala male: allungando la finestra la scritta del logo viene tagliata, mentre il sotto testo rimane al
  centro sovrapponendosi alla scritta del logo.
- [x] dashboard aggiungere pulsante per cancellare tutte le password salvate -- nel caso ci fosse errore di decrypting
  irreversibile.
- [x] dashboard aggiungere pulsante di export in vari formati (csv, json, xml)  -- implementati manca backend
- [x] dashboard aggiungere pulsante di import da vari formati (csv, json, xml)   -- implementati manca backend
- [x] migliorare dashboard:
    - [x] le card stat sono troppo grandi e prominenti rispetto la tabella
    - [x] c'è un problema di overflow con tablerow
    - [x] verificare comportamento tabella quando sono presenti migliaia di password.
- ## Gestione password registrate

- [x] creare tabella password
- [x] creare form di inserimento password
- [x] creare form di modifica password e dati
- [x] creare hook per salvataggio password nel database ( upsert )
- [x] creare dialog di eliminazione password
- [x] creare trigger async di ri-cryptaggio password nel caso un utente cambi la password. \[vedi sezione Stored
  Passwords migration]
- [x] aggiungere un "diversificatore" per chiave AES prelevato dalla stringa di creazione dell'utente per evitare il
  KEY-REUSE.
- [x] creazione funzione di suggerimento di password efficaci sulla base delle preferenze utente. -- MANCA PARTE
  FRONTEND
- [x] criptare anche location e note. Quindi usare lo stesso campo di visualizzazione per la password per la location,
  ma non per le note.
- [ ] aggiungere due toggles per lo sblocco della visualizzazione di tutte le location e di tutte le passwords.
- [ ] aggungere pulsante per ordinare la dashboard per data crescente o decrescente.
- [ ] Assicurarsi che nella fase di import lo score venga ricalcolato.
- [ ] Assicurarsi che il campo score del file di import non sia utilizzato.

## To be Threaded

- [x] scaled_avatar
- [x] funzione di crypt e salvataggio password

## Stored Passwords migration

### Frontend

#### Strength

- [x] Estrarre componente salvataggio password da componente registrazione.
- [x] Agganciare funzione di calcolo strength a TUTTI i componenti che salvano password.
- [x] Usare spinner in attesa calcolo strength.
- [x] Mostrare risultato testuale e grafico di forza della password.
- [x] Usare pulsante info per spiegare il risultato della valutazione della forza della password.

#### Migration

- [x] Creare trigger async di ri-cryptaggio password nel caso un utente cambi la password.
- [x] Creare dialog di avviso di migrazione password.
- [x] Creare dialog di migrazione password con progressbar.

### Backend

#### Strength

- [x] Sostituire funzione di calcolo strength con strength_utils.
- [x] Creare meccanismo di spiegazione della valutazione della forza della password.

#### Migration

- [x] Creare funzione di fetch per temp_old_password (usata nei test, a runtime è passato come arg).
- [x] Creare meccanismo di lavoro parallelo con rayon e tokio spawn_blocking.
- [x] Creare funzione di aggregazione delle query per salvare in batch.

#### Import/Export

- [ ] Creare funzione di import/export in vari formati (csv, json, xml)