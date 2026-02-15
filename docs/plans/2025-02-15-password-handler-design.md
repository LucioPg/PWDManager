# PasswordHandler & StrengthAnalyzer - Design Document

**Date:** 2025-02-15
**Status:** Approved
**Author:** Brainstorming session with user

## Overview

Questo design definisce l'implementazione di due componenti riutilizzabili per la gestione e la valutazione della robustezza delle password in PWDManager:

- **PasswordHandler**: Componente container che gestisce input password/retype-password, debounce, valutazione e cancellazione
- **StrengthAnalyzer**: Componente standalone che visualizza lo stato di valutazione con colori e reasons

Questi componenti saranno utilizzati sia per la MasterPassword (registrazione/account settings) che per le future StoredPassword.

## Goals

1. Creare un componente riutilizzabile per l'input e la valutazione delle password
2. Integrare strength_utils per valutare la robustezza delle password
3. Mostrare feedback visivo immediato all'utente (testo colorato + reasons)
4. Supportare cancellazione granulare della valutazione durante le fasi intermedie
5. Gestire il debounce per evitare valutazioni multiple durante la digitazione
6. Estendere PasswordStrength per includere lo stato NotEvaluated e reasons

## Non-Goals

- Implementazione diretta delle StoredPassword (task futuro)
- Modifica dell'algoritmo di valutazione oltre il refactoring per sezioni
- Creazione di nuove regole di validazione password

## Architecture

### Component Structure

```
src/components/globals/password_handler/
├── mod.rs              # Esporta entrambi i componenti
├── component.rs        # PasswordHandler (container)
└── strength_analyzer.rs # StrengthAnalyzer (visualizzazione)
```

### Data Flow

```
User Input (password/retype)
         ↓
PasswordHandler (debounce 0.5s)
         ↓
strength_utils::evaluate_password_strength_tx()
         ↓ (via mpsc channel)
PasswordEvaluation { strength, reasons }
         ↓
StrengthAnalyzer (visualizzazione)
         ↓
User Feedback (colore + tooltip)
```

## Component Specifications

### 1. PasswordEvaluation Struct

**Location:** `src/backend/strength_utils.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PasswordEvaluation {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PasswordStrength {
    NotEvaluated,  // Nuovo valore
    WEAK,
    MEDIUM,
    STRONG,
}
```

**Color Mapping:**
- `NotEvaluated` → Grigio (`text-gray-500`)
- `WEAK` → Rosso (`text-error-600`)
- `MEDIUM` → Arancione (`text-warning-600`)
- `STRONG` → Verde (`text-success-600`)

### 2. strength_utils Refactoring

**Sezioni indipendenti (Single Responsibility Principle):**

Ogni sezione è una funzione che ritorna `Result<Option<String>, ()>`:
- `Ok(Some(reason))` → Fallito con reason
- `Ok(None)` → Superato
- `Err(())` → Errore fatale, interrompi tutto

**Sezioni:**
1. **Blacklist check**: Controlla se la password è nella lista delle 10k password comuni
2. **Length check**: Verifica che la password abbia almeno 8 caratteri
3. **Character variety**: Controlla presenza di maiuscole, minuscole, numeri, speciali
4. **Pattern analysis**: Calcola score penalizzando ripetizioni e sequenze

**Funzione principale:**
```rust
pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>,
)
```

**Orchestrator pattern:**
- For loop su sezioni
- Break su `token.is_cancelled()`
- Log errori con `tracing::error!()`
- Return `PasswordEvaluation` con reason "Error" in caso di errore

### 3. PasswordHandler Component

**Location:** `src/components/globals/password_handler/component.rs`

**Props:**
```rust
#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    pub on_password_change: Callback<FormSecret>,
    #[props(default = true)]
    pub password_required: bool,
}
```

**Internal State (use_signal):**
- `password: FormSecret`
- `repassword: FormSecret`
- `strength: PasswordStrength`
- `reasons: Vec<String>`
- `is_evaluating: bool`
- `debounce_task: Option<Task>` // Task handle for cancellation
- `cancel_token: Arc<CancellationToken>` // Arc per shared ownership

**Behavior:**
1. Utente digita in entrambi i campi
2. Quando `password == repassword` e non vuoti → avvia debounce timer (500ms)
3. Scaduto il timer → avvia valutazione con `evaluate_password_strength_tx()`
4. Modifica successiva → cancella valutazione corrente, resetta debounce
5. Risultato → aggiorna `strength` e `reasons`, chiama `on_password_change`
6. On unmount → cancella timer e token

