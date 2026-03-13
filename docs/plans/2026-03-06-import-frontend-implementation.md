# Import Frontend Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

## Stato Avanzamento

| Task | Stato | Commit |
|------|-------|--------|
| 1. ImportData Context | ⏳ Pending | - |
| 2. ImportWarningDialog | ⏳ Pending | - |
| 3. ImportProgressChn | ⏳ Pending | - |
| 4. ImportProgressDialog | ⏳ Pending | - |
| 5. Modulo dialogs/features | ⏳ Pending | - |
| 6. DashboardMenu integration | ⏳ Pending | - |
| 7. Test End-to-End | ⏳ Pending | - |

**Ultimo aggiornamento:** 2026-03-06 - Piano creato

---

**Goal:** Connettere la funzionalità di import al frontend tramite il pulsante nel menu della dashboard, usando il pattern migrazione: dialog conferma con info su duplicati → dialog progress con avvio automatico → toast per errori/successo.

**Architecture:** Riutilizza i tipi esistenti (`ExportFormat`, `ExportablePassword`, `import_passwords_pipeline_with_progress`) e segue il pattern dei dialog di migrazione e export esistenti. Il flusso è: click menu → FileDialog apre file → dialog conferma con warning → dialog progress → toast completamento/errore.

**Tech Stack:** Dioxus 0.7, DaisyUI 5, rfd (rust-native-dialogs), pwd-dioxus toast, mpsc channel per progress tracking

---

## Task 1: Creare ImportData Context

**Files:**
- Create: `src/components/features/import_data.rs`

**Step 1: Creare il file import_data.rs**

```rust
//! Context per l'import delle password.
//!
//! Contiene i dati necessari per eseguire l'import:
//! - user_id: ID dell'utente corrente
//! - input_path: Path del file da importare
//! - format: Formato di import (JSON, CSV, XML)

use crate::backend::export_types::ExportFormat;
use std::path::PathBuf;

/// Dati di contesto per l'import delle password.
#[derive(Clone, Debug, Default)]
pub struct ImportData {
    pub user_id: i64,
    pub input_path: PathBuf,
    pub format: ExportFormat,
}

impl ImportData {
    pub fn new(user_id: i64, input_path: PathBuf, format: ExportFormat) -> Self {
        Self {
            user_id,
            input_path,
            format,
        }
    }
}
```

**Step 2: Verificare che il file compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Aggiungere al modulo features**

In `src/components/features/mod.rs`, aggiungere:

```rust
mod import_data;

pub use import_data::*;
```

**Step 4: Commit**

```bash
git add src/components/features/import_data.rs src/components/features/mod.rs
git commit -m "feat(import): add ImportData context struct"
```

---

## Task 2: Creare ImportWarningDialog

**Files:**
- Create: `src/components/globals/dialogs/import_warning.rs`

**Step 1: Creare il dialog di conferma import**

```rust
use super::base_modal::ModalVariant;
use crate::components::globals::WarningIcon;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

/// Dialog di conferma per l'import delle password.
///
/// Mostra info sul comportamento dell'import:
/// - I duplicati (location+password) vengono saltati
/// - Password con stessa location ma password diversa vengono importate come nuove
#[component]
pub fn ImportWarningDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Path del file di import (solo display)
    input_path: String,

    /// Formato di import (solo display)
    format: String,

    /// Callback quando l'utente conferma l'import
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
            h3 { class: "font-bold text-lg mb-2", "Import Passwords" }

            // Dettagli import
            p { class: "py-2",
                "You are about to import passwords from:"
            }
            p { class: "font-mono text-sm bg-base-200 p-2 rounded mb-2 break-all",
                "{input_path}"
            }
            p { class: "text-sm opacity-70 mb-4",
                "Format: {format}"
            }

            // Warning su duplicati
            p {
                class: "text-warning-600 py-2",
                strong { "Note: " }
                "Duplicate passwords (same location and password) in the file will be skipped. "
                "Passwords that already exist in your database will also be skipped."
            }

            p {
                class: "text-info-600 py-2",
                strong { "Info: " }
                "Passwords with the same location but different password will be imported as new entries."
            }

            // Action buttons
            div {
                class: "modal-action",

                ActionButton {
                    text: "Import".to_string(),
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
git add src/components/globals/dialogs/import_warning.rs
git commit -m "feat(import): add ImportWarningDialog component"
```

---

## Task 3: Creare ImportProgressChn Component

**Files:**
- Create: `src/components/features/import_progress.rs`

