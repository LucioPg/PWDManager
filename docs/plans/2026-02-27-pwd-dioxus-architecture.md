# Architettura Atomica: `pwd-dioxus`

> **Creato:** 2026-02-27
> **Scopo:** Definire struttura modulare atomica per evitare duplicazioni e dipendenze circolari
> **Prerequisiti:** Steps 1-3 completati (pwd-types, pwd-strength, pwd-crypto)

---

## Principio Guida

**Ogni modulo è atomico e può essere usato indipendentemente.**

Le feature flags permettono di importare solo ciò che serve:
```toml
# Solo spinner
pwd-dioxus = { version = "0.1", features = ["spinner"] }

# Password handler completo
pwd-dioxus = { version = "0.1", features = ["handler"] }
```

---

## Struttura Crate

```
pwd-dioxus/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Re-exports + feature gating
│   │
│   ├── icons/                    # MODULO ATOMICO: Solo icone SVG
│   │   ├── mod.rs
│   │   ├── base.rs               # SvgIcon (componente base)
│   │   ├── visibility.rs         # EyeIcon, EyeOffIcon
│   │   ├── action.rs             # MagicWandIcon, ClipboardIcon, DeleteIcon, EditIcon, BurgerIcon
│   │   └── alert.rs              # WarningIcon, LogoutIcon
│   │
│   ├── spinner/                  # MODULO ATOMICO: Loading indicator
│   │   ├── mod.rs
│   │   └── component.rs          # Spinner, SpinnerSize
│   │
│   ├── modal/                    # MODULO ATOMICO: Dialog wrapper
│   │   ├── mod.rs
│   │   └── component.rs          # BaseModal, ModalVariant
│   │
│   ├── form/                     # MODULO: Form components
│   │   ├── mod.rs
│   │   ├── types.rs              # FormSecret, FormValue trait, InputType
│   │   └── field.rs              # FormField<T>
│   │
│   ├── secret/                   # MODULO: Secret display (dipende da form)
│   │   ├── mod.rs
│   │   └── component.rs          # SecretDisplay
│   │
│   └── password/                 # MODULO: Password handling (dipende da pwd-types)
│       ├── mod.rs
│       ├── analyzer.rs           # StrengthAnalyzer
│       └── handler.rs            # PasswordHandler
│
└── assets/
    └── components.css            # CSS estratto
```

---

## Dipendenze tra Moduli

```
┌─────────────────────────────────────────────────────────────────┐
│                        pwd-dioxus                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ATOMICI (nessuna dipendenza interna)                           │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                         │
│  │  icons  │  │ spinner │  │  modal  │                         │
│  └────┬────┘  └────┬────┘  └─────────┘                         │
│       │            │                                             │
│       ▼            │                                             │
│  ┌─────────┐       │                                             │
│  │  form   │◄──────┘  (usa icons per visibility toggle)         │
│  └────┬────┘                                                     │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────┐                                                     │
│  │ secret  │  (usa form per FormSecret, icons per Eye/Clipboard)│
│  └────┬────┘                                                     │
│       │                                                          │
│       ▼                                                          │
│  ┌──────────┐                                                    │
│  │ password │  (usa form, spinner, icons, pwd-types)            │
│  └──────────┘                                                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   pwd-types     │  (esterno)
                    └─────────────────┘
```

---

## Feature Flags

```toml
[features]
default = ["spinner", "icons-visibility"]

# === ATOMICI ===
icons = []                                    # Tutte le icone
icons-visibility = []                         # Solo EyeIcon, EyeOffIcon
icons-action = []                             # MagicWand, Clipboard, etc.
icons-alert = []                              # Warning, Logout
spinner = []                                  # Spinner + SpinnerSize
modal = []                                    # BaseModal + ModalVariant

# === COMPOSITI ===
form = ["icons-visibility", "dep:secrecy"]    # FormField + types
secret-display = ["form", "icons-visibility", "icons-action"]
analyzer = ["spinner", "dep:pwd-types"]       # StrengthAnalyzer
handler = ["form", "analyzer", "icons-action", "dep:secrecy"]

# === CONVENIENCE ===
full = ["icons", "spinner", "modal", "handler"]
```

---

## Risoluzione Decisioni Pending

### 1. SecretDisplay ✅ Incluso

**Decisione:** Includere in modulo `secret/`

**Motivazione:**
- È generico (mostra qualsiasi valore segreto)
- Dipende solo da FormSecret (gia nel crate)
- Riutilizzabile per password, API keys, tokens

```rust
// pwd-dioxus/src/secret/component.rs
pub fn SecretDisplay(
    secret: FormSecret,        // Dal modulo form
    max_width: String,
    on_copy: Option<EventHandler<()>>,
) -> Element
```

### 2. BaseModal ✅ Incluso

**Decisione:** Includere in modulo `modal/`

**Motivazione:**
- Wrapper sottile ma standardizza API
- Utile per chi vuole modals consistenti
- Feature flag separata permette di escluderlo

```rust
// pwd-dioxus/src/modal/component.rs
pub fn BaseModal(
    open: Signal<bool>,
    variant: ModalVariant,
    on_close: EventHandler<()>,
    children: Element,
) -> Element
```

### 3. Icons ✅ Tutte (con granularità)

**Decisione:** Includere tutte con feature flags granulari

**Motivazione:**
- `icons-visibility` per chi usa solo password fields
- `icons-action` per chi vuole clipboard, delete, etc.
- `icons` include tutto

### 4. FormValue Implementations ✅ Tutte

**Decisione:** Includere tutte le implementazioni standard