**Debounce Logic:**
```rust
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

// Reset stato
strength.set(PasswordStrength::NotEvaluated);
reasons.set(Vec::new());

// Cancella task precedente
if let Some(task) = debounce_task.read().as_ref() {
    task.abort();
}
debounce_task.set(None);

// Cancella valutazione corrente
let token = Arc::new(CancellationToken::new());
cancel_token.set(token.clone());

// Avvia nuovo debounce timer (500ms)
let pwd_clone = password.read().clone();
let mut strength_sig = strength.clone();
let mut reasons_sig = reasons.clone();
let mut evaluating_sig = is_evaluating.clone();
let mut debounce_task_sig = debounce_task.clone();
let on_change = on_password_change.clone();

let task = spawn(async move {
    sleep(Duration::from_millis(500)).await;

    if token.is_cancelled() {
        return;
    }

    evaluating_sig.set(true);

    let (tx, mut rx) = mpsc::channel(1);
    evaluate_password_strength_tx(&pwd_clone.0, token.clone(), tx).await;

    if let Some(eval) = rx.recv().await {
        strength_sig.set(eval.strength);
        reasons_sig.set(eval.reasons);
        on_change.call(pwd_clone);
    }

    evaluating_sig.set(false);
});

debounce_task.set(Some(task));
```

### 4. StrengthAnalyzer Component

**Location:** `src/components/globals/password_handler/strength_analyzer.rs`

**Props:**
```rust
#[derive(Props, Clone, PartialEq)]
pub struct StrengthAnalyzerProps {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
    pub is_evaluating: bool,
}
```

**Visualizzazione:**
```
Strength: [Evaluating... / Weak / Medium / Strong] [?]?

Click [?] → Dropdown with:
Why this rating?
• Missing uppercase letters
• Missing special characters
```

**States:**
- `is_evaluating = true` → Mostra "Evaluating..." in grigio corsivo
- `strength = NotEvaluated` → Testo grigio
- `strength = WEAK/MEDIUM/STRONG` → Testo colorato
- `reasons` non vuoto → Mostra pulsante "(?)" con tooltip

### 5. Integration with UpsertUser

**Changes to `src/components/features/upsert_user.rs`:**

**Remove:**
- Signals `password` e `repassword`
- Due `FormField` per password e retype

**Add:**
- Signal `evaluated_password: Option<FormSecret>`
- Single `PasswordHandler` component

**Modified on_submit:**
```rust
let on_submit = move |_| {
    let pwd = match evaluated_password.read().clone() {
        Some(p) => p,
        None => {
            error.set(Some("Password evaluation not complete".to_string()));
            return;
        }
    };
    // ... rest of submit logic with pwd ...
};
```

**RSX:**
```rust
form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
    FormField {
        label: "Username",
        // ...
    }

    PasswordHandler {
        on_password_change: move |pwd| {
            evaluated_password.set(Some(pwd));
        },
        password_required: password_required,
    }

    ActionButton {
        // ...
    }
}
```

## CSS Styling

**Add to `assets/input_main.css`:**

```css
/* Password Handler */
@layer components {
  .password-handler {
    /* Layout container */
  }

  .strength-analyzer {
    min-height: 24px;
    position: relative;
  }

  .strength-info-btn {
    width: 20px;
    height: 20px;
    font-size: 0.75rem;
  }

  .strength-reasons-tooltip {
    animation: fadeIn 0.2s ease-out;
  }

  /* Force colors for password strength */
  .text-error-600 { color: #dc2626; }
  .text-warning-600 { color: #d97706; }
  .text-success-600 { color: #16a34a; }
  .text-gray-500 { color: #6b7280; }
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

## Error Handling & Edge Cases

| Case | Behavior |
|------|----------|
| Password vuote | `NotEvaluated`, grigio |
| Password non corrispondenti | Reason: "Passwords do not match" |
| Valutazione cancellata | `NotEvaluated`, reason: "Evaluation cancelled" |
| Errore durante valutazione | `NotEvaluated`, reason: "Error" (logged) |
| Utente lascia la pagina | Cleanup in `use_effect` |
| Debounce race condition | Ogni input cancella il precedente |

## Security Considerations

### Memory Dump Protection

- `SecretString` con `zeroize` on drop → protezione principale
- Controlli temporanei in chiaro → compromesso accettabile
- Minimizzazione tempo esposizione → cleanup immediato
- Log non contiene password in chiaro → solo messaggi di errore

### Logging Configuration

**File:** `gui_launcher/src/lib.rs`

- `tracing_subscriber::fmt()` con `tracing_appender`
- Log path: `%LOCALAPPDATA%/PWDManager/pwdmanager.log`
- Level: `INFO` (include `error!()`)
- WorkerGuard globale → garantisce scrittura fino a shutdown

## Testing Strategy

### Unit Tests (strength_utils)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blacklist_section() {
        // Test con password nella blacklist
    }

    #[test]
    fn test_length_section() {
        // Test varie lunghezze
    }

    #[test]
    fn test_variety_section() {
        // Test combinazioni caratteri
    }

    #[test]
    fn test_cancellation_at_checkpoint() {
        // Test cancellazione durante sezioni
    }
}
```

