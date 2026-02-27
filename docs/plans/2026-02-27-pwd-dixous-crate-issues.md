# Problemi e Correzioni: Piano `pwd-dioxus`

> **Creato:** 2026-02-27
> **Aggiornato:** 2026-02-27
> **Stato:** Analisi completata

> 📋 **Vedi anche:** [`2026-02-27-pwd-dixous-crate-analysis.md`](./2026-02-27-pwd-dixous-crate-analysis.md) per la mappatura completa dei componenti.

Questo documento elenca le anomalie identificate nel piano `2026-02-27-pwd-dixous-crate.md` che devono essere corrette prima di procedere con l'estrazione.

---

## 1. Dipendenze Implicite Non Documentate

### 1.1 Moduli da Estrarre Completi

Il piano menziona solo superficialmente alcuni componenti che in realtà hanno struttura complessa:

| Componente | File Attuali | Struttura |
|------------|--------------|-----------|
| **Spinner** | `src/components/globals/spinner/` | `mod.rs` + `component.rs` (definisce `SpinnerSize`) |
| **FormField** | `src/components/globals/form_field.rs` | Singolo file ma include `FormSecret`, `InputType`, `FormValue` trait |
| **Icons** | `src/components/globals/svgs/` | 5 file: `mod.rs`, `base_icon.rs`, `visibility_icons.rs`, `action_icons.rs`, `alert_icons.rs` |

### 1.2 Dipendenze Trasversali

```
PasswordHandler
    ├── FormField<T> ──────┬── FormSecret (richiede secrecy)
    │                      ├── InputType
    │                      └── FormValue trait
    ├── StrengthAnalyzer ──┼── Spinner (richiede SpinnerSize)
    │                      └── pwd-types
    ├── Icons ─────────────┴── EyeIcon, EyeOffIcon, MagicWandIcon
    └── pwd-types ─────────── PasswordScore, PasswordStrength
```

---

## 2. Problemi nelle Feature Flags

### 2.1 Dipendenze Mancanti

Il piano propone:
```toml
handler = ["dep:dioxus", "dep:pwd-types", "dep:pwd-strength"]
```

Ma `PasswordHandler` richiede anche:
```toml
handler = ["form-field", "spinner", "icons", "dep:secrecy"]
```

### 2.2 Feature Flags Corrette (Proposta)

```toml
[features]
default = ["handler"]
handler = ["form-field", "analyzer", "icons", "dep:secrecy"]
analyzer = ["spinner", "dep:pwd-types"]
form-field = ["icons", "dep:secrecy"]
spinner = ["dep:dioxus"]
icons = ["dep:dioxus"]
full = ["handler"]
```

---

## 3. Problemi API

### 3.1 Breaking Change `on_password_change`

**Attuale:**
```rust
pub on_password_change: Callback<FormSecret>
```

**Piano (non documentato come breaking):**
```rust
pub on_password_change: Callback<PasswordChangeResult>
```

**Problema:** I consumer esistenti dovranno modificare il loro codice.

**Opzioni:**
1. Mantenere API attuale e aggiungere callback separato per score/strength
2. Documentare breaking change e fornire guida migrazione
3. Fornire entrambi i callback (backward compatible)

### 3.2 `PasswordChangeResult` senza Home

La struct definita nel piano non ha una collocazione chiara:
- Se in `pwd-dioxus` → crea dipendenza da `pwd-types` per i tipi contenuti
- Se in `pwd-types` → richiede modifica a quel crate

---

## 4. CSS Incompleto

### 4.1 Classi Non Elencate nel Piano

Oltre alle righe 773-792 e 1082-1148, `FormField` usa:
- `.form-group`
- `.form-label`
- `.pwd-input`

Queste devono essere identificate e estratte.

### 4.2 Slittamento Righe

Il piano indica righe 1078-1148 per strength analyzer, ma attualmente parte da **1082**.

---

## 5. Icons da Chiarire

### 5.1 Quali Icone Estrarre?

Il progetto ha:
- `visibility_icons.rs`: `EyeIcon`, `EyeOffIcon`
- `action_icons.rs`: `MagicWandIcon` (e altre?)
- `alert_icons.rs`: icone di alert

**Domanda:** Estrarre tutte o solo quelle usate da PasswordHandler?

### 5.2 `BaseIcon` Trait

`base_icon.rs` definisce un trait comune. Deve essere estratto come parte del modulo icons.

---

## 6. FormField: Tipi da Estrarre

`form_field.rs` contiene più tipi che devono essere estratti insieme:

```rust
pub struct FormSecret(pub SecretString);
pub trait FormValue: Clone + PartialEq + 'static { ... }
pub enum InputType { ... }
pub fn FormField<T: FormValue>(...) -> Element
```

Implementazioni di `FormValue` per:
- `String`
- `i32`
- `Option<String>`
- `FormSecret`

---

## 7. Azioni Richieste

### Prima dell'Implementazione

1. [ ] Mappare completamente le dipendenze tra componenti
2. [ ] Decidere quali icone estrarre
3. [ ] Identificare tutto il CSS necessario
4. [ ] Risolvere il problema della collocazione di `PasswordChangeResult`
5. [ ] Decidere strategia per breaking change API
6. [ ] Aggiornare feature flags con dipendenze corrette

### Durante l'Analisi

- [ ] Verificare se `SpinnerSize` deve essere pubblico
- [ ] Verificare se `BaseIcon` trait è necessario
- [ ] Decidere se estrarre `FormValue` implementations per tipi standard

---

## Riferimenti

- Piano principale: `docs/plans/2026-02-27-pwd-dixous-crate.md`
- File sorgente PasswordHandler: `src/components/globals/password_handler/component.rs`
- File sorgente FormField: `src/components/globals/form_field.rs`
- File sorgente Spinner: `src/components/globals/spinner/`
- File sorgente Icons: `src/components/globals/svgs/`
