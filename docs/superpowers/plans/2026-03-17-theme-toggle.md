# Theme Toggle Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aggiungere un toggle dark/light nella tab Aspect dei Settings con propagazione immediata e persistenza DB.

**Architecture:** Un `Signal<Theme>` globale fornito da `App` viene letto da `RouteWrapper` per applicare `data-theme` sul div root. `AuthWrapper` fa fetch dei settings dopo il login e aggiorna il Signal. `AspectSettings` e un componente self-contained che modifica il Signal al toggle e persiste nel DB con Save.

**Tech Stack:** Dioxus 0.7, sqlx/sqlx-template, pwd-dioxus Toggle, SQLite

**Spec:** `docs/superpowers/specs/2026-03-17-theme-toggle-design.md`

---

### Task 1: Signal globale in App

**Files:**
- Modify: `src/main.rs` (funzione `App`, riga ~39)

- [ ] **Step 1: Aggiungere Signal<Theme> globale in App**

Aggiungere l'import in cima al file:
```rust
use crate::backend::settings_types::Theme;
```

Dopo la riga `use_context_provider(move || auth_state);` (riga 39), aggiungere:
```rust
let mut app_theme = use_signal(|| Theme::Light);
use_context_provider(move || app_theme);
```

- [ ] **Step 2: Verifica compilazione**

