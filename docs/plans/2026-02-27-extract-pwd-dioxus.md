# Piano: Estrazione `pwd-dioxus`

> **Per Claude:** Usa `superpowers:executing-plans` per implementare task per task.
>
> **Prerequisiti:**
> - Leggere `docs/plans/2026-02-26-library-extraction-orchestrator.md` (stato, lezioni apprese)
> - Leggere `docs/plans/2026-02-27-pwd-dioxus-architecture.md` (architettura atomica)

**Goal:** Estrarre componenti UI Dioxus in libreria riutilizzabile con struttura atomica.

**Architettura:** 6 moduli atomici con feature flags granulari.

---

## Task Overview

| # | Task | Dipende da | Output |
|---|------|------------|--------|
| 1 | Creare struttura crate | - | Directory `pwd-dioxus/` |
| 2 | Estrarre modulo `icons` | 1 | Icone SVG riusabili |
| 3 | Estrarre modulo `spinner` | 1 | Loading indicator |
| 4 | Estrarre modulo `modal` | 1 | BaseModal wrapper |
| 5 | Estrarre modulo `form` | 2 | FormField + types |
| 6 | Estrarre modulo `secret` | 2, 5 | SecretDisplay |
| 7 | Estrarre modulo `password` | 2, 3, 5 | StrengthAnalyzer + PasswordHandler |
| 8 | Estrarre CSS | 1-7 | components.css |
| 9 | Aggiornare progetto padre | 1-8 | Wrapper backward-compatible |
| 10 | Verifica e commit | 1-9 | Commit finale |

---

## Task 1: Creare Struttura Crate

### 1.1 Creare directory

```bash
mkdir -p pwd-dioxus/src/{icons,spinner,modal,form,secret,password}
mkdir -p pwd-dioxus/assets
```

### 1.2 Creare Cargo.toml

```toml
[package]
name = "pwd-dioxus"
version = "0.1.0"
edition = "2024"
description = "Reusable Dioxus components for password handling"

[dependencies]
dioxus = { version = "0.7", default-features = false, optional = true }
secrecy = { version = "0.10", optional = true }
pwd-types = { path = "../pwd-types", optional = true }

[features]
default = ["spinner", "icons-visibility"]

# === ATOMICI ===
# Nota: icons-* non richiedono feature esplicita per dioxus qui,
# il modulo icons è incluso se ANY delle feature icons* è attiva
icons = []
icons-visibility = []
icons-action = []
icons-alert = []
spinner = ["dep:dioxus"]
modal = ["dep:dioxus"]

# === COMPOSITI ===
form = ["icons-visibility", "dep:secrecy", "dep:dioxus"]
secret-display = ["form", "icons-visibility", "icons-action"]
analyzer = ["spinner", "dep:pwd-types"]
handler = ["form", "analyzer", "icons-action", "dep:secrecy"]

# === CONVENIENCE ===
full = ["icons", "spinner", "modal", "handler"]
```

### 1.3 Aggiornare workspace

In `Cargo.toml` root:
```toml
[workspace]
members = ["gui_launcher", ".", "custom_errors", "pwd-types", "pwd-strength", "pwd-crypto", "pwd-dioxus"]
```

### 1.4 Creare lib.rs skeleton

```rust
//! Reusable Dioxus components for password handling.

// Icons: incluso se qualsiasi feature icons* è attiva
#[cfg(any(feature = "icons", feature = "icons-visibility",
          feature = "icons-action", feature = "icons-alert"))]
pub mod icons;

#[cfg(feature = "spinner")]
pub mod spinner;

#[cfg(feature = "modal")]
pub mod modal;

#[cfg(feature = "form")]
pub mod form;

#[cfg(feature = "secret-display")]
pub mod secret;

// password module: incluso per analyzer o handler
#[cfg(any(feature = "analyzer", feature = "handler"))]
pub mod password;
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus` compila (anche con errori nei moduli)

---

## Task 2: Estrarre Modulo `icons`

### 2.1 Copiare file

