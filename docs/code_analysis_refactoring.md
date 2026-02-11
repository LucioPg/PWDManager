# Code Analysis - Refactoring Opportunities

> Analisi del codebase per identificare aree di miglioramento e debito tecnico.

**Data**: 2025-02-11
**Scopo**: Documentare problemi individuati durante il debugging e lo sviluppo

---

## main.rs

### [x] Riga 48-64: `use_effect` con troppe responsabilità

**Problema**: L'effect gestisce sia i toast che il fetching della lista utenti.

**Rischi**:
- Ogni volta che `db_resource` cambia, viene rieseguito
- Se l'utente fa logout/login, la lista utenti viene rifetchata inutilmente
- Difficile testare le due funzionalità separatamente

**Soluzione suggerita**: Separare in due `use_effect` distinti o spostare la logica di fetching in `use_resource`

---

### [x] Riga 50-64: `spawn` dentro `use_effect` senza cleanup

**Problema**: Non c'è controllo per evitare spawn multipli.

**Rischi**:
- Se `db_resource` cambia rapidamente, potresti avere più task concorrenti
- Possibile race condition

**Soluzione applicata**: Usare un `use_signal` per memorizzare il `Task` di Dioxus e chiamare `cancel()` nel cleanup dell'effect. Nota: `spawn` di Dioxus restituisce un `Task` (non `JoinHandle` di Tokio), che ha il metodo `cancel()`.

---

### [ ] Manca gestione della chiusura del pool

**Problema**: Il `SqlitePool` viene clonato ma mai chiuso.

**Rischi**:
- Possibile resource leak
- Connessioni non rilasciate correttamente

**Soluzione suggerita**: Implementare cleanup on app exit

---

## ui_utils.rs

### [ ] Riga 22-24: Pattern `let-else` con return silenzioso

**Problema**: Non distingue tra: nessun file selezionato, JoinError, o altro errore.

```rust
let Ok(Some(path)) = file_result else {
    return;  // Perché siamo qui? Cancellazione? Errore?
};
```

**Rischi**:
- L'utente non sa se ha cancellato o c'è stato un errore
- Difficile debuggare

**Soluzione suggerita**: Gestire esplicitamente i casi:
```rust
match file_result {
    Ok(Some(path)) => { /* continua */ }
    Ok(None) => { // utente ha cancellato }
    Err(e) => { // JoinError - gestisci l'errore }
}
```

---

### [x] Righe 36-48: Duplice `spawn_blocking` per operazioni diverse

**Nota**: Questa non è una issue ma una **feature intenzionale**.

**Spiegazione**: I due `spawn_blocking` sono separati per un motivo preciso:

1. **Primo spawn_blocking** (FileDialog): Viene eseguito sempre quando l'utente clicca
2. **Secondo spawn_blocking** (scale_avatar): Viene eseguito **solo se** l'utente seleziona un file

Se l'utente annulla il dialog, il secondo spawn_blocking non viene mai creato, risparmiando risorse. Unire tutto in un unico spawn_blocking sarebbe meno efficiente perché il thread verrebbe creato anche quando l'utente annulla.

**Conclusione**: La separazione è corretta e ottimale per questo caso d'uso.

---

### [ ] Manca documentazione della funzione

**Problema**: Nessuna doc che spiega cosa fa, cosa restituisce, quando fallisce.

**Soluzione suggerita**: Aggiungere documentazione rustdoc con `///` ed esempi

---

## upsert_user.rs

### [ ] Riga 93: `use_context` dentro handler (anti-pattern)

**Problema**: `use_context` viene chiamato dentro `on_delete_user` invece che al top del componente.

```rust
let on_delete_user = move || {
    let pool = use_context::<SqlitePool>();  // ❌ Dovrebbe essere al top
    // ...
};
```

**Rischi**:
- Viene rieseguito ogni volta che l'handler viene chiamato
- `use_context` è un hook, dovrebbe essere chiamato solo al top level

**Soluzione suggerita**: Spostare `use_context` al top del componente e clonare il pool

---

### [ ] Righe 51-56: `use_memo` con side effect (`set`)

**Problema**: `use_memo` dovrebbe calcolare valori, non fare side effects.

```rust
use_memo(move || {
    if let Some(img) = new_avatar.read().clone() {
        avatar.set(...);  // ❌ Side effect in memo!
    }
});
```

**Rischi**:
- Comportamento non deterministico
- Difficile prevedere quando viene eseguito
- Possibile ciclo di aggiornamenti

**Soluzione suggerita**: Usare `use_effect` con dipendenze su `new_avatar`:
```rust
use_effect(move || {
    if let Some(img) = new_avatar.read().clone() {
        avatar.set(...);
    }
});
```

---

### [ ] Righe 85-90: Handler `pick_image` clona 3 signal senza debouncing

**Problema**: Ogni click crea un nuovo task senza controllare se uno è già in corso.

**Rischi**:
- Overhead se cliccato rapidamente più volte
- Possibile race condition

**Soluzione suggerita**: Implementare debouncing o disabilitare il bottone durante `is_loading`

---

### [ ] Righe 58-82: `use_effect` con due responsabilità distinte

**Problema**: Gestisce errori E user deletion nello stesso effect.

**Rischi**:
- Violazione Single Responsibility Principle
- Difficile testare separatamente
- Codice meno leggibile

**Soluzione suggerita**: Separare in due `use_effect` distinti

---

## Riassunto Priorità

| Priorità | File | Riga | Problema | Stato |
|----------|------|------|----------|-------|
| 🔴 Alta | `upsert_user.rs` | 93 | `use_context` dentro handler | [x]   |
| 🔴 Alta | `upsert_user.rs` | 51-56 | `use_memo` con side effect | [x]   |
| 🟡 Media | `main.rs` | 50-64 | `spawn` dentro `use_effect` senza cleanup | [x]   |
| 🟡 Media | `upsert_user.rs` | 85-90 | Nessun debouncing su pick_image | [x]   |
| 🟢 Bassa | `ui_utils.rs` | 22-24 | Gestione errore silenziosa | [x]   |
| 🟢 Bassa | `ui_utils.rs` | - | Manca documentazione | [x]   |
| 🟢 Bassa | `main.rs` | - | Manca cleanup pool | [x]   |
| 🟢 Bassa | `ui_utils.rs` | 36-48 | Duplice `spawn_blocking` (feature, non bug) | [x]   |

---

## Note Generali

- Le segnalazioni con checkbox `[ ]` sono ancora da affrontare
- Le segnalazioni con checkbox `[x]` sono state risolte
- Aggiornare questo file quando si affrontano i problemi

---

## Cronologia Modifiche

| Data | Modifica |
|------|---------|
| 2025-02-11 | Creazione iniziale del documento |