**Step 1: Creare il componente di progress import**

```rust
//! Componente per mostrare il progresso dell'import.

use crate::backend::import::{import_passwords_pipeline_with_progress, ImportResult};
use crate::backend::migration_types::{MigrationStage, ProgressMessage};
use crate::components::ImportData;
use crate::components::{show_toast_error, show_toast_success, use_toast};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Formatta il messaggio dello stage per la UI (versione import).
fn format_import_stage_message(stage: &MigrationStage) -> String {
    match stage {
        MigrationStage::Idle => "Preparing import...".to_string(),
        MigrationStage::Reading => "Reading file...".to_string(),
        MigrationStage::Deserializing => "Parsing file...".to_string(),
        MigrationStage::Deduplicating => "Removing duplicates...".to_string(),
        MigrationStage::Encrypting => "Encrypting passwords...".to_string(),
        MigrationStage::Importing => "Importing to database...".to_string(),
        MigrationStage::Completed => "Import completed!".to_string(),
        MigrationStage::Failed => "Import failed".to_string(),
        _ => "Processing...".to_string(),
    }
}

#[allow(non_snake_case)]
#[component]
pub fn ImportProgressChn(
    /// Callback quando l'import è completato con successo
    on_completed: Signal<bool>,

    /// Callback quando l'import fallisce
    on_failed: Signal<bool>,
) -> Element {
    let mut stage = use_signal(|| MigrationStage::Idle);
    let mut progress = use_signal(|| 0usize);
    let mut status_message = use_signal(|| String::new());
    let mut import_started = use_signal(|| false);

    let context = use_context::<Signal<ImportData>>();
    let pool = use_context::<SqlitePool>();
    let toast = use_toast();

    // Avvia import automaticamente al mount del componente
    use_effect(move || {
        // Evita doppio avvio
        if import_started() {
            return;
        }
        import_started.set(true);

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
                status_message.set(format_import_stage_message(&msg.stage));

                if msg.stage == MigrationStage::Completed {
                    on_completed.set(true);
                }
            }
        });

        // Task per eseguire l'import
        spawn(async move {
            let user_id = context.read().user_id;
            let input_path = context.read().input_path.clone();
            let format = context.read().format;

            let result = import_passwords_pipeline_with_progress(
                &pool,
                user_id,
                &input_path,
                format,
                Some(Arc::new(tx)),
            )
            .await;

            match result {
                Ok(import_res) => {
                    show_toast_success(
                        format!(
                            "Import completed: {} passwords imported, {} skipped (duplicates)",
                            import_res.imported_count,
                            import_res.skipped_duplicates
                        ),
                        toast,
                    );
                }
                Err(e) => {
                    show_toast_error(format!("Import failed: {}", e), toast);
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
mod import_progress;

pub use import_progress::*;
```

**Step 4: Commit**

```bash
git add src/components/features/import_progress.rs src/components/features/mod.rs
git commit -m "feat(import): add ImportProgressChn component with mpsc channel"
```

---

## Task 4: Creare ImportProgressDialog

**Files:**
- Create: `src/components/globals/dialogs/import_progress.rs`

**Step 1: Creare il dialog di progress import**

```rust
use super::base_modal::ModalVariant;
use crate::components::ImportProgressChn;
use crate::components::globals::WarningIcon;
use dioxus::prelude::*;

/// Dialog che mostra il progresso dell'import.
///
/// Non può essere chiuso durante l'import (on_close vuoto).
#[component]
pub fn ImportProgressDialog(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Signal che diventa true quando l'import è completato
    on_completed: Signal<bool>,

    /// Signal che diventa true se l'import fallisce
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
            h3 { class: "font-bold text-lg mb-2", "Importing Passwords" }

            // Messaggio
            p { class: "py-4",
                "Your passwords are being imported. Please wait..."
            }

            p { class: "text-warning-600 py-2",
                "The dialog will close automatically when the import is complete."
            }

            ImportProgressChn { on_completed, on_failed }
        }
    }
}
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/dialogs/import_progress.rs
git commit -m "feat(import): add ImportProgressDialog component"
```

---

## Task 5: Aggiornare mod.rs dialogs e features

**Files:**
- Modify: `src/components/globals/dialogs/mod.rs`
- Modify: `src/components/features/mod.rs` (già fatto nei task precedenti)

**Step 1: Aggiungere i nuovi moduli in dialogs/mod.rs**

In `src/components/globals/dialogs/mod.rs`, aggiungere:

```rust
mod import_progress;
mod import_warning;

pub use import_progress::*;
pub use import_warning::*;
```