### Manual Testing Checklist

- [ ] Digitare password, vedere valutazione dopo 0.5s
- [ ] Modificare password durante valutazione, vedere reset
- [ ] Password diverse → reason "Passwords do not match"
- [ ] Password weak → rosso + reasons appropriate
- [ ] Password strong → verde + nessun reason
- [ ] Cliccare "?" → vedere tooltip con reasons
- [ ] Lasciare pagina → verificare cleanup (check log)
- [ ] Password in blacklist → weak immediato

## Implementation Flow

1. **Refactor strength_utils**
   - Add `PasswordEvaluation` struct
   - Add `NotEvaluated` to `PasswordStrength`
   - Create independent section functions
   - Implement for loop orchestrator
   - Add error logging with `tracing::error!()`

2. **Create StrengthAnalyzer**
   - File `strength_analyzer.rs`
   - Implement color-based rendering
   - Implement tooltip/dropdown for reasons

3. **Create PasswordHandler**
   - File `component.rs`
   - Implement internal state with signals
   - Implement debounce timer using `tokio::time::sleep` (NOT `set_interval`)
   - Use `Arc<CancellationToken>` for cancellation (CancellationToken is not Clone)
   - Use `Task` handle for spawn cancellation (NOT `Interval`)

4. **Integrate into UpsertUser**
   - Replace FormField pairs with PasswordHandler
   - Add evaluated_password signal
   - Modify on_submit to use evaluated password

5. **CSS and styling**
   - Add classes to `input_main.css`
   - Verify colors and animations

6. **Testing**
   - Manual tests as per checklist
   - Verify log file for errors

## Future Enhancements (Nice-to-Have)

- Password strength meter (progress bar)
- Password generation suggestions
- Real-time feedback during typing (before debounce)
- History of password strength trends
- Export evaluation results

## Dependencies

**Existing:**
- `dioxus` 0.7
- `secrecy` (SecretString)
- `tokio` (async runtime, time::sleep)
- `tracing` (logging)
- `tokio-util` (CancellationToken)
- `std::sync::Arc` (per CancellationToken shared ownership)

**No new dependencies required.**

## Migration Path

**From existing UpsertUser:**
1. Remove password/repassword FormField pairs
2. Add PasswordHandler component
3. Add evaluated_password signal
4. Update on_submit validation

**Backwards compatibility:**
- Legacy `evaluate_password_strength()` maintained with `#[deprecated]`
- No breaking changes to existing database operations

## Technical Notes

### Dioxus 0.7 Specific Considerations

**1. CancellationToken Arc Wrapper**
```rust
// CancellationToken non è Clone, usare Arc
use std::sync::Arc;
let cancel_token: Arc<CancellationToken> = Arc::new(CancellationToken::new());
let token_clone = Arc::clone(&cancel_token); // Per passare agli async tasks
```

**2. Debounce with tokio::time::sleep**
```rust
// NON usare set_interval per debounce (si ripete!)
// USARE tokio::time::sleep per esecuzione singola dopo delay
use tokio::time::{sleep, Duration};

sleep(Duration::from_millis(500)).await;
// poi esegui la valutazione...
```

**3. Task Cancellation**
```rust
// Usare spawn() che ritorna un Task
let task = spawn(async move { /* ... */ });

// Cancellare con abort()
if let Some(task) = debounce_task.read().as_ref() {
    task.abort();
}
```

**4. Cleanup on Unmount**
```rust
use_effect(move || {
    // Cleanup quando il componente viene smontato
    async move {
        if let Some(task) = debounce_task.read().as_ref() {
            task.abort();
        }
        cancel_token.cancel();
    }
});
```

### Type Safety Notes

- `FormSecret` wraps `SecretString` for UI components
- `PasswordEvaluation` contains both strength and reasons
- All password operations use `SecretString::expose_secret()` minimally
- Never log passwords in plain text - only error messages

## References

- `docs/howto_sqlitetemplate.md` - sqlx-template usage
- `docs/dioxus/` - Dioxus framework documentation
- `src/backend/strength_utils.rs` - Current implementation
- `src/components/features/upsert_user.rs` - Integration point