```
src/components/globals/svgs/base_icon.rs → pwd-dioxus/src/icons/base.rs
src/components/globals/svgs/visibility_icons.rs → pwd-dioxus/src/icons/visibility.rs
src/components/globals/svgs/action_icons.rs → pwd-dioxus/src/icons/action.rs
src/components/globals/svgs/alert_icons.rs → pwd-dioxus/src/icons/alert.rs
```

### 2.2 Creare mod.rs

```rust
// pwd-dioxus/src/icons/mod.rs

// Base è sempre incluso quando il modulo icons è attivo
mod base;
pub use base::SvgIcon;

// Visibility icons (EyeIcon, EyeOffIcon)
#[cfg(any(feature = "icons-visibility", feature = "icons"))]
mod visibility;
#[cfg(any(feature = "icons-visibility", feature = "icons"))]
pub use visibility::{EyeIcon, EyeOffIcon};

// Action icons (BurgerIcon, ClipboardIcon, DeleteIcon, EditIcon, MagicWandIcon)
#[cfg(any(feature = "icons-action", feature = "icons"))]
mod action;
#[cfg(any(feature = "icons-action", feature = "icons"))]
pub use action::{BurgerIcon, ClipboardIcon, DeleteIcon, EditIcon, MagicWandIcon};

// Alert icons (WarningIcon, LogoutIcon)
#[cfg(any(feature = "icons-alert", feature = "icons"))]
mod alert;
#[cfg(any(feature = "icons-alert", feature = "icons"))]
pub use alert::{WarningIcon, LogoutIcon};
```

### 2.3 Aggiornare import nei file icona

In `visibility.rs`, `action.rs`, `alert.rs` cambiare:
```rust
// Vecchio
use super::base_icon::SvgIcon;

// Nuovo
use super::base::SvgIcon;
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features icons-visibility` compila
- [ ] `cargo check -p pwd-dioxus --features icons` compila

---

## Task 3: Estrarre Modulo `spinner`

### 3.1 Copiare file

```
src/components/globals/spinner/component.rs → pwd-dioxus/src/spinner/component.rs
```

### 3.2 Creare mod.rs

```rust
// pwd-dioxus/src/spinner/mod.rs
mod component;
pub use component::{Spinner, SpinnerSize};
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features spinner` compila

---

## Task 4: Estrarre Modulo `modal`

### 4.1 Copiare file

```
src/components/globals/dialogs/base_modal.rs → pwd-dioxus/src/modal/component.rs
```

### 4.2 Creare mod.rs

```rust
// pwd-dioxus/src/modal/mod.rs
mod component;
pub use component::{BaseModal, ModalVariant};
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features modal` compila

---

## Task 5: Estrarre Modulo `form`

### 5.1 Struttura file

```
pwd-dioxus/src/form/
├── mod.rs
├── types.rs    # FormSecret, FormValue trait, InputType enum
└── field.rs    # FormField<T> component
```

### 5.2 types.rs

Copiare da `src/components/globals/form_field.rs` le righe 1-71:

```rust
// pwd-dioxus/src/form/types.rs
use dioxus::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use std::ops::Deref;

/// Wrapper per SecretString usato nei form
#[derive(Clone)]
pub struct FormSecret(pub SecretString);

impl PartialEq for FormSecret {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Deref for FormSecret {
    type Target = SecretString;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FormValue for FormSecret {
    fn to_form_string(&self) -> String {
        self.0.expose_secret().to_string()
    }
    fn from_form_string(s: String) -> Option<Self> {
        Some(FormSecret(SecretString::new(s.into())))
    }
}

/// Trait per valori usabili nei form
pub trait FormValue: Clone + PartialEq + 'static {
    fn to_form_string(&self) -> String;
    fn from_form_string(s: String) -> Option<Self>;
}

// Implementazioni standard
impl FormValue for String {
    fn to_form_string(&self) -> String { self.clone() }
    fn from_form_string(s: String) -> Option<Self> { Some(s) }
}

impl FormValue for i32 {
    fn to_form_string(&self) -> String { self.to_string() }
    fn from_form_string(s: String) -> Option<Self> { s.parse().ok() }
}

impl FormValue for Option<String> {
    fn to_form_string(&self) -> String { self.clone().unwrap_or_default() }
    fn from_form_string(s: String) -> Option<Self> {
        Some(if s.is_empty() { None } else { Some(s) })
    }
}

/// Tipo di input per FormField
#[derive(Clone, PartialEq, Debug)]
pub enum InputType {
    Text,
    Textarea,
    Password,
    Email,
    Number,
    Tel,
    Url,
}

impl InputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Textarea => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
    }

    pub fn is_textarea(&self) -> bool {
        matches!(self, InputType::Textarea)
    }
}
```

