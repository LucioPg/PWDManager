# SVG Components Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Estrarre gli SVG inline in componenti riutilizzabili organizzati per categoria.

**Architecture:** Componente base `SvgIcon` con props generiche wrappato da componenti specifici per ogni icona (EyeIcon, EditIcon, ecc.). Organizzazione per categoria in file separati con re-export dal modulo `globals`.

**Tech Stack:** Dioxus 0.7, Rust, RSX

---

## Task 1: Creare il componente base SvgIcon

**Files:**
- Create: `src/components/globals/svgs/base_icon.rs`

**Step 1: Creare il file base_icon.rs con il componente generico**

```rust
use dioxus::prelude::*;

/// Props per il componente SVG generico
#[derive(Props, Clone, PartialEq)]
pub struct SvgIconProps {
    /// Contenuto SVG (path, circle, line, ecc.)
    pub children: Element,
    /// Dimensione dell'icona (default: "24")
    #[props(default = "24".to_string())]
    pub size: String,
    /// Spessore del tratto (default: "2")
    #[props(default = "2".to_string())]
    pub stroke_width: String,
    /// Classe CSS aggiuntiva
    #[props(default)]
    pub class: Option<String>,
}

/// Componente SVG generico riutilizzabile
///
/// Fornisce attributi SVG standardizzati con possibilità di
/// personalizzare dimensioni, spessore tratto e classe CSS.
#[component]
pub fn SvgIcon(props: SvgIconProps) -> Element {
    let class_str = props.class.unwrap_or_default();

    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "{props.size}",
            height: "{props.size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "{props.stroke_width}",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            class: "{class_str}",
            {props.children}
        }
    }
}
```

**Step 2: Verificare che il codice compili**

Run: `cargo check`
Expected: Nessun errore (il modulo non è ancora importato)

**Step 3: Commit**

```bash
git add src/components/globals/svgs/base_icon.rs
git commit -m "feat(svg): add base SvgIcon component"
```

---

## Task 2: Creare le icone di visibilità (EyeIcon, EyeOffIcon)

**Files:**
- Create: `src/components/globals/svgs/visibility_icons.rs`

**Step 1: Creare il file visibility_icons.rs**

```rust
use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona occhio aperto - indica che la password è nascosta
#[component]
pub fn EyeIcon(
    #[props(default = "20".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

/// Icona occhio chiuso/sbarrato - indica che la password è visibile
#[component]
pub fn EyeOffIcon(
    #[props(default = "20".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" }
            line { x1: "1", y1: "1", x2: "23", y2: "23" }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/svgs/visibility_icons.rs
git commit -m "feat(svg): add EyeIcon and EyeOffIcon components"
```

---

## Task 3: Creare le icone di azione (EditIcon, DeleteIcon, BurgerIcon)

**Files:**
- Create: `src/components/globals/svgs/action_icons.rs`

**Step 1: Creare il file action_icons.rs**

```rust
use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona burger/hamburger - menu contestuale con tre linee
#[component]
pub fn BurgerIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            line { x1: "3", y1: "6", x2: "21", y2: "6" }
            line { x1: "3", y1: "12", x2: "21", y2: "12" }
            line { x1: "3", y1: "18", x2: "21", y2: "18" }
        }
    }
}

/// Icona ingranaggio - pulsante modifica/impostazioni
#[component]
pub fn EditIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

/// Icona cestino - pulsante elimina
#[component]
pub fn DeleteIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M3 6h18" }
            path { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" }
            path { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" }
            line { x1: "10", y1: "11", x2: "10", y2: "17" }
            line { x1: "14", y1: "11", x2: "14", y2: "17" }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/svgs/action_icons.rs
git commit -m "feat(svg): add BurgerIcon, EditIcon, and DeleteIcon components"
```

---

## Task 4: Creare le icone di alert (WarningIcon, LogoutIcon)

**Files:**
- Create: `src/components/globals/svgs/alert_icons.rs`

**Step 1: Creare il file alert_icons.rs**

```rust
use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona warning/triangolo - alert di pericolo
#[component]
pub fn WarningIcon(
    #[props(default = "24".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
        }
    }
}

/// Icona logout - freccia di uscita
#[component]
pub fn LogoutIcon(
    #[props(default = "64".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/svgs/alert_icons.rs
git commit -m "feat(svg): add WarningIcon and LogoutIcon components"
```

---

## Task 5: Aggiornare svgs/mod.rs con i re-export

**Files:**
- Modify: `src/components/globals/svgs/mod.rs`

**Step 1: Aggiornare mod.rs con tutti i moduli e re-export**

```rust
mod action_icons;
mod alert_icons;
mod base_icon;
mod visibility_icons;

// Re-export del componente base (per estensioni future)
pub use base_icon::SvgIcon;

// Re-export di tutte le icone specifiche
pub use action_icons::{BurgerIcon, DeleteIcon, EditIcon};
pub use alert_icons::{LogoutIcon, WarningIcon};
pub use visibility_icons::{EyeIcon, EyeOffIcon};
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/svgs/mod.rs
git commit -m "feat(svg): add module exports for all SVG components"
```

---

## Task 6: Aggiornare globals/mod.rs con i re-export

**Files:**
- Modify: `src/components/globals/mod.rs`

**Step 1: Aggiungere il modulo svgs e i re-export**

