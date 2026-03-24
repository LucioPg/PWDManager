# NSIS Custom Template - PWDManager

## Panoramica

PWDManager usa un template NSIS personalizzato per aggiungere due pagine custom con **nsDialogs** al flusso dell'installer:

1. **Disclaimer** — pagina prima dell'installazione con testo informativo e pulsanti Accept/Decline
2. **Recovery Key** — pagina dopo l'installazione che mostra la recovery key in una textbox selezionabile

## Flusso pagine

```
Welcome → License → InstallMode → Reinstall(nsDialogs) → Directory → StartMenu
→ DISCLAIMER(nsDialogs) → INSTFILES(Section + hooks) → RECOVERY KEY(nsDialogs) → Finish
```

## File coinvolti

| File | Ruolo |
|------|-------|
| `installer/custom-installer.nsi` | Template NSIS personalizzato (copia del template tauri-bundler 2.5.0) |
| `installer/nsis-hooks.nsh` | Hook NSIS eseguiti dentro `Section Install` |
| `Dioxus.toml` | Configurazione che punta al template e agli hooks |

## Configurazione Dioxus.toml

```toml
[bundle.windows.nsis]
template = "installer/custom-installer.nsi"
installer_hooks = "installer/nsis-hooks.nsh"
```

## Perché un template custom

nsDialogs (`nsDialogs::Create 1018`) **funziona solo all'interno delle page functions** (dichiarate con `Page custom`), non dentro le `Section`. Gli hook di tauri-bundler (`NSIS_HOOK_PREINSTALL`/`POSTINSTALL`) vengono espansi dentro `Section Install`, dove il template dialog 1018 non è disponibile. Pertanto, le pagine nsDialogs devono essere dichiarate come `Page custom` nel template, non negli hook.

## Bug in dioxus-cli 0.7.3

Il campo `template` in `[bundle.windows.nsis]` viene letto correttamente da `Dioxus.toml` ma **scartato** nella conversione da `dioxus_cli::NsisSettings` a `tauri_bundler::NsisSettings`.

**File affetto:** `~/.cargo/registry/src/index.crates.io-{hash}/dioxus-cli-0.7.3/src/bundle_utils.rs`

**Riga 16:**
```rust
// Prima della fix:
template: None,

// Dopo la fix:
template: val.template,
```

**Procedura per applicare la fix:**

1. Patchare il file nella cargo registry
2. Ricompilare e reinstallare dioxus-cli:
   ```bash
   cargo install --path "C:\Users\Lucio\.cargo\registry\src\index.crates.io-{hash}\dioxus-cli-0.7.3" --force
   ```
3. La registry path cambia ad ogni aggiornamento versione — verificare il percorso con:
   ```bash
   cargo install --list | grep dioxus
   find ~/.cargo/registry/src -name "bundle_utils.rs" -path "*/dioxus-cli-*"
   ```

**La fix va riapplicata dopo ogni `cargo install dioxus-cli` o aggiornamento versione.**

## Dettaglio nsis-hooks.nsh

Gli hook sono espansi dentro `Section Install` del template. Non possono usare nsDialogs.

### NSIS_HOOK_PREINSTALL

Vuoto — la UI del disclaimer è gestita dalla pagina custom `PageDisclaimer` nel template.

### NSIS_HOOK_POSTINSTALL

1. Esegue `PWDManager.exe --setup` via `nsExec::ExecToStack`
2. Se exit code != 0, setta `$SetupFailed = "1"` e chiama `Abort`
3. Salva l'output (recovery key) nella variabile globale `$RecoveryKey`

Le variabili globali `$SetupFailed`, `$RecoveryKey` e `$DisclaimerAccepted` sono dichiarate nel template con `Var` (non negli hook).

## Dettaglio pagine nsDialogs

### PageDisclaimer

- **Header:** "Disclaimer" / "Read and accept before installing"
- **Contenuto:** 4 label statici con testo informativo
- **Pulsanti:** Accept and Continue | Decline
- **Comportamento:**
  - Accept → setta `$DisclaimerAccepted = "1"`, invia `WM_COMMAND` per avanzare
  - Decline → setta `$DisclaimerAccepted = "0"`, invia `WM_COMMAND`
  - `PageLeaveDisclaimer`: se `$DisclaimerAccepted == "0"` → `Quit`
- **Passive mode:** saltata via `SkipIfPassive`

### PageRecoveryKey

- **Header:** "Recovery Key" / "Save your recovery key"
- **Contenuto:** Label + TextBox (`${NSD_CreateText}`) con `$RecoveryKey`
- **Pulsante:** "OK - I have saved my recovery key"
- **Comportamento:**
  - Se `$SetupFailed = 1` o `$RecoveryKey == ""` → `Abort` (pagina saltata)
  - `PageLeaveRecoveryKey`: salva la key in `$INSTDIR\recovery_key.txt`
- La TextBox è selezionabile — l'utente può copiare con Ctrl+C

## Coordinate nsDialogs

L'area contenuto del dialog 1018 in NSIS è circa **200 unità** di altezza. Le coordinate sono in **dialog units**.

```
Disclaimer page layout:
  y=0   : Title label (h=14u)
  y=18u : Text 1 (h=24u)
  y=46u : Text 2 (h=24u)
  y=74u : Text 3 (h=24u)
  y=110u: Buttons (h=14u)

Recovery Key page layout:
  y=0   : Title label (h=14u)
  y=18u : Instructions (h=20u)
  y=40u : TextBox (h=40u)
  y=95u : Button (h=14u)
```

**Nota importante:** Le macro `${NSD_CreateLabel}`, `${NSD_CreateButton}`, `${NSD_CreateText}` prendono come parametri `X Y Width Height` — **non un HWND**. Non passare il result di `nsDialogs::Create` come primo parametro.

## Mantenimento del template

Il template è una copia del template tauri-bundler 2.5.0 (`tauri-bundler-2.5.0/src/bundle/windows/nsis/installer.nsi`). Per aggiornarlo:

1. Copiare il nuovo template da `~/.cargo/registry/src/.../tauri-bundler-{version}/src/bundle/windows/nsis/installer.nsi`
2. Riapplicare le modifiche PWDManager:
   - Aggiungere `!include "nsDialogs.nsh"` dopo `!include MUI2.nsh`
   - Aggiungere le `Var` globali (`DisclaimerAccepted`, `RecoveryKey`, `SetupFailed`)
   - Aggiungere `!include "{{installer_hooks}}"` (già presente nel template)
   - Inserire `Page custom PageDisclaimer PageLeaveDisclaimer` tra StartMenu e INSTFILES
   - Inserire `Page custom PageRecoveryKey PageLeaveRecoveryKey` tra INSTFILES e Finish
   - Aggiungere le Function definitions alla fine del file
3. Verificare con `dx bundle --desktop --release`