### 5.3 field.rs

Copiare da `src/components/globals/form_field.rs` le righe 108-311, con questi import:

```rust
// pwd-dioxus/src/form/field.rs
use super::types::{FormValue, InputType};
use crate::icons::{EyeIcon, EyeOffIcon};
use dioxus::prelude::*;

// ... resto del componente FormField (righe 108-311 originali)
```

### 5.4 mod.rs

```rust
// pwd-dioxus/src/form/mod.rs
mod types;
mod field;

pub use types::{FormSecret, FormValue, InputType};
pub use field::FormField;
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features form` compila

---

## Task 6: Estrarre Modulo `secret`

### 6.1 Copiare e aggiornare file

Copiare `src/components/globals/secret_display/component.rs` → `pwd-dioxus/src/secret/component.rs`

Aggiornare import:
```rust
// pwd-dioxus/src/secret/component.rs
use crate::form::FormSecret;
use crate::icons::{ClipboardIcon, EyeIcon, EyeOffIcon};
use dioxus::prelude::*;
use secrecy::ExposeSecret;

// ... resto del componente
```

### 6.2 Creare mod.rs

```rust
// pwd-dioxus/src/secret/mod.rs
mod component;
pub use component::SecretDisplay;
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features secret-display` compila

---

## Task 7: Estrarre Modulo `password`

### 7.1 Copiare file

```
src/components/globals/password_handler/strength_analyzer.rs → pwd-dioxus/src/password/analyzer.rs
src/components/globals/password_handler/component.rs → pwd-dioxus/src/password/handler.rs
```

### 7.2 analyzer.rs

Aggiornare import:
```rust
// pwd-dioxus/src/password/analyzer.rs
use crate::spinner::{Spinner, SpinnerSize};
use dioxus::prelude::*;
use pwd_types::{PasswordScore, PasswordStrength};

// ... resto del componente (invariato)
```

### 7.3 handler.rs - Disaccoppiare da DB

**Import completi:**
```rust
// pwd-dioxus/src/password/handler.rs
use crate::form::{FormField, FormSecret, InputType};
use crate::icons::MagicWandIcon;
use super::StrengthAnalyzer;
use dioxus::prelude::*;
use dioxus::core::Task;
use pwd_types::{PasswordChangeResult, PasswordScore, PasswordStrength};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

const DEBOUNCE_MS: u64 = 500;
```

**Nuovi props (disaccoppiati):**
```rust
#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    // Required - callback con risultato completo
    pub on_password_change: Callback<PasswordChangeResult>,

    // Behavior
    #[props(default = true)]
    pub password_required: bool,
    pub initial_password: Option<FormSecret>,
    pub initial_score: Option<PasswordScore>,

    // Password generation (delegato al consumer)
    /// Callback chiamato quando utente clicca "Suggest"
    #[props(default = None)]
    pub on_generate: Option<Callback<()>>,
    /// Signal letta dal consumer per passare la password generata
    #[props(default = None)]
    pub generated_password: Option<Signal<Option<FormSecret>>>,
    /// Stato loading per generazione
    #[props(default = None)]
    pub is_generating: Option<Signal<bool>>,

    // Customization
    #[props(default = "Password".to_string())]
    pub password_label: String,
    #[props(default = true)]
    pub show_strength_bar: bool,
    #[props(default = true)]
    pub show_suggest_button: bool,
}
```

