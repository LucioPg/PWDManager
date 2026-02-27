# Analisi Componenti Estraibili: `pwd-dioxus`

> **Creato:** 2026-02-27
> **Aggiornato:** 2026-02-27
> **Scopo:** Mappatura completa dei componenti in `src/components/globals/` per decidere cosa estrarre

> 📐 **Vedi anche:** [`2026-02-27-pwd-dioxus-architecture.md`](./2026-02-27-pwd-dioxus-architecture.md) per l'architettura atomica definitiva.

---

## Panoramica Struttura `src/components/globals/`

```
src/components/globals/
├── mod.rs                    # Re-exports di tutti i componenti
├── action_buttons.rs         # ActionButton, ActionButtons (DaisyUI btn wrapper)
├── auth_wrapper.rs           # Route protection (app-specific)
├── avatar_selector.rs        # Avatar upload/selection (app-specific)
├── form_field.rs             # FormField<T>, FormSecret, InputType, FormValue
├── navbar.rs                 # Navigation bar (app-specific)
├── pagenotfound.rs           # 404 page (app-specific)
├── route_wrapper.rs          # Background wrapper (app-specific)
├── stat_card.rs              # StatCard (dashboard stat)
├── style.rs                  # Style utilities
├── toast_hub.rs              # Toast notifications system
├── dialogs/
│   ├── mod.rs
│   ├── base_modal.rs         # BaseModal (DaisyUI modal wrapper)
│   ├── user_deletion.rs      # User deletion dialog (app-specific)
│   └── stored_password_upsert.rs  # Password upsert dialog (app-specific)
├── password_handler/
│   ├── mod.rs
│   ├── component.rs          # PasswordHandler (main component)
│   └── strength_analyzer.rs  # StrengthAnalyzer (strength indicator)
├── secret_display/
│   ├── mod.rs
│   └── component.rs          # SecretDisplay (masked value display)
├── secret_notes_tooltip/
│   ├── mod.rs
│   └── component.rs          # SecretNotesTooltip
├── spinner/
│   ├── mod.rs
│   └── component.rs          # Spinner, SpinnerSize
├── svgs/
│   ├── mod.rs
│   ├── base_icon.rs          # SvgIcon (base component)
│   ├── visibility_icons.rs   # EyeIcon, EyeOffIcon
│   ├── action_icons.rs       # BurgerIcon, EditIcon, DeleteIcon, ClipboardIcon, MagicWandIcon
│   └── alert_icons.rs        # WarningIcon, LogoutIcon
├── table/
│   ├── mod.rs
│   ├── table.rs              # StoredRawPasswordsTable (app-specific)
│   └── table_row.rs          # StoredRawPasswordRow (app-specific)
└── tabs/
    ├── mod.rs
    └── component.rs          # Tabs, TabList, TabTrigger, TabContent (dioxus-primitives wrapper)
```

---

## Matrice Dipendenze Componenti

### Legenda
- `✓` = Dipendenza diretta
- `○` = Dipendenza indiretta (tramite altro componente)
- `DB` = Accoppiamento database (da rimuovere per estrazione)

### Componenti Core (Riutilizzabili)

| Componente | Spinner | Icons | FormField | pwd-types | secrecy | dioxus-primitives | DB | App-specific |
|------------|:-------:|:-----:|:---------:|:---------:|:-------:|:-----------------:|:--:|:------------:|
| **Spinner** | - | - | - | - | - | - | ✗ | ✗ |
| **SvgIcon** | - | - | - | - | - | - | ✗ | ✗ |
| **EyeIcon** | - | ✓ | - | - | - | - | ✗ | ✗ |
| **EyeOffIcon** | - | ✓ | - | - | - | - | ✗ | ✗ |
| **MagicWandIcon** | - | - | - | - | - | - | ✗ | ✗ |
| **FormField** | - | ✓ | - | - | ✓ | - | ✗ | ✗ |
| **StrengthAnalyzer** | ✓ | - | - | ✓ | - | - | ✗ | ✗ |
| **PasswordHandler** | ○ | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✗ |
| **SecretDisplay** | - | ✓ | - | - | ✓ | - | ✗ | ✗ |
| **BaseModal** | - | - | - | - | - | - | ✗ | ✗ |

### Componenti App-Specifici (Non Estraibili)

