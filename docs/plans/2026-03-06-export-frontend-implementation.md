# Export Frontend Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

## Stato Avanzamento

| Task | Stato | Commit |
|------|-------|--------|
| 1. Default ExportFormat | ✅ Completato | `2f89e23` |
| 2. ExportData Context | ✅ Completato | `454eab5` |
| 3. Modulo features | ✅ Completato | `7aa8a79` |
| 4. ExportWarningDialog | ✅ Completato | `295092c` |
| 5. ExportProgressChn | ✅ Completato | `caada34` |
| 6. ExportProgressDialog | ✅ Completato | `0802d5a` |
| 7. Modulo dialogs | ✅ Completato | `6a16d2b` |
| 8. DashboardMenu integration | ✅ Completato | `ae3ad1e` |
| 9. Test End-to-End | ⏳ Pending | - |

**Ultimo aggiornamento:** 2026-03-06 - Batch 3 completato (Task 7-8)

---

**Goal:** Connettere la funzionalità di export al frontend tramite il pulsante nel menu della dashboard, usando il pattern migrazione: dialog conferma → dialog progress con avvio automatico → toast per errori.

**Architecture:** Riutilizza i tipi esistenti (`ExportFormat`, `ExportablePassword`, `export_passwords_pipeline_with_progress`) e segue il pattern dei dialog di migrazione esistenti. Il flusso è: click menu → FileDialog salva → dialog conferma → dialog progress → toast completamento/errore.

**Tech Stack:** Dioxus 0.7, DaisyUI 5, rfd (rust-native-dialogs), pwd-dioxus toast, mpsc channel per progress tracking

---

## Task 1: Aggiungere Default a ExportFormat

**Files:**
- Modify: `src/backend/export_types.rs`

**Step 1: Aggiungere impl Default**

In `src/backend/export_types.rs`, dopo l'enum `ExportFormat`, aggiungere:

```rust
impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::Json
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/backend/export_types.rs
git commit -m "feat(export): add Default impl for ExportFormat"
```

---

## Task 2: Creare ExportData Context

**Files:**
- Create: `src/components/features/export_data.rs`

**Step 1: Creare il file export_data.rs**

```rust
//! Context per l'export delle password.
//!
//! Contiene i dati necessari per eseguire l'export:
//! - user_id: ID dell'utente corrente
//! - output_path: Path dove salvare il file
//! - format: Formato di export (JSON, CSV, XML)

use crate::backend::export_types::ExportFormat;
use std::path::PathBuf;

/// Dati di contesto per l'export delle password.
#[derive(Clone, Debug, Default)]
pub struct ExportData {
    pub user_id: i64,
    pub output_path: PathBuf,
    pub format: ExportFormat,
}

impl ExportData {
    pub fn new(user_id: i64, output_path: PathBuf, format: ExportFormat) -> Self {
        Self {
            user_id,
            output_path,
            format,
        }
    }
}
```

**Step 2: Verificare che il file compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/features/export_data.rs
git commit -m "feat(export): add ExportData context struct"
```

---

## Task 3: Aggiungere export_data al modulo features

**Files:**
- Modify: `src/components/features/mod.rs`

**Step 1: Aggiungere il modulo export_data**

In `src/components/features/mod.rs`, aggiungere all'inizio:

```rust
mod export_data;

pub use export_data::*;
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/features/mod.rs
git commit -m "feat(export): export ExportData from features module"
```

---

## Task 4: Creare ExportWarningDialog

**Files:**
- Create: `src/components/globals/dialogs/export_warning.rs`

**Step 1: Creare il dialog di conferma export**

```rust
use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