**Logica suggest button disaccoppiata:**
```rust
// Nel componente, sostituire on_suggest_password con:
let suggest_onclick = move |_| {
    if let Some(on_gen) = &props.on_generate {
        on_gen.call(());
    }
};

// Nel bottone:
button {
    class: "btn btn-ghost btn-sm gap-2 tooltip",
    "data-tip": "suggest password",
    r#type: "button",
    onclick: suggest_onclick,
    disabled: props.is_generating.map_or(false, |g| *g.read()),
    // ... resto
}
```

**Gestione generated_password:**
```rust
// Aggiungere use_effect per sincronizzare generated_password
use_effect(move || {
    if let Some(gen_pwd) = &props.generated_password {
        if let Some(new_pwd) = gen_pwd.read().clone() {
            password.set(new_pwd.clone());
            repassword.set(new_pwd);
            // Trigger evaluation
            on_password_change_internal(new_pwd);
        }
    }
});
```

### 7.4 Creare mod.rs

```rust
// pwd-dioxus/src/password/mod.rs
mod analyzer;
mod handler;

pub use analyzer::StrengthAnalyzer;
pub use handler::PasswordHandler;
```

### Criterio di successo
- [ ] `cargo check -p pwd-dioxus --features handler` compila

---

## Task 8: Estrarre CSS

### 8.1 Creare assets/components.css

Estrarre da `assets/input_main.css` (verificare righe attuali):

```css
/* ============================================================
   PWD-DIOXUS COMPONENTS CSS
   ============================================================ */

/* === SPINNER (righe 497-560) === */
.spinner-sm {
    @apply w-6 h-6;
    --spinner-size: 24px;
    --spinner-inner: 18px;
}
.spinner-md {
    @apply w-10 h-10;
    --spinner-size: 40px;
    --spinner-inner: 30px;
}
.spinner-lg {
    @apply w-14 h-14;
    --spinner-size: 56px;
    --spinner-inner: 42px;
}
.spinner-xl {
    @apply w-20 h-20;
    --spinner-size: 80px;
    --spinner-inner: 60px;
}
.spinner-4xl {
    @apply w-50 h-50;
    --spinner-size: 200px;
    --spinner-inner: 150px;
}
.spinner {
    position: relative;
    height: var(--spinner-size);
    width: var(--spinner-size);
    box-sizing: border-box;
    background: conic-gradient(
        from 90deg at 50% 50%,
        rgba(39, 174, 96, 0) 0deg,
        rgba(31, 144, 255, 0) 0.04deg,
        currentColor 360deg
    );
    border-radius: 50%;
    animation: 1s rotate infinite linear;
}
/* ... copiare resto spinner dal file originale */

/* === FORM - Password Input (righe 773-798) === */
.password-input-wrapper {
    @apply relative flex items-center;
}
.password-input-with-toggle {
    @apply pr-12;
}
.password-visibility-toggle {
    @apply absolute right-3 p-1 rounded;
    @apply text-neutral-500 hover:text-neutral-700 hover:bg-neutral-100;
    @apply transition-colors duration-150;
    @apply focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-1;
    @apply disabled:opacity-50 disabled:cursor-not-allowed;
    @apply flex items-center justify-center;
    background: transparent;
    border: none;
    cursor: pointer;
}

/* === SECRET DISPLAY (righe 1224-1262) === */
.secret-display-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: fit-content;
}
.pwd-secret-display {
    background-color: transparent;
    border: none;
    outline: none;
    font-size: 1rem;
    cursor: text;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.pwd-display-action-btn {
    padding: 0.25rem;
    border-radius: 0.25rem;
    transition: background-color 150ms cubic-bezier(0.4, 0, 0.2, 1);
    color: inherit;
}
.pwd-display-action-btn:hover {
    background-color: var(--primary-color-3);
}
.pwd-display-action-btn:focus-visible {
    outline: 2px solid var(--focused-border-color);
    outline-offset: 2px;
}
.pwd-display-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
}

/* === PASSWORD - Strength Analyzer (righe 1082-1148) === */
.strength-analyzer {
    min-height: 24px;
}
.strength-info-btn {
    width: 20px;
    height: 20px;
    min-width: 20px;
    font-size: 0.75rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: theme('colors.base-300');
    color: theme('colors.base-content');
    border: none;
    cursor: pointer;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
}
.strength-info-btn:hover {
    transform: scale(1.2);
    box-shadow: 0 0 12px 3px rgba(59, 130, 246, 0.6);
}
.strength-reasons-tooltip {
    animation: fadeIn 0.2s ease-out;
}
.strength-bar-container {
    width: 100%;
    padding: 4px 0;
}
.strength-bar {
    width: 100%;
    height: 8px;
    border-radius: 4px;
    background: linear-gradient(
        to right,
        #ef4444 0%, #f97316 15%, #eab308 30%,
        #22c55e 50%, #3b82f6 70%, #8b5cf6 95%, #a855f7 100%
    );
    position: relative;
}
.strength-cursor {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 16px;
    height: 16px;
    background-color: white;
    border: 2px solid #374151;
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transition: left 0.3s ease-out;
}
```