Run: `cargo check`
Expected: Compilazione senza errori

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add global Theme signal in App component"
```

---

### Task 2: Rendere pubblico il modulo aspect_settings

**Files:**
- Modify: `src/components/features/mod.rs` (righe 1, 29)

- [ ] **Step 1: Rendere pubblico il modulo e aggiungere re-export**

In `src/components/features/mod.rs`:

1. Cambiare riga 1 da `mod aspect_settings;` a `pub mod aspect_settings;` — questo rende il percorso `crate::components::features::aspect_settings::AspectSettings` accessibile (usato da settings.rs riga 2).

2. Aggiungere alla fine (riga 29, dopo gli altri `pub use`):
```rust
pub use aspect_settings::*;
```

Questo segue il pattern di tutti gli altri moduli nel file (es. `pub mod dashboard;` + `pub use dashboard::*;`).

- [ ] **Step 2: Commit**

```bash
git add src/components/features/mod.rs
git commit -m "feat: export AspectSettings from features module"
```

---

### Task 3: Fetch tema in AuthWrapper

**Files:**
- Modify: `src/components/globals/auth_wrapper.rs` (intero file)

- [ ] **Step 1: Implementare fetch theme dopo login**

Riscrivere `AuthWrapper` per fare fetch dei settings utente dopo il login, aggiornando il Signal globale. Il fetch e silente (nessun toast): il tema e un preferenza non critica, non serve allarmare l'utente.

```rust
use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::Theme;
use crate::auth::AuthState;
use crate::Route;
use dioxus::prelude::*;
use sqlx::SqlitePool;

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    let mut app_theme = use_context::<Signal<Theme>>();

    // Flag per fetch unico dei settings
    let mut theme_fetched = use_signal(|| false);

    if !auth_state.is_logged_in() {
        nav.push(Route::LandingPage);
    }

    let user_id = auth_state.get_user_id();

    use_resource(move || {
        let pool = pool.clone();
        let mut app_theme = app_theme.clone();
        let mut theme_fetched = theme_fetched.clone();
        let user_id = user_id.clone();
        async move {
            if theme_fetched() || user_id <= 0 {
                return;
            }
            if let Ok(Some(settings)) = fetch_user_settings(&pool, user_id).await {
                app_theme.set(settings.theme);
            }
            theme_fetched.set(true);
        }
    });

    rsx! {
        Outlet::<Route> {}
    }
}
```

Note: non si usa `use_toast()` qui. Il fetch del tema e silente perche e una preferenza non critica.

- [ ] **Step 2: Verifica compilazione**

Run: `cargo check`
Expected: Compilazione senza errori

- [ ] **Step 3: Commit**

```bash
git add src/components/globals/auth_wrapper.rs
git commit -m "feat: fetch user theme in AuthWrapper after login"
```

---

### Task 4: data-theme in RouteWrapper

**Files:**
- Modify: `src/components/globals/route_wrapper.rs:5-20` (import + div root)

- [ ] **Step 1: Leggere Signal globale e applicare data-theme**

Aggiungere import in cima al file (riga 4):
```rust
use crate::backend::settings_types::Theme;
```

All'interno della funzione `RouteWrapper`, dopo la riga `let slogan_visibility = ...` (riga 17), aggiungere:
```rust
let mut app_theme = use_context::<Signal<Theme>>();
let data_theme = match *app_theme.read() {
    Theme::Dark => "dark",
    Theme::Light => "light",
};
```

Sostituire il div root da:
```rust
div { class: "relative min-h-screen w-full",
```
a:
```rust
div { class: "relative min-h-screen w-full", "data-theme": data_theme,
```

**IMPORTANTE:** `let mut app_theme` — in Dioxus 0.7 tutti i Signal devono essere `mut` (vedi MEMORY.md).

- [ ] **Step 2: Verifica compilazione**

Run: `cargo check`
Expected: Compilazione senza errori

- [ ] **Step 3: Commit**

```bash
git add src/components/globals/route_wrapper.rs
git commit -m "feat: apply data-theme attribute in RouteWrapper"
```

---

### Task 5: AspectSettings self-contained

**Files:**
- Modify: `src/components/features/aspect_settings.rs` (intero file)

- [ ] **Step 1: Implementare AspectSettings come componente self-contained**

Il componente:
- Legge `Signal<Theme>` dal context per lo stato del toggle
- Usa `use_resource` per fetch dei settings (serve l'id per upsert)
- Toggle aggiorna il Signal globale (cambio immediato)
- Save persiste tramite `UserSettings::upsert_by_id(&settings, pool)`

```rust
use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::{Theme, UserSettings};
use crate::auth::AuthState;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;
use pwd_dioxus::{show_toast_error, show_toast_success, use_toast};
use pwd_dioxus::{Toggle, ToggleColor, ToggleSize};
use sqlx::SqlitePool;

#[component]
pub fn AspectSettings() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let mut app_theme = use_context::<Signal<Theme>>();
    let toast = use_toast();
    let user_id = auth_state.get_user_id();

    // Signal per lo stato del toggle (light = checked, dark = unchecked)
    let mut is_light = use_signal(|| *app_theme.read() == Theme::Light);

    // Fetch settings per ottenere l'id (necessario per upsert)
    let mut settings_id = use_signal(|| Option::<i64>::None);
    let mut error = use_signal(|| None::<String>);
    let mut ready = use_signal(|| false);

    let _settings_resource = use_resource(move || {
        let pool = pool.clone();
        let user_id = user_id.clone();
        let mut settings_id = settings_id.clone();
        let mut ready = ready.clone();
        let mut error = error.clone();
        async move {
            match fetch_user_settings(&pool, user_id).await {
                Ok(Some(settings)) => {
                    settings_id.set(settings.id);
                    ready.set(true);
                }
                Ok(None) => {
                    // Nessun record: primo save creera il record
                    ready.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    ready.set(true);
                }
            }
        }
    });

    use_effect(move || {
        let mut this_error = error.clone();
        let toast = toast.clone();
        if let Some(msg) = this_error() {
            show_toast_error(format!("Error fetching settings: {}", msg), toast);
            this_error.set(None);
        }
    });

    // Sincronizza il Signal globale con il toggle locale
    let on_toggle = move |_| {
        let new_theme = if is_light() { Theme::Dark } else { Theme::Light };
        is_light.set(new_theme == Theme::Light);
        app_theme.set(new_theme);
    };

    let on_save = move |_| {
        let pool = pool.clone();
        let toast = toast.clone();
        let app_theme = app_theme.clone();
        let settings_id = settings_id.clone();
        spawn(async move {
            let theme = *app_theme.read();
            let settings = UserSettings {
                id: settings_id(),
                user_id,
                theme,
            };
            match UserSettings::upsert_by_id(&settings, pool).await {
                Ok(_) => {
                    show_toast_success("Theme saved!".to_string(), toast);
                }
                Err(e) => {
                    show_toast_error(format!("Failed to save theme: {}", e), toast);
                }
            }
        });
    };

    if !ready() {
        return rsx! {};
    }

    rsx! {
        div { class: "settings-page-body flex flex-col gap-2",
            div { class: "flex flex-row justify-between",
                label { class: "label cursor-pointer",
                    strong {
                        span { class: "label-text", "Light Theme" }
                    }
                }
                Toggle {
                    checked: is_light(),
                    onchange: on_toggle,
                    size: ToggleSize::Large,
                    color: ToggleColor::Success,
                }
            }
            ActionButton {
                text: "Save".to_string(),
                variant: ButtonVariant::Primary,
                button_type: ButtonType::Submit,
                size: ButtonSize::Normal,
                on_click: on_save,
            }
        }
    }
}
```

Note:
- `let _settings_resource = ...` — il binding non viene letto direttamente (il side effect aggiorna i signal), ma il prefix `_` previene warning del compilatore
- `UserSettings::upsert_by_id(&settings, pool)` — il secondo argomento viene passato per valore (pattern da `db_backend.rs:1013`)

- [ ] **Step 2: Verifica compilazione**

Run: `cargo check`
Expected: Compilazione senza errori

- [ ] **Step 3: Commit**

```bash
git add src/components/features/aspect_settings.rs
git commit -m "feat: implement AspectSettings with toggle and save"
```

---

### Task 6: Collegare AspectSettings in SettingsTabContent

**Files:**
- Modify: `src/components/features/settings.rs:58-71` (tab Aspect)

- [ ] **Step 1: Sostituire placeholder con AspectSettings**

L'import `use crate::components::features::aspect_settings::AspectSettings;` esiste gia alla riga 2 di settings.rs (aggiunto in Task 2 con il pub use). Verificare che sia presente.

Sostituire il contenuto della tab "Aspect" (righe 58-71):

Da:
```rust
TabContent {
    index: 2usize,
    class: "tabs-content",
    value: "Aspect".to_string(),
    div {
        width: "100%",
        height: "5rem",
        display: "flex",
        align_items: "center",
        justify_content: "center",
                    // ToDo!
            // AspectSettings { user_settings }
    }
}
```

A:
```rust
TabContent {
    index: 2usize,
    class: "tabs-content",
    value: "Aspect".to_string(),
    AspectSettings {}
}
```

- [ ] **Step 2: Verifica compilazione**

Run: `cargo check`
Expected: Compilazione senza errori

- [ ] **Step 3: Commit**

```bash
git add src/components/features/settings.rs
git commit -m "feat: wire AspectSettings into Settings tab"
```

---

### Task 7: Verifica finale

- [ ] **Step 1: Verifica compilazione completa**

Run: `cargo check`
Expected: Compilazione senza errori né warning

- [ ] **Step 2: Verifica test (se presenti)**

Run: `cargo test`
Expected: Nessun test rotto
