## Generali

- [ ] rimuovere possibilità di aprire in devtools in release
- [ ] rimuovere menu contestuali in release
- [ ] rimuovere menu finestra (windows, help etc...)

## Registrazione

- [x] Verificare che il nome utente non sia già usato
- [x] Verificare i campi username e password siano pieni
- [x] Verificare che i campi password e re-type-password siano uguali

## Gestione utenti

- [x] Eliminare utente
- [x] Modificare password
- [x] Modificare nome utente
- [x] Preferenze utente

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
- [ ] modificare table row perché la password non sia immediatamente visibile
- [ ] modificare table row perché ci siano i pulsanti per copiare location e passwords

## Gestione password registrate

- [x] creare tabella password
- [ ] creare form di inserimento password
- [ ] creare form di modifica password e dati
- [ ] creare dialog di eliminazione password
- [ ] creare trigger async di ri-cryptaggio password nel caso un utente cambi la password. \[vedi sezione Stored
  Passwords migration]
- [x] aggiungere un "diversificatore" per chiave AES prelevato dalla stringa di creazione dell'utente per evitare il
  KEY-REUSE.

## To be Threaeded

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

- [ ] Creare trigger async di ri-cryptaggio password nel caso un utente cambi la password.
- [ ] Creare dialog di avviso di migrazione password.
- [ ] Creare dialog di migrazione password con progressbar e tasto pausa.

### Backend

#### Strength

- [ ] Sostituire funzione di calcolo strength con strength_utils. --- Parzialmente fatto
- [x] Creare meccanismo di spiegazione della valutazione della forza della password.

#### Migration

- [ ] Creare funzione di fetch per temp_old_password.
- [ ] Creare meccanismo di lavoro parallelo con rayon e tokio spawn_blocking. parzialmente fatto
- [ ] Creare funzione di aggregazione delle query per salvare in batch.