### 8.2 Documentare integrazione consumer

```markdown
## CSS Integration

 Nel progetto consumer (es. PWDManager), il CSS è già in input_main.css.
 Per nuovi progetti, includere nel Tailwind build:

```css
@import "pwd-dioxus/assets/components.css";
```

**Nota:** Le classi `.form-group`, `.form-label`, `.pwd-input` usano Tailwind inline
e non sono definite come classi CSS standalone.
```

### Criterio di successo
- [ ] File `pwd-dioxus/assets/components.css` creato
- [ ] CSS copiato correttamente

---

## Task 9: Aggiornare Progetto Padre

### 9.1 Aggiungere dipendenza

In `PWDManager/Cargo.toml`:
```toml
pwd-dioxus = { path = "pwd-dioxus", features = ["handler"] }
```

### 9.2 Creare wrapper backward-compatible

Sostituire completamente `src/components/globals/password_handler/component.rs`:

```rust
// src/components/globals/password_handler/component.rs
//
// WRAPPER: Fornisce callback DB a pwd-dioxus::PasswordHandler
// Questo file mantiene backward compatibility con il resto del progetto

use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_passwords_generation_settings;
use crate::backend::password_utils::generate_suggested_password;
use crate::backend::evaluate_password_strength_tx;
use dioxus::prelude::*;
use pwd_dioxus::{PasswordHandler as LibPasswordHandler, FormSecret, InputType};
use pwd_types::{PasswordChangeResult, PasswordScore, PasswordStrength};
use secrecy::SecretString;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

const DEBOUNCE_MS: u64 = 500;

#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    /// Callback quando la password cambia (solo FormSecret per backward compat)
    pub on_password_change: Callback<FormSecret>,
    #[props(default = true)]
    pub password_required: bool,
    pub initial_password: Option<FormSecret>,
    pub initial_score: Option<PasswordScore>,
}

#[component]
pub fn PasswordHandler(props: PasswordHandlerProps) -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();

    // State per generazione password
    let mut generated_pwd = use_signal(|| None::<Option<FormSecret>>);
    let mut is_generating = use_signal(|| false);

    // Callback per generazione password (chiama DB)
    let auth_for_gen = auth_state.clone();
    let on_generate = use_callback(move |_| {
        let pool = pool.clone();
        let auth = auth_for_gen.clone();
        let mut is_gen = is_generating.clone();
        let mut gen_pwd = generated_pwd.clone();

        spawn(async move {
            is_gen.set(true);

            let config = if let Some(user) = auth.get_user() {
                fetch_user_passwords_generation_settings(&pool, user.id).await.ok()
            } else {
                None
            };

            let pwd = generate_suggested_password(config);
            gen_pwd.set(Some(Some(FormSecret(pwd)))));

            is_gen.set(false);
        });
    });

    // Callback per cambiamento password (converte PasswordChangeResult → FormSecret)
    let on_change = use_callback(move |result: PasswordChangeResult| {
        props.on_password_change.call(FormSecret(result.password));
    });

    rsx! {
        LibPasswordHandler {
            on_password_change: on_change,
            password_required: props.password_required,
            initial_password: props.initial_password,
            initial_score: props.initial_score,
            on_generate: Some(on_generate),
            generated_password: Some(generated_pwd),
            is_generating: Some(is_generating),
            password_label: "Password".to_string(),
            show_strength_bar: true,
            show_suggest_button: true,
        }
    }
}
```

