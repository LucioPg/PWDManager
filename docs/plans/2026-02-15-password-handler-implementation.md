# PasswordHandler & StrengthAnalyzer - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implementare componenti riutilizzabili per input e valutazione password con debounce, feedback visivo e cancellazione granulare

**Architecture:**
- **PasswordHandler**: Container component che gestisce input password/retype-password, debounce timer (500ms), e cancellazione valutazioni
- **StrengthAnalyzer**: Visual-only component che mostra forza password con colori e tooltip reasons
- **PasswordEvaluation**: Nuovo struct che unisce strength + reasons per comunicazione atomica
- **Data flow**: User input → PasswordHandler (debounce) → strength_utils::evaluate_password_strength_tx (mpsc) → PasswordEvaluation → StrengthAnalyzer

**Tech Stack:** Dioxus 0.7, tokio (async/sleep/CancellationToken), tokio-util (CancellationToken), secrecy (SecretString), tracing (logging)

---

## Task 1: Aggiungere PasswordEvaluation e NotEvaluated a strength_utils

**Files:**
- Modify: `src/backend/strength_utils.rs`

**Step 1: Aggiungere struct PasswordEvaluation**

Apri `src/backend/strength_utils.rs` e aggiungi dopo gli import esistenti:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PasswordEvaluation {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}
```

**Step 2: Aggiungere variante NotEvaluated a PasswordStrength**

Trova l'enum `PasswordStrength` e aggiungi la variante:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PasswordStrength {
    NotEvaluated,  // Nuovo stato iniziale
    WEAK,
    MEDIUM,
    STRONG,
}
```

**Step 3: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/backend/strength_utils.rs
git commit -m "feat: add PasswordEvaluation struct and NotEvaluated to PasswordStrength"
```

---

## Task 2: Creare directory password_handler

**Files:**
- Create: `src/components/globals/password_handler/`
- Create: `src/components/globals/password_handler/mod.rs`

**Step 1: Creare directory structure**

Run: `mkdir -p src/components/globals/password_handler`

**Step 2: Creare mod.rs con esportazioni**

Crea `src/components/globals/password_handler/mod.rs`:

```rust
mod component;
mod strength_analyzer;

pub use component::PasswordHandler;
pub use strength_analyzer::StrengthAnalyzer;

// Re-export types per convenienza
pub use crate::backend::strength_utils::{PasswordEvaluation, PasswordStrength};
```

**Step 3: Aggiungere modulo al parent mod.rs**

Modifica `src/components/globals/mod.rs`, aggiungi:

```rust
pub mod password_handler;
```

**Step 4: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS (module empty per ora)

**Step 5: Commit**

```bash
git add src/components/globals/password_handler/ src/components/globals/mod.rs
git commit -m "feat: create password_handler module structure"
```

---

## Task 3: Scrivere test per refactoring sezioni strength_utils

**Files:**
- Create: `src/backend/strength_utils_tests.rs` (o aggiungere a strength_utils.rs come #[cfg(test)])
- Test: `src/backend/strength_utils.rs` (tests module)

**Step 1: Scrivere test per blacklist_section**

In `src/backend/strength_utils.rs` aggiungi alla fine:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_blacklist_section_with_common_password() {
        let pwd = SecretString::new("password".to_string());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(Some("Password is in the top 10,000 most common".to_string())));
    }

    #[test]
    fn test_blacklist_section_with_strong_password() {
        let pwd = SecretString::new("CorrectHorseBatteryStaple!123".to_string());
        let result = blacklist_section(&pwd);
        assert_eq!(result, Ok(None));
    }
}
```

**Step 2: Scrivere test per length_section**

```rust
    #[test]
    fn test_length_section_too_short() {
        let pwd = SecretString::new("Short1!".to_string());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(Some("Password must be at least 8 characters".to_string())));
    }

    #[test]
    fn test_length_section_valid() {
        let pwd = SecretString::new("LongEnough123!".to_string());
        let result = length_section(&pwd);
        assert_eq!(result, Ok(None));
    }
```

**Step 3: Scrivere test per character_variety_section**

```rust
    #[test]
    fn test_variety_section_missing_uppercase() {
        let pwd = SecretString::new("lowercase123!".to_string());
        let result = character_variety_section(&pwd);
        assert!(result.is_ok());
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("uppercase") || reason.contains("variety"));
        }
    }

    #[test]
    fn test_variety_section_all_categories() {
        let pwd = SecretString::new("HasAll123!@#".to_string());
        let result = character_variety_section(&pwd);
        assert_eq!(result, Ok(None));
    }
```