**Step 2: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 3: Commit**

```bash
git add src/components/globals/dialogs/mod.rs
git commit -m "feat(import): export import dialogs from module"
```

---

## Task 6: Aggiornare DashboardMenu con Import

**Files:**
- Modify: `src/components/features/dashboard_menu.rs`

**Step 1: Aggiungere imports necessari**

All'inizio di `src/components/features/dashboard_menu.rs`, aggiungere agli use esistenti:

```rust
use crate::backend::export_types::ExportFormat;
use crate::backend::import::validate_import_path;
use crate::components::{ImportData, ImportWarningDialog, ImportProgressDialog};
use rfd::FileDialog;
```

**Step 2: Aggiungere signals per l'import**

Dentro `DashboardMenu`, dopo i signal esistenti, aggiungere:

```rust
// Import state
let mut import_warning_open = use_signal(|| false);
let mut import_progress_open = use_signal(|| false);
let mut import_completed = use_signal(|| false);
let mut import_failed = use_signal(|| false);
let mut import_data = use_signal(|| ImportData::default());
let mut import_format = use_signal(|| ExportFormat::Json);
```

**Step 3: Aggiungere funzione helper per aprire FileDialog**

Prima del rsx!, aggiungere:

```rust
// Funzione per aprire il dialog di selezione file per import
let open_import_dialog = move |format: ExportFormat| {
    let user_clone = user.clone();
    let toast = toast.clone();
    let mut import_data = import_data.clone();
    let mut import_format = import_format.clone();
    let mut import_warning_open = import_warning_open.clone();

    spawn(async move {
        if let Some(user) = user_clone {
            // Usa spawn_blocking per FileDialog
            let file_result = tokio::task::spawn_blocking(move || {
                FileDialog::new()
                    .add_filter("Import File", &[format.extension()])
                    .set_title(&format!("Import {} passwords", format.extension().to_uppercase()))
                    .pick_file()
            })
            .await;

            match file_result {
                Ok(Some(path)) => {
                    // Valida il path e rileva il formato
                    match validate_import_path(&path) {
                        Ok(detected_format) => {
                            import_data.set(ImportData::new(user.id, path, detected_format));
                            import_format.set(detected_format);
                            import_warning_open.set(true);
                        }
                        Err(e) => {
                            show_toast_error(format!("Invalid file: {}", e), toast);
                        }
                    }
                }
                Ok(None) => {
                    // Utente ha annullato
                    tracing::info!("Import cancelled by user");
                }
                Err(e) => {
                    show_toast_error(
                        format!("Error opening file dialog: {}", e),
                        toast,
                    );
                }
            }
        }
    });
};

// Handler per confermare l'import
let on_import_confirm = move |_| {
    import_warning_open.set(false);
    import_progress_open.set(true);
};

// Handler per chiudere il progress dopo completamento
use_effect(move || {
    if import_completed() || import_failed() {
        import_progress_open.set(false);
        // Trigger restart per ricaricare le password
        on_need_restart.set(true);
    }
});
```

**Step 4: Aggiornare i pulsanti Import nel menu**

Sostituire i button dell'Import submenu con:

```rust
// Import submenu
li {
    details {
        summary { class: "cursor-pointer", "Import" }
        ul {
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_import_dialog(ExportFormat::Json);
                    },
                    "JSON"
                }
            }
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_import_dialog(ExportFormat::Csv);
                    },
                    "CSV"
                }
            }
            li {
                button {
                    r#type: "button",
                    onclick: move |_| {
                        open_import_dialog(ExportFormat::Xml);
                    },
                    "XML"
                }
            }
        }
    }
}
```

**Step 5: Aggiungere i dialog prima della chiusura del componente**

Prima della chiusura del rsx!, aggiungere (dopo `AllStoredPasswordDeletionDialog`):

```rust
// Import warning dialog
ImportWarningDialog {
    open: import_warning_open,
    input_path: import_data.read().input_path.display().to_string(),
    format: format!("{:?}", import_format()),
    on_confirm: on_import_confirm,
    on_cancel: move |_| {
        import_warning_open.set(false);
    },
}

// Import progress dialog
ImportProgressDialog {
    open: import_progress_open,
    on_completed: import_completed,
    on_failed: import_failed,
}
```

**Step 6: Verificare che compili**

Run: `cargo check`
Expected: Nessun errore

**Step 7: Commit**

```bash
git add src/components/features/dashboard_menu.rs
git commit -m "feat(import): integrate import functionality in dashboard menu"
```

