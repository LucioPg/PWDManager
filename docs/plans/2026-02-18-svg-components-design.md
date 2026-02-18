# Design: Sistema Componenti SVG Riutilizzabili

**Data:** 2026-02-18
**Stato:** Approvato
**Branch target:** dev-improve-table-&-tablerow-dioxus-25

## Obiettivo

Estrarre gli SVG inline in componenti riutilizzabili per:
- Ridurre la verbosità del codice RSX
- Centralizzare la gestione degli attributi SVG standard
- Facilitare la manutenzione e l'aggiunta di nuove icone

## Analisi dello Stato Attuale

### SVG Identificati

| File | Icona | Uso |
|------|-------|-----|
| `form_field.rs` | Eye (occhio aperto) | Toggle visibilità password |
| `form_field.rs` | Eye-off (occhio chiuso) | Toggle visibilità password |
| `table_row.rs` | Burger (tre linee) | Menu contestuale |
| `table_row.rs` | Gear (ingranaggio) | Pulsante modifica |
| `table_row.rs` | Trash (cestino) | Pulsante elimina |
| `logout.rs` | Logout (freccia uscita) | Pagina logout |
| `user_deletion.rs` | Warning (triangolo) | Alert pericolo |

### Problemi Correnti

1. **Duplicazione**: Ogni uso ripete tutti gli attributi SVG standard
2. **Manutenibilità**: Cambiare un attributo richiede modifiche in più file
3. **Leggibilità**: Gli SVG inline occupano molto spazio nel RSX

## Design

### Struttura File

```
src/components/globals/svgs/
├── mod.rs              # Re-export di tutti i componenti
├── base_icon.rs        # Componente generico SvgIcon
├── action_icons.rs     # EditIcon, DeleteIcon, BurgerIcon
├── visibility_icons.rs # EyeIcon, EyeOffIcon
└── alert_icons.rs      # WarningIcon, LogoutIcon
```

### Componente Base: SvgIcon

Componente generico con props configurabili:

```rust
#[derive(Props, Clone, PartialEq)]
pub struct SvgIconProps {
    pub children: Element,
    #[props(default = "24".to_string())]
    pub size: String,
    #[props(default = "2".to_string())]
    pub stroke_width: String,
    #[props(default)]
    pub class: Option<String>,
}
```

Attributi fissi standardizzati:
- `view_box: "0 0 24 24"`
- `fill: "none"`
- `stroke: "currentColor"`
- `stroke_linecap: "round"`
- `stroke_linejoin: "round"`

### Componenti Specifici

Ogni icona wrappa `SvgIcon` con i path predefiniti e defaults sensati:

| Icona | Default Size | Categoria |
|-------|--------------|-----------|
| EyeIcon | 20 | visibility |
| EyeOffIcon | 20 | visibility |
| BurgerIcon | 18 | action |
| EditIcon | 18 | action |
| DeleteIcon | 18 | action |
| WarningIcon | 24 | alert |
| LogoutIcon | 64 | alert |

### Sistema di Re-export

**svgs/mod.rs:**
```rust
mod base_icon;
mod action_icons;
mod visibility_icons;
mod alert_icons;

pub use base_icon::SvgIcon;
pub use action_icons::{EditIcon, DeleteIcon, BurgerIcon};
pub use visibility_icons::{EyeIcon, EyeOffIcon};
pub use alert_icons::{WarningIcon, LogoutIcon};
```

**globals/mod.rs** (aggiunta):
```rust
pub use svgs::{
    SvgIcon,
    EditIcon, DeleteIcon, BurgerIcon,
    EyeIcon, EyeOffIcon,
    WarningIcon, LogoutIcon,
};
```

## Esempio di Refactoring

### Prima (form_field.rs):
```rust
svg {
    xmlns: "http://www.w3.org/2000/svg",
    width: "20",
    height: "20",
    view_box: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    stroke_width: "2",
    stroke_linecap: "round",
    stroke_linejoin: "round",
    path { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }
    circle { cx: "12", cy: "12", r: "3" }
}
```

### Dopo:
```rust
EyeIcon { class: Some("text-current".to_string()) }
```

## Benefici Attesi

| Componente | Righe prima | Righe dopo | Riduzione |
|------------|-------------|------------|-----------|
| form_field.rs | ~30 | ~2 | ~93% |
| table_row.rs | ~45 | ~6 | ~87% |
| logout.rs | ~12 | ~2 | ~83% |
| user_deletion.rs | ~10 | ~2 | ~80% |

## Convenzioni

1. **Attributi SVG in snake_case**: `view_box`, `stroke_width`, `stroke_linecap`
2. **Defaults sensati per categoria**: icone tabella più piccole (18), toggle password (20), alert (24+)
3. **Re-export da globals**: coerente con il pattern esistente del progetto

## Decisioni Prese

1. **Flessibilità**: Props per `size`, `class`, `stroke_width` (non tutti gli attributi SVG)
2. **Organizzazione**: Raggruppamento logico per categoria (action, visibility, alert)
3. **API**: Props opzionali con defaults sensati
4. **Import**: Re-export dal modulo `globals` (pattern esistente)