**Step 4: Scrivere test per pattern_analysis_section**

```rust
    #[test]
    fn test_pattern_section_repetitive() {
        let pwd = SecretString::new("aaaaBBBB1111".to_string());
        let result = pattern_analysis_section(&pwd);
        assert!(result.is_ok());
        if let Ok(Some(reason)) = result {
            assert!(reason.contains("repetitive") || reason.contains("pattern"));
        }
    }

    #[test]
    fn test_pattern_section_strong() {
        let pwd = SecretString::new("RandomPass123!@#Word".to_string());
        let result = pattern_analysis_section(&pwd);
        assert_eq!(result, Ok(None));
    }
```

**Step 5: Verifica che i test falliscano (funzioni non ancora esistono)**

Run: `cargo test strength_utils::tests --lib`
Expected: COMPILE ERROR - functions not found

**Step 6: Commit skeleton tests**

```bash
git add src/backend/strength_utils.rs
git commit -m "test: add failing tests for strength_utils section functions"
```

---

## Task 4: Implementare funzioni sezione indipendenti (TDD)

**Files:**
- Modify: `src/backend/strength_utils.rs`

**Step 1: Implementare blacklist_section**

Prima della funzione `evaluate_password_strength_tx`, aggiungi:

```rust
/// Controlla se la password è nella blacklist delle 10k password comuni
/// Returns: Ok(Some(reason)) se in blacklist, Ok(None) se safe, Err(()) per errore fatale
fn blacklist_section(password: &SecretString) -> Result<Option<String>, ()> {
    use std::sync::OnceLock;

    // Lazy-loaded blacklist (solo prime 10 per esempio reale)
    static BLACKLIST: OnceLock<std::collections::HashSet<String>> = OnceLock::new();

    let blacklist = BLACKLIST.get_or_init(|| {
        // In production, caricare da file
        [
            "password", "12345678", "qwerty", "abc123", "monkey",
            "master", "dragon", "111111", "baseball", "trustno1",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    });

    let pwd = password.expose_secret();
    if blacklist.contains(pwd) {
        return Ok(Some("Password is in the top 10,000 most common".to_string()));
    }

    Ok(None)
}
```

**Step 2: Run test blacklist**

Run: `cargo test blacklist_section --lib`
Expected: PASS (2 tests passano)

**Step 3: Implementare length_section**

```rust
/// Verifica lunghezza minima password
fn length_section(password: &SecretString) -> Result<Option<String>, ()> {
    const MIN_LENGTH: usize = 8;

    if password.expose_secret().len() < MIN_LENGTH {
        return Ok(Some(format!("Password must be at least {} characters", MIN_LENGTH)));
    }

    Ok(None)
}
```

**Step 4: Run test length**

Run: `cargo test length_section --lib`
Expected: PASS (2 tests passano)

**Step 5: Implementare character_variety_section**

```rust
/// Verifica presenza di maiuscole, minuscole, numeri, speciali
fn character_variety_section(password: &SecretString) -> Result<Option<String>, ()> {
    let pwd = password.expose_secret();

    let has_upper = pwd.chars().any(|c| c.is_uppercase());
    let has_lower = pwd.chars().any(|c| c.is_lowercase());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
    let has_special = pwd.chars().any(|c| !c.is_alphanumeric());

    let missing = vec![
        if !has_upper { Some("uppercase") } else { None },
        if !has_lower { Some("lowercase") } else { None },
        if !has_digit { Some("numbers") } else { None },
        if !has_special { Some("special characters") } else { None },
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if !missing.is_empty() {
        return Ok(Some(format!("Missing: {}", missing.join(", "))));
    }

    Ok(None)
}
```

**Step 6: Run test variety**

Run: `cargo test character_variety_section --lib`
Expected: PASS (2 tests passano)

**Step 7: Implementare pattern_analysis_section**