| Componente | Motivo |
|------------|--------|
| auth_wrapper.rs | Dipende da AuthState, routing |
| avatar_selector.rs | Dipende da DB, filesystem |
| navbar.rs | Link a Route, AuthState |
| pagenotfound.rs | Link a Route |
| route_wrapper.rs | AuthState, Route |
| stat_card.rs | Usa pwd-types ma è dashboard-specific |
| toast_hub.rs | Gestione stato globale app |
| dialogs/user_deletion.rs | DB operations |
| dialogs/stored_password_upsert.rs | DB operations |
| table/* | StoredRawPassword, DB |

---

## Dettaglio Dipendenze per Componente

### 1. Spinner

```
File: spinner/component.rs
Dependencies:
  - dioxus (prelude)
  - Nessuna dipendenza interna

Types esportati:
  - SpinnerSize (enum: Small, Medium, Large, XLarge, XXXXLarge)
  - Spinner (component)

CSS richiesto:
  - .spinner
  - .spinner-sm, .spinner-md, .spinner-lg, .spinner-xl, .spinner-4xl
```

### 2. Icons (svgs/)

```
File: svgs/base_icon.rs
Dependencies:
  - dioxus (prelude)

Types esportati:
  - SvgIcon (component base)

File: svgs/visibility_icons.rs
Dependencies:
  - dioxus (prelude)
  - super::base_icon::SvgIcon

Types esportati:
  - EyeIcon
  - EyeOffIcon

File: svgs/action_icons.rs
Dependencies:
  - dioxus (prelude)
  - super::base_icon::SvgIcon

Types esportati:
  - BurgerIcon
  - EditIcon
  - DeleteIcon
  - ClipboardIcon
  - MagicWandIcon

File: svgs/alert_icons.rs
Dependencies:
  - dioxus (prelude)
  - super::base_icon::SvgIcon

Types esportati:
  - WarningIcon
  - LogoutIcon
```

### 3. FormField

```
File: form_field.rs
Dependencies:
  - dioxus (prelude)
  - secrecy::{ExposeSecret, SecretString}
  - super::svgs::{EyeIcon, EyeOffIcon}

Types esportati:
  - FormSecret (struct wrapper SecretString)
  - FormValue (trait)
  - InputType (enum: Text, Textarea, Password, Email, Number, Tel, Url)
  - FormField<T: FormValue> (component)

Implementazioni FormValue:
  - String
  - i32
  - Option<String>
  - FormSecret

CSS richiesto:
  - .form-group
  - .form-label
  - .pwd-input
  - .password-input-wrapper
  - .password-input-with-toggle
  - .password-visibility-toggle
```

### 4. StrengthAnalyzer

```
File: password_handler/strength_analyzer.rs
Dependencies:
  - dioxus (prelude)
  - pwd_types::{PasswordScore, PasswordStrength}
  - crate::components::globals::spinner::{Spinner, SpinnerSize}

Types esportati:
  - StrengthAnalyzerProps (strength, reasons, is_evaluating, score, show_bar)
  - StrengthAnalyzer (component)

CSS richiesto:
  - .strength-analyzer
  - .strength-info-btn
  - .strength-reasons-tooltip
  - .strength-bar-container
  - .strength-bar
  - .strength-cursor
```

### 5. PasswordHandler

```
File: password_handler/component.rs
Dependencies:
  - dioxus (prelude)
  - secrecy::{ExposeSecret, SecretString}
  - pwd_types::{PasswordScore, PasswordStrength}
  - sqlx::SqlitePool ⚠️ DB DEPENDENCY
  - crate::auth::AuthState ⚠️ APP-SPECIFIC
  - crate::backend::* ⚠️ APP-SPECIFIC
  - crate::components::globals::form_field::{FormSecret, InputType}
  - crate::components::globals::svgs::MagicWandIcon
  - tokio::sync::mpsc
  - tokio_util::sync::CancellationToken

Types esportati:
  - PasswordHandlerProps
  - PasswordHandler (component)

DA RIMUOVERE per estrazione:
  - use_context::<AuthState>()
  - use_context::<SqlitePool>()
  - fetch_user_passwords_generation_settings()
  - generate_suggested_password()

DA SOSTITUIRE CON:
  - on_generate_password: Option<Callback<()>>
  - generated_password: Option<Signal<Option<FormSecret>>>

CSS richiesto:
  - .password-handler
  - (tutto quello di FormField e StrengthAnalyzer)
```

### 6. SecretDisplay

```
File: secret_display/component.rs
Dependencies:
  - dioxus (prelude)
  - secrecy::ExposeSecret
  - crate::components::globals::svgs::{ClipboardIcon, EyeIcon, EyeOffIcon}
  - crate::components::globals::form_field::FormSecret

Types esportati:
  - SecretDisplay (component)

CSS richiesto:
  - .secret-display-wrapper
  - .pwd-secret-display
  - .secret-display-actions
  - .pwd-display-action-btn
```

### 7. BaseModal

```
File: dialogs/base_modal.rs
Dependencies:
  - dioxus (prelude)
  - Nessuna dipendenza interna

Types esportati:
  - ModalVariant (enum: Middle, Top, Bottom)
  - BaseModal (component)

CSS richiesto:
  - .modal (DaisyUI)
  - .modal-open (DaisyUI)
  - .modal-middle, .modal-top, .modal-bottom (DaisyUI)
  - .modal-backdrop (DaisyUI)
  - .modal-box (DaisyUI)
```

---

## CSS Completo da Estrarre

### Da `input_main.css`

| Righe | Classi | Usato da |
|-------|--------|----------|
| 497-539 | `.spinner-*` | Spinner |
| 773-798 | `.password-input-*`, `.password-visibility-toggle` | FormField |
| 1082-1148 | `.strength-*` | StrengthAnalyzer |
| 1224-1260 | `.secret-display-*`, `.pwd-secret-*` | SecretDisplay |

### Classi da verificare/aggiungere

```css
/* FormField - non presenti come blocchi separati */
.form-group { }
.form-label { }
.pwd-input { }