/// Dialog di conferma per l'export delle password.
///
/// Mostra un warning che le password saranno esportate in chiaro.
#[component]
pub fn ExportWarningDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Path del file di export (solo display)
    output_path: String,

    /// Formato di export (solo display)
    format: String,

    /// Callback quando l'utente conferma l'export
    on_confirm: EventHandler<()>,

    /// Callback quando l'utente annulla
    #[props(default)]
    on_cancel: EventHandler<()>,
) -> Element {
    let mut open_clone = open.clone();

    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open: open,
            on_close: move |_| {
                on_cancel.call(());
                open_clone.set(false);
            },
            variant: ModalVariant::Middle,

            // Close button "X" in alto a destra
            button {
                class: "absolute top-2 right-2 btn btn-sm btn-circle btn-ghost",
                onclick: move |_| {
                    on_cancel.call(());
                    open_clone.set(false);
                },
                "✕"
            }

            // Icona warning
            div {
                class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon {
                    class: Some("w-6 h-6".to_string()),
                }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Export Passwords" }

            // Dettagli export
            p { class: "py-2",
                "You are about to export your passwords to:"
            }
            p { class: "font-mono text-sm bg-base-200 p-2 rounded mb-2 break-all",
                "{output_path}"
            }
            p { class: "text-sm opacity-70 mb-4",
                "Format: {format}"
            }

            // Warning
            p {
                class: "text-warning-600 py-2",
                strong { "Warning: " }
                "Your passwords will be exported in plaintext. Keep the file secure!"
            }

            // Action buttons
            div {
                class: "modal-action",

                ActionButton {
                    text: "Export".to_string(),
                    variant: ButtonVariant::Primary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_confirm.call(());
                        open_clone.set(false);
                    },
                }
                ActionButton {
                    text: "Cancel".to_string(),
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Button,
                    size: ButtonSize::Normal,
                    on_click: move |_| {
                        on_cancel.call(());
                        open_clone.set(false);
                    },
                }
            }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/dialogs/export_warning.rs
git commit -m "feat(export): add ExportWarningDialog component"
```

---

## Task 5: Creare ExportProgressChn Component

**Files:**
- Create: `src/components/features/export_progress.rs`

**Step 1: Creare il componente di progress export**

```rust
//! Componente per mostrare il progresso dell'export.

use crate::backend::export::export_passwords_pipeline_with_progress;
use crate::backend::migration_types::{MigrationStage, ProgressMessage};
use crate::components::ExportData;
use crate::components::{show_toast_error, show_toast_success, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI (versione export).
fn format_export_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing export...".to_string(),
        MigrationStage::Decrypting => "Decrypting passwords...".to_string(),
        MigrationStage::Serializing => "Serializing data...".to_string(),
        MigrationStage::Writing => "Writing file...".to_string(),
        MigrationStage::Completed => "Export completed!".to_string(),
        MigrationStage::Failed => "Export failed".to_string(),
        _ => "Processing...".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ExportProgressChn(
    /// Callback quando l'export è completato con successo
    on_completed: Signal<bool>,

    /// Callback quando l'export fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    let mut status_message = use_signal(|| String::new());
    let mut export_started = use_signal(|| false);

    let context = use_context::<Signal<ExportData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia export automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio
        if export_started() {
            return;
        }
        export_started.set(true);

        let context = context.clone();
        let pool = pool.clone();
        let mut on_completed = on_completed.clone();
        let mut on_failed = on_failed.clone();
        let toast = toast.clone();

        let (tx, mut rx) = mpsc::channel::<ProgressMessage>(100);

        // Task per ricevere progress updates
        spawn(async move {
            while let Some(msg) = rx.recv().await {
                stage.set(msg.stage.clone());
                progress.set(msg.percentage());
                status_message.set(format_export_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed.set(true);
                }
            }
        });

        // Task per eseguire l'export
        spawn(async move {
            let user_id = context.read().user_id;
            let output_path = context.read().output_path.clone();
            let format = context.read().format;

            let result = export_passwords_pipeline_with_progress(
                &pool,
                user_id,
                &output_path,
                format,
                Some(Arc::new(tx)),
            )
            .await;

            match result {
                Ok(()) => {
                    show_toast_success(
                        format!("Export completed: {}", output_path.display()),
                        toast,
                    );
                }
                Err(e) => {
                    show_toast_error(format!("Export failed: {}", e), toast);
                    stage.set(MigrationStage::Failed);
                    on_failed.set(true);
                }
            }
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            // Messaggio stato
            p { class: "text-center font-medium text-base-content",
                "{status_message}"
            }

            // Progress bar DaisyUI
            progress {
                class: "progress progress-primary w-full",
                value: "{progress}",
                max: "100",
            }

            // Percentuale
            p { class: "text-center text-sm opacity-70",
                "{progress}%"
            }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Aggiungere al modulo features**

In `src/components/features/mod.rs`, aggiungere:

```rust
mod export_progress;

pub use export_progress::*;
```

**Step 4: Commit**

```bash
git add src/components/features/export_progress.rs src/components/features/mod.rs
git commit -m "feat(export): add ExportProgressChn component with mpsc channel"
```

---

## Task 6: Creare ExportProgressDialog

**Files:**
- Create: `src/components/globals/dialogs/export_progress.rs`

**Step 1: Creare il dialog di progress export**

```rust
use super::base_modal::ModalVariant;
use crate::components::ExportProgressChn;
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

/// Dialog che mostra il progresso dell'export.
///
/// Non può essere chiuso durante l'export (on_close vuoto).
#[component]
pub fn ExportProgressDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Signal che diventa true quando l'export è completato
    on_completed: Signal<bool>,

    /// Signal che diventa true se l'export fallisce
    #[props(default)]
    on_failed: Signal<bool>,
) -> Element {
    rsx! {
        crate::components::globals::dialogs::BaseModal {
            open,
            on_close: move |_| {},
            variant: ModalVariant::Middle,

            // Icona warning
            div {
                class: "alert alert-warning mb-4 flex items-center justify-center mx-10",
                WarningIcon { class: Some("w-6 h-6".to_string()) }
            }

            // Titolo
            h3 { class: "font-bold text-lg mb-2", "Exporting Passwords" }

            // Messaggio
            p { class: "py-4",
                "Your passwords are being exported. Please wait..."
            }

            p { class: "text-warning-600 py-2",
                "The dialog will close automatically when the export is complete."
            }

            ExportProgressChn { on_completed, on_failed }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/dialogs/export_progress.rs
git commit -m "feat(export): add ExportProgressDialog component"
```

---

## Task 7: Aggiornare mod.rs dialogs

**Files:**
- Modify: `src/components/globals/dialogs/mod.rs`

**Step 1: Aggiungere i nuovi moduli**

In `src/components/globals/dialogs/mod.rs`, aggiungere:

```rust
mod export_progress;
mod export_warning;

pub use export_progress::*;
pub use export_warning::*;
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/dialogs/mod.rs
git commit -m "feat(export): export export dialogs from module"
```

---

## Task 8: Aggiornare DashboardMenu con Export

**Files:**
- Modify: `src/components/features/dashboard_menu.rs`

**Step 1: Aggiungere imports necessari**

All'inizio di `src/components/features/dashboard_menu.rs`, aggiungere agli use esistenti:

```rust
use crate::backend::export_types::ExportFormat;
use crate::components::ExportData;
use crate::components::ExportWarningDialog;
use crate::components::ExportProgressDialog;
use rfd::FileDialog;
```

**Step 2: Aggiungere signals per l'export**

Dentro `DashboardMenu`, dopo i signal esistenti, aggiungere:

```rust
// Export state
let mut export_warning_open = use_signal(|| false);
let mut export_progress_open = use_signal(|| false);
let mut export_completed = use_signal(|| false);
let mut export_failed = use_signal(|| false);
let mut export_data = use_signal(|| ExportData::default());
let mut export_format = use_signal(|| ExportFormat::Json);
```

**Step 3: Aggiungere funzione helper per aprire FileDialog**

Prima del rsx!, aggiungere:

```rust
// Funzione per aprire il dialog di salvataggio
let open_save_dialog = move |format: ExportFormat| {
    let user_clone = user.clone();
    let toast = toast.clone();

    spawn(async move {
        if let Some(user) = user_clone {
            // Usa spawn_blocking per FileDialog
            let file_result = tokio::task::spawn_blocking(move || {
                FileDialog::new()
                    .add_filter(
                        "Export File",
                        &[format.extension()],
                    )
                    .set_file_name(&format!(
                        "pwdmanager_export.{}",
                        format.extension()
                    ))
                    .save_file()
            })
            .await;

            match file_result {
                Ok(Some(path)) => {
                    export_data.set(ExportData::new(user.id, path, format));
                    export_format.set(format);
                    export_warning_open.set(true);
                }
                Ok(None) => {
                    // Utente ha annullato
                    tracing::info!("Export cancelled by user");
                }
                Err(e) => {
                    show_toast_error(
                        format!("Error opening save dialog: {}", e),
                        toast,
                    );
                }
            }
        }
    });
};

// Handler per confermare l'export
let on_export_confirm = move |_| {
    export_warning_open.set(false);
    export_progress_open.set(true);
};

// Handler per chiudere il progress dopo completamento
use_effect(move || {
    if export_completed() || export_failed() {
        export_progress_open.set(false);
    }
});
```

**Step 4: Aggiornare i pulsanti Export nel menu**

Sostituire i button dell'Export submenu con:

```rust
// Export submenu
li {
    details {
        summary { class: "cursor-pointer", "Export" }
        ul {
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_save_dialog(ExportFormat::Json);
                    },
                    "JSON"
                }
            }
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_save_dialog(ExportFormat::Csv);
                    },
                    "CSV"
                }
            }
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_save_dialog(ExportFormat::Xml);
                    },
                    "XML"
                }
            }
        }
    }
}
```

**Step 5: Aggiungere i dialog prima della chiusura del componente**

Prima della chiusura del rsx!, aggiungere:

```rust
// Export warning dialog
ExportWarningDialog {
    open: export_warning_open,
    output_path: export_data.read().output_path.display().to_string(),
    format: format!("{:?}", export_format()),
    on_confirm: on_export_confirm,
    on_cancel: move |_| {
        export_warning_open.set(false);
    },
}

// Export progress dialog
ExportProgressDialog {
    open: export_progress_open,
    on_completed: export_completed,
    on_failed: export_failed,
}
```

**Step 6: Verificare che compili**

Run: `cargo check`
Expected: Possibili errori da risolvere

**Step 7: Commit**

```bash
git add src/components/features/dashboard_menu.rs
git commit -m "feat(export): integrate export functionality in dashboard menu"
```

---

## Task 9: Test Manuale End-to-End

**Step 1: Avviare l'applicazione**

Run: `dx serve --desktop`

**Step 2: Testare il flusso completo**

1. Login con utente esistente
2. Aprire dashboard menu (tre puntini)
3. Cliccare Export → JSON
4. Verificare che si apra FileDialog
5. Selezionare percorso e salvare
6. Verificare che si apra ExportWarningDialog
7. Cliccare Export
8. Verificare che si apra ExportProgressDialog con progress bar
9. Verificare toast di successo
10. Verificare che il file sia stato creato

**Step 3: Testare errori**

1. Ripetere con password create apposta per testare edge cases
2. Provare a cancellare FileDialog (should do nothing)
3. Provare Export Cancel (should do nothing)

**Step 4: Commit finale**

```bash
git add -A
git commit -m "feat(export): complete export frontend integration"
```

---

## Note Tecniche

### Flusso Dati Completo

```
User Click Export → FileDialog (save_file)
                      ↓
               ExportData creato
                      ↓
            ExportWarningDialog (conferma)
                      ↓
            ExportProgressDialog
                      ↓
         ExportProgressChn (use_effect)
                      ↓
    export_passwords_pipeline_with_progress
                      ↓
         ProgressMessage via mpsc channel
                      ↓
         UI aggiorna progress bar
                      ↓
         Toast success/error
```

### Dipendenze già presenti

- `rfd` - per FileDialog nativo
- `pwd-dioxus` - per toast (`show_toast_error`, `show_toast_success`)
- `tokio::sync::mpsc` - per progress channel
- Tipi backend già esistenti in `export.rs` e `export_types.rs`

### Pattern riutilizzati

- `MigrationWarningDialog` → `ExportWarningDialog`
- `MigrationProgressDialog` → `ExportProgressDialog`
- `ProgressMigrationChn` → `ExportProgressChn`
- `MigrationData` → `ExportData`
- `spawn_blocking` per FileDialog (da `ui_utils.rs`)
- `WarningIcon` per icone (gia esistente nel progetto)