```rust
/// Analizza pattern per penalizzare ripetizioni e sequenze
fn pattern_analysis_section(password: &SecretString) -> Result<Option<String>, ()> {
    let pwd = password.expose_secret();
    let chars: Vec<char> = pwd.chars().collect();

    if chars.len() < 3 {
        return Ok(None);
    }

    // Controlla sequenze ripetute (es. "aaa", "111")
    let mut repeated_count = 1;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            repeated_count += 1;
            if repeated_count >= 3 {
                return Ok(Some("Password contains repeated characters".to_string()));
            }
        } else {
            repeated_count = 1;
        }
    }

    // Controlla sequenze (es. "abc", "123")
    for i in 2..chars.len() {
        let prev_prev = chars[i - 2] as u32;
        let prev = chars[i - 1] as u32;
        let curr = chars[i] as u32;

        // Sequenza crescente (es. "abc", "123")
        if curr == prev + 1 && prev == prev_prev + 1 {
            return Ok(Some("Password contains sequential characters".to_string()));
        }

        // Sequenza decrescente (es. "cba", "321")
        if curr as i32 == prev as i32 - 1 && prev as i32 == prev_prev as i32 - 1 {
            return Ok(Some("Password contains sequential characters".to_string()));
        }
    }

    Ok(None)
}
```

**Step 8: Run test pattern**

Run: `cargo test pattern_analysis_section --lib`
Expected: PASS (2 tests passano)

**Step 9: Run tutti i test strength_utils**

Run: `cargo test strength_utils::tests --lib`
Expected: PASS (tutti i test passano)

**Step 10: Commit**

```bash
git add src/backend/strength_utils.rs
git commit -m "feat: implement independent section functions for password evaluation"
```

---

## Task 5: Refactor evaluate_password_strength_tx con orchestrator pattern

**Files:**
- Modify: `src/backend/strength_utils.rs`

**Step 1: Sostituire implementazione esistente con orchestrator**

Trova `evaluate_password_strength_tx` e sostituisci con:

```rust
pub async fn evaluate_password_strength_tx(
    password: &SecretString,
    token: CancellationToken,
    tx: mpsc::Sender<PasswordEvaluation>,
) {
    use tracing::error;

    let mut reasons = Vec::new();
    let mut strength = PasswordStrength::NotEvaluated;

    // Orchestrator: esegui sezioni in sequenza
    let sections: Vec<(&str, fn(&SecretString) -> Result<Option<String>, ()>)> = vec![
        ("blacklist", blacklist_section),
        ("length", length_section),
        ("variety", character_variety_section),
        ("pattern", pattern_analysis_section),
    ];

    for (section_name, section_fn) in sections {
        // Check cancellation prima di ogni sezione
        if token.is_cancelled() {
            strength = PasswordStrength::NotEvaluated;
            reasons.push("Evaluation cancelled".to_string());
            break;
        }

        match section_fn(password) {
            Ok(Some(reason)) => {
                reasons.push(reason);
            }
            Ok(None) => {
                // Sezione passata, continua
            }
            Err(()) => {
                error!(section = %section_name, "Fatal error in password evaluation section");
                reasons.push("Error".to_string());
                strength = PasswordStrength::NotEvaluated;
                break;
            }
        }
    }

    // Calcola strength finale basata su reasons
    if strength != PasswordStrength::NotEvaluated {
        strength = if reasons.is_empty() {
            PasswordStrength::STRONG
        } else if reasons.len() <= 2 {
            PasswordStrength::MEDIUM
        } else {
            PasswordStrength::WEAK
        };
    }

    let evaluation = PasswordEvaluation { strength, reasons };

    // Invia risultato
    if let Err(e) = tx.send(evaluation).await {
        error!(error = %e, "Failed to send password evaluation result");
    }
}
```

**Step 2: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/backend/strength_utils.rs
git commit -m "refactor: implement orchestrator pattern for password evaluation with cancellation support"
```

---

## Task 6: Creare StrengthAnalyzer component

**Files:**
- Create: `src/components/globals/password_handler/strength_analyzer.rs`

**Step 1: Scrivere test di rendering (opzionale - skip per UI component, facoltativo)**

Per UI components in Dioxus, i test sono difficili. Saltiamo e procediamo direttamente all'implementazione.

**Step 2: Implementare StrengthAnalyzer component**

Crea `src/components/globals/password_handler/strength_analyzer.rs`:

```rust
use dioxus::prelude::*;

use super::PasswordStrength;

#[derive(Props, Clone, PartialEq)]
pub struct StrengthAnalyzerProps {
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
    #[props(default)]
    pub is_evaluating: bool,
}