### 9.3 Aggiornare strength_analyzer import

In `src/components/globals/password_handler/mod.rs`:
```rust
mod component;
// Rimuovere: mod strength_analyzer;  (ora viene da pwd-dioxus)

pub use component::PasswordHandler;
pub use pwd_dioxus::StrengthAnalyzer;  // Re-export dalla libreria
```

### Criterio di successo
- [ ] `cargo check --workspace` compila
- [ ] PasswordHandler esistente continua a funzionare

---

## Task 10: Verifica e Commit

### 10.1 Verifica finale

```bash
cargo test --workspace --lib
cargo check --workspace --all-features
```

### 10.2 Commit

```bash
git add pwd-dioxus/
git add pwd-types/src/change_result.rs pwd-types/src/lib.rs
git add Cargo.toml
git add src/components/globals/password_handler/
git commit -m "feat: extract pwd-dioxus library with atomic modules

- Add pwd-dioxus crate with 6 atomic modules (icons, spinner, modal, form, secret, password)
- Add PasswordChangeResult to pwd-types for callback API
- Disassociate PasswordHandler from DB via callback props
- Extract CSS to pwd-dioxus/assets/components.css
- Add backward-compatible wrapper in PWDManager

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

### 10.3 Aggiornare orchestrator

Cambiare stato Step 4 da ⏳ a ✅ e aggiungere data.

---

## Checklist Finale

- [ ] Task 1: Struttura crate creata
- [ ] Task 2: Modulo icons funzionante
- [ ] Task 3: Modulo spinner funzionante
- [ ] Task 4: Modulo modal funzionante
- [ ] Task 5: Modulo form funzionante
- [ ] Task 6: Modulo secret funzionante
- [ ] Task 7: Modulo password funzionante (disaccoppiato)
- [ ] Task 8: CSS estratto
- [ ] Task 9: Progetto padre aggiornato
- [ ] Task 10: Verifica e commit

---

## Troubleshooting

### Problema: Feature flag non riconosciuta

Verificare che:
1. La feature sia dichiarata in `[features]`
2. I moduli siano gated con `#[cfg(any(feature = "...", feature = "..."))]`
3. In lib.rs, il modulo sia incluso con la feature corretta

### Problema: Icone non trovate

Verificare che `use crate::icons::...` punti al crate pwd-dioxus.
Nei test, assicurarsi che la feature corretta sia abilitata.

### Problema: FormSecret mismatch

- `pwd_dioxus::FormSecret` è il tipo della libreria
- Nel wrapper, usare `use pwd_dioxus::FormSecret;` per evitare conflitti
- Per convertire da `SecretString`: `FormSecret(SecretString::new(s.into()))`

---

## Note per l'Implementazione

1. **Ordine di estrazione:** Seguire l'ordine dei task. Ogni task dipende dai precedenti.

2. **Test incrementali:** Dopo ogni task, verificare con `cargo check -p pwd-dioxus --features <feature>`.

3. **CSS:** Le righe indicate possono slittare se input_main.css è stato modificato.
   Verificare sempre prima di copiare.

4. **Wrapper:** Il wrapper in Task 9 è la parte più complessa. Testare attentamente
   che la UI funzioni come prima.