---

## Task 7: Test Manuale End-to-End

**Step 1: Avviare l'applicazione**

Run: `dx serve --desktop`

**Step 2: Testare il flusso completo**

1. Login con utente esistente
2. Aprire dashboard menu (tre puntini)
3. Cliccare Import → JSON
4. Verificare che si apra FileDialog
5. Selezionare un file JSON valido
6. Verificare che si apra ImportWarningDialog con info su duplicati
7. Cliccare Import
8. Verificare che si apra ImportProgressDialog con progress bar
9. Verificare toast di successo con conteggio importate/saltate
10. Verificare che la dashboard si ricarichi con le nuove password

**Step 3: Testare edge cases**

1. Provare a selezionare file con estensione non supportata → toast errore
2. Provare file JSON malformato → toast errore
3. Provare file vuoto → toast con 0 importate
4. Cancellare FileDialog → nessuna azione
5. Cliccare Cancel nel warning → nessuna azione
6. Importare file con duplicati → verificare conteggio skipped

**Step 4: Testare duplicati e nuove entry**

1. Creare password con location "test.com", password "pass123", notes "original"
2. Creare file JSON con:
   - location "test.com", password "pass123", notes "imported" (stessa location+password)
   - location "test.com", password "different", notes "new entry" (stessa location, password diversa)
3. Importare il file
4. Verificare che:
   - La prima password sia SALTATA (stessa location+password → nota originale preservata)
   - La seconda password sia IMPORTATA come nuova voce
5. Verificare conteggio toast: 1 imported, 1 skipped (la prima è duplicato nel DB)

**Step 5: Commit finale**

```bash
git add -A
git commit -m "feat(import): complete import frontend integration"
```

---

## Note Tecniche

### Flusso Dati Completo

```
User Click Import → FileDialog (pick_file)
                      ↓
               validate_import_path (rileva formato)
                      ↓
               ImportData creato
                      ↓
            ImportWarningDialog (conferma con warning)
                      ↓
            ImportProgressDialog
                      ↓
         ImportProgressChn (use_effect auto-start)
                      ↓
    import_passwords_pipeline_with_progress
                      ↓
         ProgressMessage via mpsc channel
                      ↓
         UI aggiorna progress bar
                      ↓
         Toast success (imported count + skipped count)
                      ↓
         on_need_restart.set(true) → dashboard refresh
```

### Dipendenze già presenti

- `rfd` - per FileDialog nativo
- `pwd-dioxus` - per toast (`show_toast_error`, `show_toast_success`)
- `tokio::sync::mpsc` - per progress channel
- Tipi backend già esistenti in `import.rs` e `export_types.rs`
- `validate_import_path` - già esistente in `import.rs`

### Pattern riutilizzati

- `MigrationWarningDialog` → `ImportWarningDialog`
- `MigrationProgressDialog` → `ImportProgressDialog`
- `ProgressMigrationChn` → `ImportProgressChn`
- `MigrationData` → `ImportData`
- `spawn_blocking` per FileDialog (da `ui_utils.rs`)
- `WarningIcon` per icone (già esistente nel progetto)

### Differenze chiave rispetto all'Export

| Aspect | Export | Import |
|--------|--------|--------|
| FileDialog | `save_file()` | `pick_file()` |
| Validation | Nessuna (l'utente sceglie dove salvare) | `validate_import_path()` rileva formato |
| Warning | Password in chiaro nel file | Info su comportamento duplicati |
| Toast success | Path del file salvato | Count importate +count saltate |
| Post-action | Nessuna | `on_need_restart.set(true)` per refresh |

### Comportamento Import

L'import ha due livelli di deduplicazione:
1. **Nel file**: Se il file ha password duplicate per (location, password), mantiene la prima
2. **Nel DB**: Se una password (location, password) esiste già nel DB, viene saltata

**NOTA IMPORTANTE**: Il confronto è su **(location + password)**, non solo location!
- Se (location, password) esiste già → SALTATA (non importata, non sovrascritta)
- Se solo location esiste con password diversa → IMPORTATA come nuova voce

Il conteggio `skipped_duplicates` include entrambi i tipi (duplicati nel file + duplicati nel DB).

### Upsert Behavior

L'import usa `upsert_stored_passwords_batch` ma tutte le password hanno `id: None` (vedi `ExportablePassword::to_stored_raw()`), quindi è sempre un INSERT, mai un REPLACE. Le password esistenti vengono preventivamente filtrate e saltate.