#[component]
pub fn StrengthAnalyzer(props: StrengthAnalyzerProps) -> Element {
    let show_tooltip = use_signal(|| false);

    // Color mapping
    let (text_class, strength_text) = match props.strength {
        PasswordStrength::NotEvaluated => ("text-gray-500", "Not evaluated".to_string()),
        PasswordStrength::WEAK => ("text-error-600", "Weak".to_string()),
        PasswordStrength::MEDIUM => ("text-warning-600", "Medium".to_string()),
        PasswordStrength::STRONG => ("text-success-600", "Strong".to_string()),
    };

    rsx! {
        div { class: "strength-analyzer flex items-center gap-2",
            // Stato evaluating
            if props.is_evaluating {
                span { class: "text-gray-500 italic", "Evaluating..." }
            } else {
                // Strength text
                span { class: "{text_class} font-medium", "{strength_text}" }

                // Tooltip button con (?)
                if !props.reasons.is_empty() {
                    div { class: "relative",
                        button {
                            class: "strength-info-btn btn btn-circle btn-ghost btn-xs",
                            r#type: "button",
                            onclick: move |_| show_tooltip.set(!show_tooltip()),
                            "?"
                        }

                        // Tooltip dropdown
                        if show_tooltip() {
                            div { class: "strength-reasons-tooltip absolute top-full left-0 mt-2 z-10",
                                div { class: "dropdown-content mockup-code bg-base-200 shadow-lg rounded-lg p-3 min-w-[200px]",
                                    h4 { class: "font-bold text-sm mb-2", "Why this rating?" }
                                    ul { class: "text-xs space-y-1",
                                        for reason in &props.reasons {
                                            li { class: "flex items-start gap-1",
                                                span { class: "text-base-content/70", "•" }
                                                span { "{reason}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

**Step 3: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Aggiungere CSS per StrengthAnalyzer**

Modifica `assets/input_main.css`, aggiungi alla fine:

```css
/* Password Handler - Strength Analyzer */
@layer components {
  .strength-analyzer {
    min-height: 24px;
  }

  .strength-info-btn {
    width: 20px;
    height: 20px;
    font-size: 0.75rem;
  }

  .strength-reasons-tooltip {
    animation: fadeIn 0.2s ease-out;
  }
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

**Step 5: Verifica che le classi CSS siano registrate**

Le classi usate sono già in `input_main.css` o sono Tailwind standard. Verifica:

Run: `cargo check`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add src/components/globals/password_handler/strength_analyzer.rs assets/input_main.css
git commit -m "feat: add StrengthAnalyzer component with tooltip for password evaluation reasons"
```

---

## Task 7: Implementare PasswordHandler con debounce

**Files:**
- Create: `src/components/globals/password_handler/component.rs`

**Step 1: Scrivere struct Props e implementare base component**

Crea `src/components/globals/password_handler/component.rs`:

```rust
use dioxus::prelude::*;
use secrecy::SecretString;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

use super::PasswordEvaluation;
use crate::backend::strength_utils::{evaluate_password_strength_tx, PasswordStrength};
use crate::components::globals::form::FormSecret;

#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    pub on_password_change: Callback<FormSecret>,
    #[props(default = true)]
    pub password_required: bool,
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    // Internal state
    let password = use_signal(|| FormSecret(SecretString::new(String::new())));
    let repassword = use_signal(|| FormSecret(SecretString::new(String::new())));
    let strength = use_signal(|| PasswordStrength::NotEvaluated);
    let reasons = use_signal(|| Vec::<String>::new());
    let is_evaluating = use_signal(|| false);

    let debounce_task = use_signal(|| None::<dioxus::prelude::Task>);
    let cancel_token = use_signal(|| Arc::new(CancellationToken::new()));

    rsx! {
        // Placeholder - implementato nei prossimi step
        div { "Password Handler Component" }
    }
}
```

**Step 2: Verifica compilazione base**

Run: `cargo check`
Expected: SUCCESS

**Step 3: Implementare handler per password input con debounce logica**

Sostituisci il placeholder rsx! con implementazione completa:

```rust
#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    // Internal state
    let password = use_signal(|| FormSecret(SecretString::new(String::new())));
    let repassword = use_signal(|| FormSecret(SecretString::new(String::new())));
    let strength = use_signal(|| PasswordStrength::NotEvaluated);
    let reasons = use_signal(|| Vec::<String>::new());
    let is_evaluating = use_signal(|| false);

    let debounce_task = use_signal(|| None::<dioxus::prelude::Task>);
    let cancel_token = use_signal(|| Arc::new(CancellationToken::new()));

    // Cleanup on unmount
    use_effect(move || {
        async move {
            // Cleanup quando il componente viene smontato
        }
    });

    // Handler per input password
    let on_password_input = move |pwd: FormSecret| {
        password.set(pwd.clone());

        // Reset stato valutazione
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

        // Controlla se le password corrispondono e non sono vuote
        let pwd_match = password.read().0.expose_secret() == repassword.read().0.expose_secret();
        let is_empty = password.read().0.expose_secret().is_empty();

        if !is_empty && pwd_match {
            // Avvia debounce timer (500ms)
            let pwd_clone = pwd.clone();
            let mut strength_sig = strength.clone();
            let mut reasons_sig = reasons.clone();
            let mut evaluating_sig = is_evaluating.clone();
            let mut debounce_task_sig = debounce_task.clone();
            let on_change = props.on_password_change.clone();

            let task = dioxus::prelude::spawn(async move {
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
        }
    };

    // Handler per input repassword
    let on_repassword_input = move |re_pwd: FormSecret| {
        repassword.set(re_pwd);

        // Reset e avvia debounce se password match
        let pwd = password.read().clone();
        on_password_input(pwd);
    };

    rsx! {
        div { class: "password-handler flex flex-col gap-3",
            // Password field
            crate::components::globals::form::FormField {
                label: "Password",
                value: password.read().clone(),
                on_input: on_password_input,
                input_type: crate::components::globals::form::InputType::Password,
                required: props.password_required,
            }

            // Retype password field
            crate::components::globals::form::FormField {
                label: "Confirm Password",
                value: repassword.read().clone(),
                on_input: on_repassword_input,
                input_type: crate::components::globals::form::InputType::Password,
                required: props.password_required,
            }

            // Strength analyzer
            super::StrengthAnalyzer {
                strength: strength.read().clone(),
                reasons: reasons.read().clone(),
                is_evaluating: is_evaluating(),
            }

            // Password mismatch warning
            if !password.read().0.expose_secret().is_empty()
                && !repassword.read().0.expose_secret().is_empty()
                && password.read().0.expose_secret() != repassword.read().0.expose_secret()
            {
                div { class: "text-error-600 text-sm", "Passwords do not match" }
            }
        }
    }
}
```

**Step 4: Verifica compilazione completa**

Run: `cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src/components/globals/password_handler/component.rs
git commit -m "feat: implement PasswordHandler with debounce and cancellation support"
```

---

## Task 8: Aggiungere cleanup per use_effect

**Files:**
- Modify: `src/components/globals/password_handler/component.rs`

**Step 1: Implementare cleanup use_effect**

Sostituisci il use_effect placeholder con:

```rust
    // Cleanup on unmount
    use_effect(move || {
        async move {
            // Cleanup quando il componente viene smontato - gestito dal drop
        }
    });

    // Cleanup effect
    use_effect(move || {
        let task = debounce_task.read().clone();
        let token = cancel_token.read().clone();

        async move {
            // Quando il componente viene smontato, cancella task e token
            if let Some(t) = task {
                t.abort();
            }
            token.cancel();
        }
    });
```

**Step 2: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/components/globals/password_handler/component.rs
git commit -m "fix: add cleanup effect for PasswordHandler debounce and cancellation"
```

---

## Task 9: Integrazione in UpsertUser

**Files:**
- Modify: `src/components/features/upsert_user.rs`

**Step 1: Rimuovere vecchi signal password/repassword**

Trova e rimuovi in `upsert_user.rs`:

```rust
// DA RIMUOVERE:
let password = use_signal(|| FormSecret(SecretString::new(String::new())));
let repassword = use_signal(|| FormSecret(SecretString::new(String::new())));
```

**Step 2: Aggiungere nuovo signal evaluated_password**

Aggiungi dopo le altre variabili state:

```rust
let evaluated_password = use_signal(|| Option::<FormSecret>::None);
```

**Step 3: Sostituire FormField pairs con PasswordHandler**

Trova i due FormField per password e sostituisci con:

```rust
// Sostituisci i FormField password/retype con:
crate::components::globals::password_handler::PasswordHandler {
    on_password_change: move |pwd| {
        evaluated_password.set(Some(pwd));
    },
    password_required: password_required,
}
```

**Step 4: Modificare on_submit per usare evaluated_password**

Trova `on_submit` e modifica:

```rust
let on_submit = move |_| {
    // Reset errori
    error.set(None);

    // Validazione password valutata
    let pwd = match evaluated_password.read().clone() {
        Some(p) => p,
        None => {
            error.set(Some("Password evaluation required".to_string()));
            return;
        }
    };

    // Usa pwd per il resto della logica...
    // ... resto del codice esistente ...
};
```

**Step 5: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 6: Test manuale in dev**

Run: `dx serve --desktop`
Test:
- Vai a /register
- Digita password diverse → vedi "Passwords do not match"
- Digita password uguali weak → vedi "Weak" in rosso con (?)
- Clicca (?) → vedi tooltip con reasons
- Digita password forte → vedi "Strong" in verde

**Step 7: Commit**

```bash
git add src/components/features/upsert_user.rs
git commit -m "refactor: integrate PasswordHandler in UpsertUser replacing FormField pairs"
```

---

## Task 10: Aggiungere color classes CSS se mancanti

**Files:**
- Modify: `assets/input_main.css`

**Step 1: Verificare presenza color classes**

Cerca in `input_main.css`:

```css
.text-error-600
.text-warning-600
.text-success-600
.text-gray-500
```

**Step 2: Aggiungere se mancanti**

Se non esistono, aggiungi al fondo:

```css
@layer utilities {
  /* Force colors for password strength */
  .text-error-600 { color: #dc2626; }
  .text-warning-600 { color: #d97706; }
  .text-success-600 { color: #16a34a; }
  .text-gray-500 { color: #6b7280; }
}
```

**Step 3: Verifica compilazione**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add assets/input_main.css
git commit -m "style: add password strength color utility classes"
```

---

## Task 11: Testing manuale completo

**Files:**
- None (manual testing)

**Step 1: Avvio applicazione**

Run: `dx serve --desktop`

**Step 2: Checklist test manuale**

- [ ] Naviga a /register
- [ ] Digita in password (lascia vuoto confirm) → nessuna valutazione
- [ ] Digita in confirm password (diversa) → "Passwords do not match"
- [ ] Modifica password per matchare (dopo 0.5s) → appare "Evaluating..." poi "Weak/Medium/Strong"
- [ ] Password "123" → Weak rosso + reasons (length, variety)
- [ ] Clicca (?) → tooltip con lista reasons
- [ ] Password "SecurePass123!" → Strong verde + nessun (?) o reasons
- [ ] Modifica password rapidamente → valutazione precedente cancellata
- [ ] Password in blacklist ("password") → Weak immediato

**Step 3: Verifica log per errori**

Controlla `%LOCALAPPDATA%/PWDManager/pwdmanager.log` per error tracing.

**Step 4: Commit not test completati**

Non serve commit per test manuale, ma documenta in memoria.

---

## Task 12: Documentazione e cleanup

**Files:**
- Create: `docs/testing/password_handler_testing.md` (opzionale)

**Step 1: Aggiorna MEMORY.md se necessario**

Se ci sono nuove convenzioni da ricordare:

```markdown
## PasswordHandler Pattern

Quando usi PasswordHandler:
- Valuta sempre tramite evaluated_password Option<FormSecret>
- Non accedere direttamente a password/repassword signals
- Usa on_password_change callback per ricevere FormSecret valutato
```

**Step 2: Verifica documentazione esistente**

Controlla che `docs/howto_sqlitetemplate.md` sia aggiornato.

**Step 3: Verifica CLAUDE.md**

Se ci sono nuovi pattern da documentare per future sessioni.

**Step 4: Commit finale documentazione**

```bash
git add MEMORY.md docs/
git commit -m "docs: update project memory with PasswordHandler patterns"
```

---

## Final Verification

**Step 1: Run tutti i test**

Run: `cargo test --lib`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Run format check**

Run: `cargo fmt --check`
Se fallisce: `cargo fmt`

**Step 4: Final commit**

```bash
git add .
git commit -m "chore: final formatting and cleanup for PasswordHandler implementation"
```

---

## Dipendenze

Nessuna dipendenza nuova richiesta. Tutte già presenti:
- `dioxus` 0.7
- `secrecy` (SecretString)
- `tokio` (async runtime, time::sleep)
- `tracing` (logging)
- `tokio-util` (CancellationToken)
- `std::sync::Arc` (per CancellationToken shared ownership)

---

## Note Tecniche

### CancellationToken Arc Wrapper

`CancellationToken` non è Clone, usare Arc:

```rust
use std::sync::Arc;
let cancel_token: Arc<CancellationToken> = Arc::new(CancellationToken::new());
let token_clone = Arc::clone(&cancel_token); // Per passare agli async tasks
```

### Debounce con tokio::time::sleep

NON usare `set_interval` per debounce (si ripete!).
USARE `tokio::time::sleep` per esecuzione singola dopo delay.

### Task Cancellation

Usare `spawn()` che ritorna un Task, cancellare con `abort()`.

---

## Implementation Status & Issues Log

**Last Updated:** 2026-02-15
**Branch:** dev-passwordhandler-strenghtanalyzer--dioxus-19
**Session:** Subagent-driven development execution (PARTIAL - CRITICAL BUG FOUND)

### Completed Tasks (1-9) ✅

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| 1. Add PasswordEvaluation & NotEvaluated | ✅ Complete | b160824 | Types added correctly |
| 2. Create password_handler directory | ✅ Complete | d2c6b0e | Module structure created |
| 3. Write tests for section functions | ✅ Complete | 43512b9 | 8 tests written (TDD) |
| 4. Implement section functions | ✅ Complete | b95068f | All 4 functions working |
| 5. Refactor evaluate_password_strength_tx | ✅ Complete | 3929a87 | Orchestrator pattern implemented |
| 6. Create StrengthAnalyzer | ✅ Complete | 7551191 | Visual component with tooltip |
| 7. Implement PasswordHandler | ✅ Complete | eecd205 | With cleanup (fixed after review) |
| 8. Add cleanup for use_effect | ✅ Complete | eecd205 | use_drop hook added |
| 9. Integrate in UpsertUser | ✅ Complete | fb467a1 | With validation fix |

### Bug Fix #1: Asset Loading Issue 🔧

**Issue:** Password evaluation not working - blacklist never loads

**Root Cause:**
- Dioxus asset system uses relative path resolution that fails at runtime
- `COMMON_PASSWORDS` remains uninitialized (None)
- `println!`/`eprintln!` don't write to tracing log file

**Fix Applied:** Commit `5bc5615`
```rust
// OLD (broken):
static BLACKLIST_ASSET: Asset = asset!("/assets/10k-most-common.txt", ...);
// Fails silently - relative path resolution

// NEW (working):
const BLACKLIST_CONTENT: &str = include_str!("../../assets/10k-most-common.txt");
// Embedded at compile-time - always works
```

**Also fixed:** Logging now uses `tracing::info!`/`tracing::error!`

---

### Bug Fix #2: Password Evaluation Not Triggering 🔧

**Issue:** Password evaluation never starts when user types in password fields

**Symptoms:**
- PasswordHandler component renders correctly
- FormField inputs accept text
- BUT: Evaluation process NEVER triggers
- StrengthAnalyzer always shows "Not evaluated"

**Root Cause Analysis:**

Il problema era nell'uso di `use_effect` per monitorare i cambiamenti dei signal `password` e `repassword`.

**Problema #1: Closure move issue (Fix attempt #1)**
```rust
// PROBLEMATICO: Due use_effect che chiamano la stessa closure
let mut trigger_evaluation = move || { /* ... */ };

use_effect(move || {
    let _ = password.read();
    trigger_evaluation();  // Closure viene MOSSA qui
});

use_effect(move || {
    let _ = repassword.read();
    trigger_evaluation();  // Closure non più disponibile!
});
```
Fix: Commit `9dc6001` - Unificato in un singolo `use_effect`

**Problema #2: use_effect non rileva cambiamenti (Fix attempt #2)**
Anche con un singolo `use_effect`, la valutazione non triggerava. Il log mostrava:
```
use_effect triggered password_len=0 repassword_len=0
```
Il signal non veniva aggiornato quando l'utente digitava.

**Root Cause:** In Dioxus 0.7, `use_effect` ha timing issues quando si tratta di rilevare cambiamenti ai signal passati come props a componenti figli. Il FormField aggiorna il signal, ma l'effect potrebbe non essere notificato correttamente.

**Soluzione Finale:** Commit `3a302e9` - **Callback-based approach**

Invece di usare `use_effect` per "osservare" cambiamenti, usare callback espliciti:

```rust
// FormField ora accetta un callback opzionale
#[component]
pub fn FormField<T: FormValue>(
    // ... altri props ...
    #[props(default)]
    on_change: Option<Callback<T>>,  // <-- NUOVO
) -> Element {
    rsx! {
        input {
            oninput: move |e| {
                if let Some(new_value) = T::from_form_string(e.value()) {
                    value.set(new_value.clone());
                    if let Some(callback) = on_change {
                        callback.call(new_value);  // <-- Chiama subito
                    }
                }
            },
        }
    }
}
```

```rust
// PasswordHandler usa callback espliciti
let on_password_change = move |new_pwd: FormSecret| {
    password.set(new_pwd.clone());
    // ... logica di valutazione ...
    if !is_empty && pwd_match {
        spawn(async move { /* debounce + evaluate */ });
    }
};

// Passa il callback al FormField
FormField::<FormSecret> {
    value: password,
    on_change: on_password_change,  // <-- Callback esplicito
}
```

**Perché funziona:**
1. Il callback viene chiamato **immediatamente** dopo `value.set()` nel FormField
2. Non c'è latenza o timing issue tra signal update e effect execution
3. È il pattern idiomatico in Dioxus per gestire input form

**Files modificati:**
- `src/components/globals/form_field.rs`: Aggiunto `on_change: Option<Callback<T>>` prop
- `src/components/globals/password_handler/component.rs`: Sostituito `use_effect` con callback

**Lezione imparata:** In Dioxus 0.7, per gestire reazione a input utente, preferire **callback espliciti** invece di `use_effect` per osservare signal. `use_effect` è più adatto per side effects che non dipendono direttamente da interazioni utente.

---

### Bug Fix #3: Strength Sempre "Not Evaluated" 🔧

**Issue:** La valutazione della password veniva eseguita correttamente (le reasons apparivano nel tooltip), ma lo strength rimaneva sempre "Not Evaluated".

**Root Cause:**
In `evaluate_password_strength_tx()`, la condizione per calcolare lo strength era sbagliata:

```rust
// CODICE BUGGATO
let mut strength = PasswordStrength::NotEvaluated;
// ... esegui sezioni ...

if strength != PasswordStrength::NotEvaluated {  // SEMPRE FALSE!
    strength = if reasons.is_empty() {
        PasswordStrength::STRONG
    } else if reasons.len() <= 2 {
        PasswordStrength::MEDIUM
    } else {
        PasswordStrength::WEAK
    };
}
```

La variabile `strength` viene inizializzata a `NotEvaluated` e la condizione `if strength != NotEvaluated` è **sempre false**, quindi lo strength non viene mai calcolato.

**Fix Applied:** Commit `d6d934d`
```rust
// CODICE CORRETTO
strength = if reasons.is_empty() {
    PasswordStrength::STRONG
} else if reasons.len() <= 2 {
    PasswordStrength::MEDIUM
} else {
    PasswordStrength::WEAK
};
```

Rimossa la condizione errata - lo strength deve sempre essere calcolato alla fine della valutazione.

**File modificato:** `src/backend/strength_utils.rs`

---

### Git History Summary

```
3f63cef - chore: remove debug tracing from password handler
d6d934d - fix: remove incorrect condition in strength calculation
64dbc3e - docs: document use_effect issues and callback-based solution
3a302e9 - refactor: use callback-based approach for password evaluation
9dc6001 - fix: unify password evaluation use_effect to fix closure move bug
5bc5615 - fix: use include_str! for password blacklist
fb467a1 - refactor: integrate PasswordHandler in UpsertUser
eecd205 - feat: implement PasswordHandler with debounce
7551191 - feat: add StrengthAnalyzer component
3929a87 - refactor: orchestrator pattern for evaluation
b95068f - feat: implement section functions
43512b9 - test: add failing tests
d2c6b0e - feat: create module structure
b160824 - feat: add PasswordEvaluation types
```

---

### Current Status (After Bug Fix #3)

**Status:** ✅ EVALUATION WORKING

**What Works:**
- ✅ Types compile
- ✅ Section functions work
- ✅ Components render
- ✅ Blacklist loads
- ✅ FormField accepts input with callback
- ✅ Debounce task spawns correctly
- ✅ Evaluation executes and returns results
- ✅ Strength displays correctly (Weak/Medium/Strong)
- ✅ Reasons show in tooltip

**Minor issues to address:**
- Tooltip text color (fixed in CSS)

**Next Step:** Task 11 - Final Manual Testing Checklist

---

### Pattern Raccomandato per Form Input in Dioxus 0.7

```rust
// ✅ CORRETTO: Callback-based
FormField {
    value: my_signal,
    on_change: |new_value| {
        my_signal.set(new_value);
        trigger_side_effect();
    },
}

// ❌ EVITARE: use_effect per monitorare input
use_effect(move || {
    let _ = my_signal.read();  // Timing issues!
    trigger_side_effect();
});
```