/* PasswordHandler */
.password-handler { }
```

---

## Proposta di Estrazione

### Tier 1: Componenti Base (nessuna dipendenza pwd-types)

```
pwd-dioxus-base/
├── Spinner + SpinnerSize
├── Icons (tutti i file svgs/)
└── BaseModal + ModalVariant
```

### Tier 2: Componenti Form (dipendono da Tier 1 + secrecy)

```
pwd-dioxus-form/
├── FormField<T> + FormSecret + InputType + FormValue
└── SecretDisplay
```

### Tier 3: Componenti Password (dipendono da Tier 1-2 + pwd-types + pwd-strength)

```
pwd-dioxus-password/
├── StrengthAnalyzer
└── PasswordHandler (disaccoppiato da DB)
```

---

## Feature Flags Proposte (Riviste)

```toml
[features]
default = ["handler"]

# Tier 1 - Base
spinner = []
icons = []
modal = []
base = ["spinner", "icons", "modal"]

# Tier 2 - Form
form-field = ["icons", "dep:secrecy"]
secret-display = ["form-field"]

# Tier 3 - Password
analyzer = ["spinner", "dep:pwd-types"]
handler = ["form-field", "analyzer", "dep:secrecy"]

# Convenience
full = ["base", "secret-display", "handler"]
```

---

## Dipendenze Crate Esterne

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false }
secrecy = { version = "0.10", optional = true }
pwd-types = { path = "../pwd-types", optional = true }
```

**Nota:** `pwd-strength` non è necessario nel crate estratto perché:
- La valutazione avviene nel consumer
- `PasswordHandler` chiama `evaluate_password_strength_tx` che è app-specific

---

## Decisioni Pending

1. **SecretDisplay**: Includere in `pwd-dioxus` o lasciare nel progetto padre?
   - È generico ma dipende da FormSecret

2. **BaseModal**: Includere?
   - È un wrapper sottile di DaisyUI, potrebbe essere overkill estrarlo

3. **MagicWandIcon**: È l'unica icona "action" usata da PasswordHandler
   - Estrarre solo questa o tutto `action_icons.rs`?

4. **FormValue implementations**: Per `String`, `i32`, `Option<String>`
   - Includerle tutte o solo `FormSecret`?

---

## Prossimi Passi

1. [ ] Decidere quali Tier includere
2. [ ] Decidere su SecretDisplay e BaseModal
3. [ ] Definire API disaccoppiata per PasswordHandler
4. [ ] Identificare CSS esatto da estrarre
5. [ ] Aggiornare piano principale