```rust
// pwd-dioxus/src/form/types.rs
impl FormValue for String { ... }
impl FormValue for i32 { ... }
impl FormValue for Option<String> { ... }
impl FormValue for FormSecret { ... }
```

**Motivazione:** Servono per FormField generico, non ha senso frammentarle.

---

## CSS da Estrarre

### File: `assets/components.css`

```css
/* === SPINNER === */
.spinner { ... }
.spinner-sm { ... }
.spinner-md { ... }
.spinner-lg { ... }
.spinner-xl { ... }
.spinner-4xl { ... }

/* === FORM === */
.form-group { ... }
.form-label { ... }
.pwd-input { ... }
.password-input-wrapper { ... }
.password-input-with-toggle { ... }
.password-visibility-toggle { ... }

/* === SECRET DISPLAY === */
.secret-display-wrapper { ... }
.pwd-secret-display { ... }
.secret-display-actions { ... }
.pwd-display-action-btn { ... }

/* === PASSWORD === */
.password-handler { ... }
.strength-analyzer { ... }
.strength-info-btn { ... }
.strength-reasons-tooltip { ... }
.strength-bar-container { ... }
.strength-bar { ... }
.strength-cursor { ... }
```

### Integrazione Consumer

Il consumer deve includere il CSS nel proprio Tailwind build:

```css
/* Nel progetto consumer */
@import "pwd-dioxus/assets/components.css";
```

---

## API Disaccoppiata: PasswordHandler

### Props Attuali (accoppiate)

```rust
// ❌ Accoppiato a DB
let auth_state = use_context::<AuthState>();
let pool = use_context::<SqlitePool>();
let suggested = generate_suggested_password(config);  // Chiama DB
```

### Props Nuove (disaccoppiate)

```rust
pub struct PasswordHandlerProps {
    // Required - callback per notificare cambiamenti
    pub on_password_change: Callback<PasswordChangeResult>,

    // Behavior
    pub password_required: bool,
    pub initial_password: Option<FormSecret>,
    pub initial_score: Option<PasswordScore>,

    // NUOVO: Password generation delegata al consumer
    pub on_generate: Option<Callback<()>>,              // Trigger generation
    pub generated_password: Option<Signal<Option<FormSecret>>>,  // Result dal parent
    pub is_generating: Option<Signal<bool>>,            // Loading state

    // Customization
    pub password_label: String,
    pub show_strength_bar: bool,
    pub show_suggest_button: bool,
}

pub struct PasswordChangeResult {
    pub password: FormSecret,
    pub score: Option<PasswordScore>,
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}
```

### Esempio Consumer (Progetto Padre)

```rust
// Nel progetto PWDManager
fn MyForm() -> Element {
    let mut generated_pwd = use_signal(|| None);
    let mut is_generating = use_signal(|| false);
    let pool = use_context::<SqlitePool>();
    let auth = use_context::<AuthState>();

    let on_generate = use_callback(move |_| {
        let pool = pool.clone();
        let auth = auth.clone();
        spawn(async move {
            is_generating.set(true);
            let config = fetch_config(&pool, auth.get_user().id).await;
            let pwd = generate_suggested_password(config);
            generated_pwd.set(Some(FormSecret(pwd)));
            is_generating.set(false);
        });
    });

    rsx! {
        PasswordHandler {
            on_password_change: |result| { /* ... */ },
            on_generate: Some(on_generate),
            generated_password: Some(generated_pwd),
            is_generating: Some(is_generating),
        }
    }
}
```

---

## Modifiche a Crate Esistenti

### pwd-types: Aggiungere FormSecret?

**NO** - FormSecret è UI-specific (usa secrecy + dioxus Signal).
Mantenere in pwd-dioxus/form.

### pwd-types: Aggiungere PasswordChangeResult?

**SÌ** - È un tipo di dato puro che può essere utile anche altrove.

```rust
// pwd-types/src/form.rs (o nuovo modulo)
#[cfg(feature = "secrecy")]
use secrecy::SecretString;

#[derive(Clone)]
pub struct PasswordChangeResult {
    pub password: SecretString,
    pub score: Option<PasswordScore>,
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}
```

**Nota:** Questo richiede una modifica a pwd-types, da pianificare prima di Step 4.

---

## Dipendenze Cargo

```toml
# pwd-dioxus/Cargo.toml
[package]
name = "pwd-dioxus"
version = "0.1.0"
edition = "2024"

[dependencies]
dioxus = { version = "0.7", default-features = false, optional = true }
secrecy = { version = "0.10", optional = true }
pwd-types = { path = "../pwd-types", optional = true }

[features]
default = ["spinner", "icons-visibility"]
# ... (come sopra)

[package.metadata.docs.rs]
all-features = true
```

---

## Checklist Pre-Step 4

Prima di iniziare l'estrazione, verificare:

- [ ] Step F (Finalizzazione) completato o deciso di procedere in parallelo
- [ ] Deciso se aggiungere `PasswordChangeResult` a pwd-types
- [ ] Piano dedicato creato: `docs/plans/2026-02-27-extract-pwd-dioxus.md`
- [ ] Analisi CSS completata con righe esatte

---

## Prossimi Passi

1. **Decidere su PasswordChangeResult** in pwd-types
2. **Completare Step F** o procedere in parallelo
3. **Creare piano dedicato** per Step 4
4. **Implementare per moduli** (icons → spinner → modal → form → secret → password)

---

## Changelog

| Data | Versione | Modifica |
|------|----------|----------|
| 2026-02-27 | 1.0 | Architettura atomica iniziale |