Cercare la sezione dei moduli e aggiungere:
```rust
pub mod svgs;
```

Cercare la sezione dei re-export e aggiungere:
```rust
pub use svgs::{
    BurgerIcon, DeleteIcon, EditIcon, EyeIcon, EyeOffIcon, LogoutIcon, SvgIcon, WarningIcon,
};
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/mod.rs
git commit -m "feat(svg): re-export SVG components from globals module"
```

---

## Task 7: Refactoring di form_field.rs

**Files:**
- Modify: `src/components/globals/form_field.rs`

**Step 1: Aggiungere gli import delle icone**

All'inizio del file, aggiungere:
```rust
use crate::components::globals::{EyeIcon, EyeOffIcon};
```

**Step 2: Sostituire l'SVG Eye con EyeIcon**

Cercare il blocco SVG dell'occhio aperto (circa righe 244-256) e sostituire con:
```rust
EyeIcon { class: Some("text-current".to_string()) }
```

**Step 3: Sostituire l'SVG EyeOff con EyeOffIcon**

Cercare il blocco SVG dell'occhio chiuso (circa righe 229-241) e sostituire con:
```rust
EyeOffIcon { class: Some("text-current".to_string()) }
```

**Step 4: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 5: Commit**

```bash
git add src/components/globals/form_field.rs
git commit -m "refactor(form_field): replace inline SVGs with icon components"
```

---

## Task 8: Refactoring di table_row.rs

**Files:**
- Modify: `src/components/globals/table/table_row.rs`

**Step 1: Aggiungere gli import delle icone**

All'inizio del file, aggiungere:
```rust
use crate::components::globals::{BurgerIcon, DeleteIcon, EditIcon};
```

**Step 2: Sostituire l'SVG Burger con BurgerIcon**

Cercare il blocco SVG burger (circa righe 69-82) e sostituire con:
```rust
BurgerIcon {}
```

**Step 3: Sostituire l'SVG Gear/Edit con EditIcon**

Cercare il blocco SVG ingranaggio (circa righe 134-148) e sostituire con:
```rust
EditIcon {}
```

**Step 4: Sostituire l'SVG Trash/Delete con DeleteIcon**

Cercare il blocco SVG cestino (circa righe 159-174) e sostituire con:
```rust
DeleteIcon {}
```

**Step 5: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 6: Commit**

```bash
git add src/components/globals/table/table_row.rs
git commit -m "refactor(table_row): replace inline SVGs with icon components"
```

---

## Task 9: Refactoring di logout.rs

**Files:**
- Modify: `src/components/features/logout.rs`

**Step 1: Aggiungere l'import dell'icona**

All'inizio del file, aggiungere:
```rust
use crate::components::globals::LogoutIcon;
```

**Step 2: Sostituire l'SVG Logout con LogoutIcon**

Cercare il blocco SVG logout (circa righe 27-38) e sostituire con:
```rust
LogoutIcon {
    class: Some("w-16 h-16 text-error-600 mx-auto mb-4".to_string()),
}
```

**Step 3: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 4: Commit**

```bash
git add src/components/features/logout.rs
git commit -m "refactor(logout): replace inline SVG with LogoutIcon component"
```

---

## Task 10: Refactoring di user_deletion.rs

**Files:**
- Modify: `src/components/globals/dialogs/user_deletion.rs`

**Step 1: Aggiungere l'import dell'icona**

All'inizio del file, aggiungere:
```rust
use crate::components::globals::WarningIcon;
```

**Step 2: Sostituire l'SVG Warning con WarningIcon**

Cercare il blocco SVG warning (circa righe 45-56) e sostituire con:
```rust
WarningIcon {
    class: Some("w-6 h-6".to_string()),
}
```

**Step 3: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 4: Commit**

```bash
git add src/components/globals/dialogs/user_deletion.rs
git commit -m "refactor(user_deletion): replace inline SVG with WarningIcon component"
```

---

## Task 11: Verifica finale e build

**Step 1: Eseguire cargo check completo**

Run: `cargo check`
Expected: Nessun errore o warning

**Step 2: Eseguire una build di test**

Run: `dx build --desktop`
Expected: Build completata con successo

**Step 3: Commit finale (se necessario)**

```bash
git add -A
git commit -m "chore: finalize SVG component refactoring"
```

---

## Riepilogo File Modificati/Creati

| File | Azione |
|------|--------|
| `src/components/globals/svgs/base_icon.rs` | Creato |
| `src/components/globals/svgs/visibility_icons.rs` | Creato |
| `src/components/globals/svgs/action_icons.rs` | Creato |
| `src/components/globals/svgs/alert_icons.rs` | Creato |
| `src/components/globals/svgs/mod.rs` | Modificato |
| `src/components/globals/mod.rs` | Modificato |
| `src/components/globals/form_field.rs` | Refactoring |
| `src/components/globals/table/table_row.rs` | Refactoring |
| `src/components/features/logout.rs` | Refactoring |
| `src/components/globals/dialogs/user_deletion.rs` | Refactoring |

---

## Note per l'Implementazione

1. **Attributi snake_case**: Tutti gli attributi SVG camelCase diventano snake_case in RSX (es. `viewBox` → `view_box`)
2. **Defaults sensati**: Le icone tabella usano size 18, i toggle password size 20, le alert size variabile
3. **Ordine export**: Mantenere ordine alfabetico nei re-export per consistenza
