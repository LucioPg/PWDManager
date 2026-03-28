## CRITICAL

- [x] non è possibile modificare il campo password dopo che è stato usato il pulsante suggest
- [x] le celle della tabella non sono responsive (c'è un max-width in pwd-secret-display)
- [x] i pulsanti di copia e show delle password e delle locations vengono nascossti quando si ridimensiona la finestra
- [x] le note non sono visibili perché restano in overflow dentro la tabella
- [x] in edit non è possibile salvare il form perché la password risulta vuota
- [x] la prima volta che si apre il form per l'edit il campo password è vuoto
- [x] created_at non salvato nel database
- [x] diceware non rileva la lingua di sistema per i nuovi utenti, usa inglese come default

## Generali

- [x] rimuovere possibilità di aprire in devtools in release
- [x] rimuovere menu contestuali in release
- [x] rimuovere menu finestra (windows, help etc...)
- [x] aggiungere password al database tramite sqlcipher con chiave nel keyring del sistema operativo (vedi
  `src/backend/db_key.rs` e `src/backend/db_backend.rs`)
- ~~la password db deve essere offuscata tramite il crate obfstr~~ (sostituito dal keyring del SO)

## Registrazione

- [x] Verificare che il nome utente non sia già usato
- [x] Verificare i campi username e password siano pieni
- [x] Verificare che i campi password e re-type-password siano uguali

## Gestione utenti

- [x] Eliminare utente
- [x] Modificare password
- [x] Modificare nome utente
- [x] Preferenze utente:
    - [x] Creazione tabella settings
    - [x] Creazione di un form per la modifica delle preferenze utente per le password
    - [x] Creazione trigger creazione di settings per ogni nuovo utente
- [ ] gestire l'auto logout:
    - [ ] aggiungere componente combobox in settings General per scegliere un set predefinito di tempistiche
    - [ ] aggiornare la tabella user settings (quella dove c'è anche il theme)
    - [ ] implementare l'event listener effettivo per il calcolo del timeout

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
- [x] aggiungere toggle per tema scuro in settings
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
- [x] migliorare le tabs dei settings
    - [ x] ~~creare tabs verticali per le preferenze per le password~~

## Gestione password registrate

- [x] creare tabella password
- [x] creare form di inserimento password
- [x] creare form di modifica password e dati
- [x] creare hook per salvataggio password nel database ( upsert )
- [x] creare dialog di eliminazione password
- [x] creare trigger async di ri-cryptaggio password nel caso un utente cambi la password. \[vedi sezione Stored
  Passwords migration]
- [x] aggiungere un "diversificatore" per chiave AES prelevato dalla stringa di creazione dell'utente per evitare il
  KEY-REUSE.
- [x] creazione funzione di suggerimento di password efficaci sulla base delle preferenze utente.
- [x] creazione di alternativa per la suggerimento di password, usando diceware:
    - [x] creare funzione di generazione di diceware [backend]
    - [x] creare menu alla pressione del tasto "suggest"
    - [x] creare pagina di settings per la modalità diceware
- [x] criptare anche location e note. Quindi usare lo stesso campo di visualizzazione per la password per la location,
  ma non per le note.
- [x] aggiungere due toggles per lo sblocco della visualizzazione di tutte le location e di tutte le passwords.
- [x] aggiungere pulsante per ordinare la dashboard per data crescente o decrescente. --- usata una combobox.
- [x] il pulsante per ordinare la dashboard deve basarsi sul name e non sulla location.
- [x] inserire un campo input per la ricerca, impostare filtro.
- [x] Assicurarsi che nella fase di import lo score venga ricalcolato.
- [x] Assicurarsi che il campo score del file di import non sia utilizzato.
- [x] Aggiungere un campo per il nome della StoredPassword alla tabella.
- [x] Aggiungere il campo username per la url/location della StoredPassword.
- [x] Rimuovere dalla table della dashboard url/location e password, usare solo name e score.
- [x] Rimuovere toggles per lo sblocco totale delle locations e passwords
- [x] Rimuovere tooltip alla pressione del pulsante "info" nel row della table della dashboard.
- [x] Creare dialog per visionare username, url/location, password e note, l'apertura deve essere governata dal pulsante
  precedentemente usato per le info.

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

- [x] Creare funzione di import/export in vari formati (csv, json, xml)

### Instruction

- [ ] creare README.md
- [ ] creare istruzioni

### Automatic Updates

- [x] creare sistema di aggiornamento automatico

### FIXES

- [ ] spostare import_data.rs e export_data.rs dal frontend al backend
- [ ] forzare spinner al caricamendo della dashboard o aumentare la velocità dell'animaizione (bug di rallentamento
  quando un utente fa il login con il dark mode attivo)
- [x] ~~la disinstallazione deve chiedere all'utente se vuole eliminare anche il database, cosa che non deve succedere
  se
  la disinstallazione avviene per un update.~~ Database viene eliminato automaticamente.