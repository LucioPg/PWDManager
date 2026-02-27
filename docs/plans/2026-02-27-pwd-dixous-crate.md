# Piano: Estrazione `pwd-dioxus` Crate

> ⚠️ **IMPORTANTE:** Prima di procedere con l'implementazione, leggere il documento
> [`2026-02-27-pwd-dixous-crate-issues.md`](./2026-02-27-pwd-dixous-crate-issues.md)
> che elenca le anomalie identificate e le correzioni necessarie.

---

## Context

**Obiettivo:** Estrarre i componenti UI Dioxus per la gestione password in una libreria riutilizzabile.

**Motivazione:** L'utente vuole riutilizzare `PasswordHandler` in un altro progetto Dioxus. Attualmente il componente è accoppiato al database tramite `use_context`, rendendolo non riutilizzabile.

**Sfida principale:** Rimuovere l'accoppiamento database (righe 35-36, 168-198 di `component.rs`) sostituendolo con callback props.

---

## Componenti da Estrarre

| Componente | File | Dipendenze |
|------------|------|------------|
| `PasswordHandler` | `src/components/globals/password_handler/component.rs` | pwd-types, pwd-strength, database |
| `StrengthAnalyzer` | `src/components/globals/password_handler/strength_analyzer.rs` | pwd-types, Spinner |
| `FormField<T>` | `src/components/globals/form_field.rs` | FormSecret, icons |
| `Spinner` | `src/components/globals/spinner/component.rs` | Nessuna |
| Icons | `src/components/globals/svgs/` | EyeIcon, EyeOffIcon, MagicWandIcon |

---

## Struttura Crate

```
pwd-dioxus/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── password_handler/
│   │   │   ├── mod.rs
│   │   │   ├── component.rs
│   │   │   └── strength_analyzer.rs
│   │   ├── form_field/
│   │   │   ├── mod.rs
│   │   │   └── component.rs
│   │   └── spinner/
│   │       ├── mod.rs
│   │       └── component.rs
│   └── icons/
│       ├── mod.rs
│       ├── base.rs
│       └── visibility.rs
└── assets/
    └── components.css
```

---

## Feature Flags

```toml
[features]
default = ["handler"]
handler = ["dep:dioxus", "dep:pwd-types", "dep:pwd-strength"]
analyzer = ["dep:dioxus", "dep:pwd-types"]
form-field = ["dep:dioxus", "dep:secrecy"]
spinner = ["dep:dioxus"]
icons = ["dep:dioxus"]
full = ["handler", "analyzer", "form-field", "spinner", "icons"]
```

---

## API: PasswordHandler Props (Riprogettata)

```rust
#[derive(Props, Clone, PartialEq)]
pub struct PasswordHandlerProps {
    // Required
    pub on_password_change: Callback<PasswordChangeResult>,

    // Behavior
    #[props(default = true)]
    pub password_required: bool,
    pub initial_password: Option<FormSecret>,
    pub initial_score: Option<PasswordScore>,
    #[props(default = true)]
    pub show_suggest_button: bool,

    // NUOVO: Callback per generazione password (parent gestisce DB)
    #[props(default = None)]
    pub on_generate_password: Option<Callback<()>>,
    #[props(default = None)]
    pub generated_password: Option<Signal<Option<FormSecret>>>,

    // Customization
    #[props(default = "Password".to_string())]
    pub password_label: String,
    #[props(default = true)]
    pub show_strength_bar: bool,
}

pub struct PasswordChangeResult {
    pub password: FormSecret,
    pub score: Option<PasswordScore>,
    pub strength: PasswordStrength,
    pub reasons: Vec<String>,
}
```

---

## Step di Implementazione

### Step 1: Creare Struttura Crate
1. Creare `pwd-dioxus/` directory
2. Creare `Cargo.toml` con feature flags
3. Aggiungere a `[workspace.members]` in root `Cargo.toml`
4. Creare file `mod.rs` vuoti

### Step 2: Estrarre Spinner
1. Copiare `spinner/component.rs` → `components/spinner/component.rs`
2. Verificare: `cargo check -p pwd-dioxus`

### Step 3: Estrarre Icons
1. Copiare `base_icon.rs` → `icons/base.rs`
2. Copiare `visibility_icons.rs` → `icons/visibility.rs`
3. Aggiungere `MagicWandIcon` da `action_icons.rs`

### Step 4: Estrarre FormField
1. Copiare `form_field.rs` → `components/form_field/component.rs`
2. Includere `FormSecret`, `InputType`, `FormValue` trait
3. Aggiornare import icons

### Step 5: Estrarre StrengthAnalyzer
1. Copiare `strength_analyzer.rs` → `components/password_handler/strength_analyzer.rs`
2. Aggiornare import Spinner e pwd-types

### Step 6: Estrarre PasswordHandler (Critico)
1. Copiare `component.rs` → `components/password_handler/component.rs`
2. **Rimuovere accoppiamento database:**
   - Eliminare righe 1-2, 35-36 (imports e use_context)
   - Eliminare righe 168-198 (`on_suggest_password` con DB)
   - Aggiungere nuovi props: `on_generate_password`, `generated_password`
3. Aggiornare callback suggerimento per usare props esterni

### Step 7: Estrarre CSS
1. Creare `assets/components.css`
2. Copiare da `input_main.css`:
   - Righe 773-792: password input classes
   - Righe 1078-1148: strength analyzer classes

### Step 8: Aggiornare Progetto Padre
1. Aggiungere `pwd-dioxus` a `PWDManager/Cargo.toml`
2. Creare wrapper in `src/components/globals/password_handler/` che:
   - Usa `pwd_dioxus::PasswordHandler`
   - Fornisce callback per generazione password (con DB access)
3. Mantenere backward compatibility

### Step 9: Test e Verifica
```bash
cargo test --workspace --all-features
cargo check --workspace --all-features
```

---

## CSS da Estrarre

**Da `input_main.css`:**

| Righe | Classi |
|-------|--------|
| 773-792 | `.password-input-wrapper`, `.password-input-with-toggle`, `.password-visibility-toggle` |
| 1078-1148 | `.strength-analyzer`, `.strength-info-btn`, `.strength-reasons-tooltip`, `.strength-bar-container`, `.strength-bar`, `.strength-cursor` |

---

## File Critici

1. `src/components/globals/password_handler/component.rs` - Logica principale da disaccoppiare
2. `src/components/globals/form_field.rs` - FormSecret, InputType, FormField
3. `assets/input_main.css` (righe 773-792, 1078-1148) - CSS da estrarre
4. `src/components/globals/svgs/visibility_icons.rs` - Icone visibilità
5. `pwd-strength/Cargo.toml` - Pattern da seguire

---

## Verifica Finale

```bash
# Verificare compilazione
cargo check -p pwd-dioxus --all-features

# Verificare integrazione
cargo check --workspace --all-features

# Eseguire test
cargo test --workspace --all-features
```

---

## Note

- Il wrapper nel progetto padre mantiene backward compatibility
- I consumer devono fornire il proprio callback `on_generate_password`
- Il CSS può essere incluso nel Tailwind build del consumer
