# Auto-Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementare l'auto-update per PWDManager che controlla GitHub Releases, scarica l'aggiornamento firmato con minisign, e lancia il NSIS installer silenzioso.

**Architecture:** Separazione trigger/consumer: `AuthWrapper` triggera il check quando `AutoUpdate(true)` dal DB, scrive su `Signal<UpdateState>`. `App()` crea il Signal, fornisce context, e renderizza `UpdateNotification` come overlay fisso. Il modulo backend `updater.rs` contiene logica pure async per check, download, verifica firma e installazione.

**Tech Stack:** Rust, Dioxus 0.7.3, reqwest 0.12, semver 1, zip 2, minisign-verify 0.2, base64 0.22, futures 0.3, tokio 1, DaisyUI 5

**Spec:** `docs/superpowers/specs/2026-03-19-auto-updater-design.md`

---

## File Structure

| File | Responsabilità |
|------|---------------|
| `src/backend/updater_types.rs` | Struct per deserializzazione `latest.json` + enum stati update |
| `src/backend/updater.rs` | Logica async: check, download, verify, install |
| `src/components/features/update_notification.rs` | Componente UI overlay per notifica e progress |
| `keys/update-public.key` | Public key minisign (committato, non secret) |
| `.env` | Private key firma (NON committato), usata dallo script di build |
| `scripts/build-updater-artifacts.sh` | Script custom: firma artefatto NSIS + genera `latest.json` |

---

### Task 1: Dipendenze e Configurazione Build

**Files:**
- Modify: `Cargo.toml:14-47` (dependencies section)
- Modify: `Dioxus.toml:8` (version)
- Modify: `.gitignore` (append at end of file)
- Create: `.env`
- Create: `keys/update-public.key`
- Create: `scripts/build-updater-artifacts.sh`

- [ ] **Step 1: Aggiungere dipendenze runtime in `Cargo.toml`**

Aggiungere dopo la riga 47 (`chrono = "0.4"`):

```toml
# Updater
reqwest = { version = "0.12", features = ["json", "stream"] }
semver = "1"
zip = "2"
minisign-verify = "0.2"
base64 = "0.22"
futures = "0.3"
```

- [ ] **Step 2: Sync versione in `Dioxus.toml`**

Cambiare riga 8 da `version = "0.1.0"` a `version = "0.2.0"`.

- [ ] **Step 3: Aggiungere `.env` a `.gitignore`**

Aggiungere alla fine del file `.gitignore`:

```gitignore
# Chiavi di firma updater (MAI committare)
.env
```

- [ ] **Step 4: Creare file `.env` placeholder**

Creare `.env` nella root del progetto:

```env
# Chiave privata per firmare gli updater artifacts
# Genera con: minisign -G -p keys/update-public.key -s keys/pwdmanager.key
# (oppure installa minisign da https://jedisct1.github.io/minisign/)
# Queste variabili sono lette dallo script scripts/build-updater-artifacts.sh
DIOXUS_SIGNING_PRIVATE_KEY=
DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD=
```

- [ ] **Step 5: Creare file placeholder per la public key**

Creare `keys/update-public.key`:

```
# La public key minisign va qui.
# Genera la coppia di chiavi e copia la public key in questo file.
# La private key va nel file .env (non committato).
```

- [ ] **Step 6: Creare lo script custom per generare updater artifacts**

Creare `scripts/build-updater-artifacts.sh`:

```bash
#!/usr/bin/env bash
# build-updater-artifacts.sh
# Firma l'artefatto NSIS e genera latest.json per l'auto-update.
# Uso: ./scripts/build-updater-artifacts.sh <versione> <cartella_bundle_output>
#
# Prerequisiti:
#   - minisign installato (https://jedisct1.github.io/minisign/)
#   - .env nella root con DIOXUS_SIGNING_PRIVATE_KEY e DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD
#   - Bundle NSIS gia compilato con: dx bundle --desktop --package-types "nsis" --release

set -euo pipefail

VERSION="${1:?Usage: $0 <version> <bundle_output_dir>}"
BUNDLE_DIR="${2:?Usage: $0 <version> <bundle_output_dir>}"

# Carica variabili d'ambiente dal .env
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ -f "$PROJECT_ROOT/.env" ]; then
    set -a
    source "$PROJECT_ROOT/.env"
    set +a
else
    echo "ERROR: .env not found at $PROJECT_ROOT/.env"
    exit 1
fi

if [ -z "$DIOXUS_SIGNING_PRIVATE_KEY" ]; then
    echo "ERROR: DIOXUS_SIGNING_PRIVATE_KEY not set in .env"
    exit 1
fi

# Trova il file .exe NSIS nella cartella di output
NSIS_EXE=$(find "$BUNDLE_DIR" -name "*.exe" -path "*nsis*" | head -1)
if [ -z "$NSIS_EXE" ]; then
    echo "ERROR: No NSIS .exe found in $BUNDLE_DIR"
    exit 1
fi

echo "==> Found NSIS installer: $NSIS_EXE"

# Firma l'artefatto con minisign
SIG_FILE="${NSIS_EXE}.sig"
echo "==> Signing artifact..."
if [ -n "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
    echo "$DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD" | minisign \
        -Sm "$NSIS_EXE" \
        -s - \
        -t "PWDManager v$VERSION" \
        -x "$SIG_FILE"
else
    minisign -Sm "$NSIS_EXE" -t "PWDManager v$VERSION" -x "$SIG_FILE"
fi

# Crea lo zip per l'update (contiene solo l'installer .exe)
NSIS_ZIP="${NSIS_EXE%.*}.nsis.zip"
echo "==> Creating update zip: $NSIS_ZIP"
cp "$NSIS_EXE" "$(basename "$NSIS_EXE")"
zip -j "$NSIS_ZIP" "$(basename "$NSIS_EXE")"
rm "$(basename "$NSIS_EXE")"

# Legge la firma e la converte in base64 per latest.json
SIGNATURE_B64=$(base64 -w 0 "$SIG_FILE")

# Determina il nome file dell'exe per l'URL
EXE_BASENAME=$(basename "$NSIS_EXE")
ZIP_BASENAME=$(basename "$NSIS_ZIP")

# Genera latest.json
NOTES_FILE="$PROJECT_ROOT/RELEASE_NOTES.md"
NOTES="Release v$VERSION"
if [ -f "$NOTES_FILE" ]; then
    NOTES=$(cat "$NOTES_FILE")
fi

PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "$BUNDLE_DIR/latest.json" <<EOF
{
  "version": "$VERSION",
  "notes": $(echo "$NOTES" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read().strip()))'),
  "pub_date": "$PUB_DATE",
  "platforms": {
    "windows-x86_64": {
      "signature": "$SIGNATURE_B64",
      "url": "https://github.com/LucioPg/PWDManager/releases/download/v$VERSION/$ZIP_BASENAME"
    }
  }
}
EOF

echo "==> Generated $BUNDLE_DIR/latest.json"
echo ""
echo "=== Artifacts ready for release ==="
echo "  Installer: $NSIS_EXE"
echo "  Signature: $SIG_FILE"
echo "  Update zip: $NSIS_ZIP"
echo "  Manifest: $BUNDLE_DIR/latest.json"
echo ""
echo "Upload these to GitHub Release v$VERSION:"
echo "  gh release create v$VERSION --title \"v$VERSION\" --notes-file \"$NOTES_FILE\" \\"
echo "    \"$NSIS_ZIP\" \"$BUNDLE_DIR/latest.json\""
```

- [ ] **Step 7: Verificare compilazione**

Run: `cargo check`
Expected: Compilazione riuscita (le nuove dipendenze vengono scaricate)

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock Dioxus.toml .gitignore keys/update-public.key scripts/build-updater-artifacts.sh
git commit -m "chore: add updater dependencies, signing keys config, custom build script, sync bundle version"
```

---

### Task 2: Modulo Backend - Tipi

**Files:**
- Create: `src/backend/updater_types.rs`
- Modify: `src/backend/mod.rs:1`

- [ ] **Step 1: Creare `src/backend/updater_types.rs`**

```rust
use serde::Deserialize;
use std::collections::HashMap;

/// Struttura deserializzata da latest.json generato dal bundler Tauri.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: HashMap<String, PlatformInfo>,
}

#[derive(Debug, Deserialize)]
pub struct PlatformInfo {
    pub signature: String,
    pub url: String,
}

/// Stato dell'aggiornamento, guidato dalla macchina a stati.
/// Usato come Signal<UpdateState> per il componente UI.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available { version: String, notes: String },
    Downloading { progress: u8 },
    Installing,
    UpToDate,
    Error(String),
}
```

- [ ] **Step 2: Registrare il modulo in `src/backend/mod.rs`**

Aggiungere dopo la riga 12 (`pub mod avatar_utils;`):

```rust
pub mod updater_types;
pub mod updater;
```

- [ ] **Step 3: Verificare compilazione**

Run: `cargo check`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add src/backend/updater_types.rs src/backend/mod.rs
git commit -m "feat: add updater types (UpdateManifest, UpdateState)"
```

---

### Task 3: Modulo Backend - Logica Updater

**Files:**
- Create: `src/backend/updater.rs`

- [ ] **Step 1: Creare `src/backend/updater.rs`**

```rust
use crate::backend::updater_types::{UpdateManifest, UpdateState};
use futures::stream::StreamExt;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const UPDATE_ENDPOINT: &str =
    "https://github.com/LucioPg/PWDManager/releases/latest/download/latest.json";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Public key minisign per verificare la firma degli aggiornamenti.
/// Il file contiene il formato standard minisign (untrusted comment + base64).
/// Generata con: minisign -G -p keys/update-public.key -s keys/pwdmanager.key
const PUBLIC_KEY: &str = include_str!("../../keys/update-public.key");

/// Controlla se esiste un aggiornamento disponibile confrontando
/// la versione corrente con quella nel latest.json.
pub async fn check_for_update() -> Result<Option<UpdateManifest>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(UPDATE_ENDPOINT)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let manifest: UpdateManifest = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let current = Version::parse(CURRENT_VERSION)
        .map_err(|e| format!("Invalid current version: {}", e))?;
    let available = Version::parse(&manifest.version.trim_start_matches('v'))
        .map_err(|e| format!("Invalid available version: {}", e))?;

    if available > current {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

/// Verifica la firma minisign di un file scaricato.
///
/// Il campo `signature` in latest.json è il contenuto **base64-encoded** del file .sig
/// generato da minisign (formato multi-riga: untrusted comment + firma + trusted comment).
/// Quindi servono due passaggi: base64-decode → Signature::decode().
fn verify_update_signature(
    signature_b64: &str,
    file_path: &Path,
) -> Result<(), String> {
    // PublicKey::decode() accetta il formato file completo (con riga "untrusted comment")
    let pk = PublicKey::decode(PUBLIC_KEY)
        .map_err(|e| format!("Invalid public key: {}", e))?;

    // Il campo signature in latest.json è base64-encoded: decodifichiamo prima
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("Invalid signature base64: {}", e))?;
    let sig_text = String::from_utf8(sig_bytes)
        .map_err(|e| format!("Signature not valid UTF-8: {}", e))?;
    let sig = Signature::decode(&sig_text)
        .map_err(|e| format!("Invalid signature format: {}", e))?;

    // PublicKey::verify richiede i bytes del file, non il path
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("Cannot read file for verification: {}", e))?;

    // Terzo parametro: allow_legacy = false (solo firme moderne)
    pk.verify(&file_bytes, &sig, false)
        .map_err(|e| format!("Signature verification failed: {}", e))?;

    Ok(())
}

/// Scarica l'aggiornamento, verifica la firma, estrae e lancia l'installer NSIS.
/// Aggiorna `update_state` con il progress durante il download.
pub async fn download_and_install(
    manifest: &UpdateManifest,
    update_state: dioxus::prelude::Signal<UpdateState>,
) -> Result<(), String> {
    // Determina la piattaforma - per ora solo Windows
    let platform_key = "windows-x86_64";

    let platform_info = manifest
        .platforms
        .get(platform_key)
        .ok_or_else(|| format!("No update for platform: {}", platform_key))?;

    let temp_dir = std::env::temp_dir().join("pwdmanager_update");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Cannot create temp dir: {}", e))?;

    let archive_path = temp_dir.join("update.nsis.zip");

    // Download con progress — usa bytes_stream() + StreamExt per chunk-by-chunk
    let client = reqwest::Client::new();
    let response = client
        .get(&platform_info.url)
        .send()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&archive_path)
        .await
        .map_err(|e| format!("Cannot create file: {}", e))?;

    let mut stream = response.bytes_stream();
    while let Some(result) = stream.next().await {
        let chunk = result.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let pct = (downloaded as f64 / total_size as f64 * 100.0) as u8;
            update_state.set(UpdateState::Downloading { progress: pct.min(95) });
        }
    }
    drop(file);

    update_state.set(UpdateState::Downloading { progress: 96 });

    // Verifica firma sul file zip prima dell'estrazione
    // signature è base64-encoded (contenuto del file .sig di minisign)
    verify_update_signature(&platform_info.signature, &archive_path)?;

    update_state.set(UpdateState::Downloading { progress: 98 });

    // Estrai lo zip
    let extract_dir = temp_dir.join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Cannot create extract dir: {}", e))?;

    let zip_file = std::fs::File::open(&archive_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Cannot read zip: {}", e))?;
    archive
        .extract(&extract_dir)
        .map_err(|e| format!("Extract error: {}", e))?;

    // Trova il file .exe nell'archivio estratto
    let installer = find_exe_in_dir(&extract_dir)?;

    // Lancia l'installer NSIS in modalita silenziosa
    update_state.set(UpdateState::Installing);
    std::process::Command::new(&installer)
        .arg("/S")
        .spawn()
        .map_err(|e| format!("Cannot launch installer: {}", e))?;

    Ok(())
}

/// Cerca il primo file .exe nella directory estratta.
fn find_exe_in_dir(dir: &Path) -> Result<PathBuf, String> {
    std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read extract dir: {}", e))?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        })
        .map(|entry| entry.path())
        .ok_or_else(|| "No .exe installer found in archive".to_string())
}
```

- [ ] **Step 2: Verificare compilazione**

Run: `cargo check`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add src/backend/updater.rs
git commit -m "feat: add updater backend logic (check, download, verify, install)"
```

---

### Task 4: CSS per UpdateNotification

**Files:**
- Modify: `assets/input_main.css`

- [ ] **Step 1: Aggiungere classi `pwd-update-*` in `input_main.css`**

Aggiungere dopo la sezione TOAST NOTIFICATIONS (dopo `.pwd-toast-info` alla riga ~851, prima della sezione `/* Auth forms */` alla riga ~854):

```css
/* ============================================================
   UPDATE NOTIFICATION (overlay fisso)
   ============================================================ */
@utility pwd-update-overlay {
    @apply fixed top-4 right-4 z-50 w-96 max-w-[calc(100vw-2rem)];
}

.pwd-update-card {
    @apply rounded-xl shadow-xl border p-5 backdrop-blur-sm;
    background-color: rgba(255, 255, 255, 0.95);
    border-color: var(--color-base-300);
    color: var(--color-base-content);
}

.pwd-update-title {
    @apply text-base font-bold;
}

.pwd-update-version {
    @apply text-sm text-base-content/70 mt-1;
}

.pwd-update-changelog {
    @apply text-xs text-base-content/60 mt-2 max-h-32 overflow-y-auto leading-relaxed;
}

.pwd-update-progress-bar {
    @apply w-full bg-base-300 rounded-full h-2.5 mt-3;
}

.pwd-update-progress-fill {
    @apply h-2.5 rounded-full bg-primary transition-all duration-300;
}

.pwd-update-actions {
    @apply flex gap-2 mt-4;
}

.pwd-update-error-text {
    @apply text-sm text-error mt-2;
}

.pwd-update-spinner {
    @apply flex items-center gap-3;
}
```

- [ ] **Step 2: Aggiungere dark theme overrides**

Aggiungere nel blocco `[data-theme="dark"]` (riga 1943), dopo la sezione Navbar:

```css
    /* ---- Update Notification ---- */

    .pwd-update-card {
        background-color: rgba(30, 30, 46, 0.95);
        border-color: var(--color-base-300);
        color: var(--color-base-content);
    }
```

- [ ] **Step 3: Commit**

```bash
git add assets/input_main.css
git commit -m "style: add update notification CSS classes (pwd-update-*)"
```

---

### Task 5: Componente UI UpdateNotification

**Files:**
- Create: `src/components/features/update_notification.rs`
- Modify: `src/components/features/mod.rs:1-31`

- [ ] **Step 1: Creare `src/components/features/update_notification.rs`**

```rust
use crate::backend::updater_types::UpdateState;
use crate::components::{Spinner, SpinnerSize};
use dioxus::prelude::*;

#[component]
pub fn UpdateNotification(update_state: Signal<UpdateState>) -> Element {
    let state = update_state.read();

    match &*state {
        UpdateState::Idle | UpdateState::UpToDate => None,
        UpdateState::Checking => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner { size: SpinnerSize::Medium, color_class: "text-primary" }
                        span { class: "pwd-update-version", "Verifica aggiornamenti..." }
                    }
                }
            }
        },
        UpdateState::Available { version, notes } => {
            let version = version.clone();
            let notes = notes.clone();
            let update_state_avail = update_state.clone();
            let update_state_dismiss = update_state.clone();
            let manifest = notes.clone();
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        // Icona aggiornamento (freccia circolare)
                        svg {
                            class: "w-10 h-10 text-primary shrink-0",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path { d: "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" }
                            path { d: "M21 3v5h-5" }
                        }
                        div { class: "flex-1 min-w-0",
                            h3 { class: "pwd-update-title", "Aggiornamento disponibile!" }
                            p { class: "pwd-update-version", "Versione {version}" }
                            if !manifest.is_empty() {
                                p { class: "pwd-update-changelog",
                                    dangerous_inner_html: "{manifest}"
                                }
                            }
                        }
                        div { class: "pwd-update-actions",
                            button {
                                class: "btn btn-primary btn-sm",
                                onclick: move |_| {
                                    // TODO(human): il manifest deve essere passato a download_and_install
                                    // per ora salviamo lo stato come placeholder
                                    update_state_avail.set(UpdateState::Downloading { progress: 0 });
                                },
                                "Aggiorna ora"
                            }
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| update_state_dismiss.set(UpdateState::Idle),
                                "Più tardi"
                            }
                        }
                    }
                }
            }
        },
        UpdateState::Downloading { progress } => {
            let progress_val = *progress;
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-title", "Download aggiornamento..." }
                        div { class: "pwd-update-progress-bar",
                            div {
                                class: "pwd-update-progress-fill",
                                style: "width: {progress_val}%",
                            }
                        }
                        p { class: "pwd-update-version mt-2", "{progress_val}%" }
                    }
                }
            }
        },
        UpdateState::Installing => rsx! {
            div { class: "pwd-update-overlay",
                div { class: "pwd-update-card",
                    div { class: "pwd-update-spinner",
                        Spinner { size: SpinnerSize::Medium, color_class: "text-primary" }
                        span { class: "pwd-update-version", "Installazione in corso, l'app si riavviera..." }
                    }
                }
            }
        },
        UpdateState::Error(e) => {
            let error_msg = e.clone();
            let update_state_err = update_state.clone();
            rsx! {
                div { class: "pwd-update-overlay",
                    div { class: "pwd-update-card",
                        p { class: "pwd-update-error-text", "Errore aggiornamento: {error_msg}" }
                        div { class: "pwd-update-actions",
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| update_state_err.set(UpdateState::Idle),
                                "Chiudi"
                            }
                        }
                    }
                }
            }
        },
    }
}
```

- [ ] **Step 2: Registrare il modulo in `src/components/features/mod.rs`**

Aggiungere dopo `mod import_progress;` (riga 8):

```rust
mod update_notification;
```

E aggiungere dopo `pub use import_progress::*;` (riga 24):

```rust
pub use update_notification::*;
```

- [ ] **Step 3: Verificare compilazione**

Run: `cargo check`
Expected: OK (il componente non è ancora usato, ma deve compilare)

- [ ] **Step 4: Commit**

```bash
git add src/components/features/update_notification.rs src/components/features/mod.rs
git commit -m "feat: add UpdateNotification component with all state variants"
```

---

### Task 6: Integrazione in App() e AuthWrapper

**Files:**
- Modify: `src/main.rs:9,42-44,135-139` (Signal, context, render)
- Modify: `src/components/globals/auth_wrapper.rs:1-54` (trigger check)

- [ ] **Step 1: Aggiungere import e Signal in `src/main.rs`**

Aggiungere import (riga 9, dopo `use crate::backend::settings_types::AutoUpdate;`):

```rust
use crate::backend::updater_types::UpdateState;
```

Aggiungere ai component imports (riga 12, dopo `ToastHubState`):

```rust
    UpdateNotification,
```

Dopo la riga 44 (`use_context_provider(|| auto_update);`), aggiungere:

```rust
    let mut update_state = use_signal(|| UpdateState::Idle);
    use_context_provider(|| update_state);
```

- [ ] **Step 2: Renderizzare `UpdateNotification` in `App()`**

Nello scope `Some(Ok(pool))` (riga 135-140), aggiungere `UpdateNotification` nel rsx:

```rust
        Some(Ok(pool)) => {
            // Se il pool è pronto, lo forniamo al resto dell'app
            use_context_provider(|| pool.clone());
            rsx! {
                // Carica il CSS di Tailwind globalmente
                Style {}
                ToastContainer {}
                UpdateNotification { update_state }
                Router::<Route> {}
            }
        }
```

- [ ] **Step 3: Aggiungere trigger check in `src/components/globals/auth_wrapper.rs`**

Aggiungere import (riga 4):

```rust
use crate::backend::updater::check_for_update;
use crate::backend::updater_types::UpdateState;
```

Aggiungere `use std::time::Duration;` agli import.

Dopo il blocco `use_resource` (riga 49), aggiungere `use_context` per `Signal<UpdateState>` a livello componente e un flag `update_check_started`:

```rust
    // Leggi Signal<UpdateState> fornito da App() — NON dentro use_effect!
    let mut update_state = use_context::<Signal<UpdateState>>();
    // Guardia: evita check multipli concorrenti
    let mut update_check_started = use_signal(|| false);
```

Poi aggiungere il `use_effect` che reagisce al cambio di `AutoUpdate`:

```rust
    // Trigger check aggiornamenti quando AutoUpdate viene letto dal DB
    use_effect(move || {
        let auto_update_enabled = *auto_update.read();
        if !auto_update_enabled || update_check_started() {
            return;
        }

        update_check_started.set(true);
        let update_state_clone = update_state.clone();

        spawn(async move {
            // Attendi 3 secondi dopo il login
            tokio::time::sleep(Duration::from_secs(3)).await;

            update_state_clone.set(UpdateState::Checking);

            match check_for_update().await {
                Ok(Some(manifest)) => {
                    let version = manifest.version.clone();
                    let notes = manifest.notes.clone();
                    // Salva il manifest per il download
                    update_state_clone.set(UpdateState::Available { version, notes });
                    // TODO(human): il manifest deve essere disponibile per il download
                }
                Ok(None) => {
                    update_state_clone.set(UpdateState::UpToDate);
                    // Auto-clear dopo 1 secondo come da spec
                    let state_for_clear = update_state_clone.clone();
                    spawn(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        state_for_clear.set(UpdateState::Idle);
                    });
                }
                Err(e) => {
                    update_state_clone.set(UpdateState::Error(e));
                }
            }
        });
    });
```

**Nota importante:** `use_context::<Signal<UpdateState>>()` va chiamato a livello componente (come `auto_update` e `app_theme`), MAI dentro una closure o `use_effect`. In Dioxus 0.7 gli hook devono essere chiamati al top-level del componente.

- [ ] **Step 4: Implementare il download nel pulsante "Aggiorna ora"**

In `update_notification.rs`, nel gestore `onclick` di "Aggiorna ora", il `TODO(human)` indica dove collegare il download. Il manifest deve essere accessibile per passarlo a `download_and_install()`.

**Questa e la parte che richiede la tua implementazione** — vedi sezione "Learn by Doing" sotto.

- [ ] **Step 5: Verificare compilazione**

Run: `cargo check`
Expected: OK

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/components/globals/auth_wrapper.rs src/components/features/update_notification.rs
git commit -m "feat: integrate updater trigger in AuthWrapper, render UpdateNotification in App"
```

---

### Task 7: Learn by Doing — Connessione Download Button

**Context:** Il componente `UpdateNotification` mostra un pulsante "Aggiorna ora" e l'`AuthWrapper` triggera il check. Ma c'e un problema: quando `check_for_update()` restituisce `Some(manifest)`, il manifest va salvato da qualche parte per poterlo passare a `download_and_install()` quando l'utente clicca "Aggiorna ora". Il `UpdateState` enum attualmente contiene solo `version` e `notes` (stringhe), non l'intero `UpdateManifest`.

**Your Task:** In `src/components/features/update_notification.rs`, implementa la logica nel gestore `onclick` del pulsante "Aggiorna ora" dove trovi il `TODO(human)`. Devi decidere come rendere il `UpdateManifest` disponibile per il download. Considera queste opzioni:

1. Aggiungere un campo `manifest: Option<UpdateManifest>` allo `UpdateState::Available` variant — richiede di rendere `UpdateManifest` cloneable
2. Usare un `Signal<Option<UpdateManifest>>` separato, salvato quando il check ha successo
3. Salvare il manifest in una `static OnceLock`

**Guidance:** L'opzione 2 e la piu semplice e non inquina i tipi esistenti. Ricorda che `download_and_install()` ha bisogno del manifest per ottenere URL e firma. Il `update_state` Signal e gia disponibile come prop del componente. Considera anche come gestire gli errori durante il download — devono aggiornare il `UpdateState` a `Error(msg)`.

- [ ] **Step 1: Implementare la logica di download nel pulsante**
- [ ] **Step 2: Verificare che `cargo check` passi**
- [ ] **Step 3: Commit**

```bash
git add src/components/features/update_notification.rs
git commit -m "feat: connect download button to updater backend"
```

---

### Task 8: Verifica Compilazione Finale e Cleanup

**Files:**
- Nessun nuovo file

- [ ] **Step 1: Build completo**

Run: `cargo check`
Expected: OK, zero warnings

- [ ] **Step 2: Verificare che `dx serve --desktop` funzioni**

Run: `dx serve --desktop`
Expected: App si avvia, login funziona, toggle auto-update nelle settings funziona

- [ ] **Step 3: Verificare versioni sincronizzate**

Run: `grep -E "^version" Cargo.toml Dioxus.toml`
Expected: Entrambi mostrano `0.2.0`

- [ ] **Step 4: Commit finale (se necessario)**

```bash
git add -A
git commit -m "chore: final cleanup for auto-updater feature"
```

---

## Post-Implementation (manuale, non nel piano)

Dopo aver completato tutti i task:

1. **Generare chiavi firma:** `minisign -G -p keys/update-public.key -s keys/pwdmanager.key`
   (installa minisign da https://jedisct1.github.io/minisign/ se non presente)
2. **Copiare public key** in `keys/update-public.key` (già fatto dal comando sopra)
3. **Copiare private key** nel file `.env` come `DIOXUS_SIGNING_PRIVATE_KEY`
4. **Compilare il bundle NSIS:** `dx bundle --desktop --package-types "nsis" --release`
5. **Generare updater artifacts:** `./scripts/build-updater-artifacts.sh v0.2.0 <cartella_bundle_output>`
   Questo firma l'artefatto, crea lo `.nsis.zip` e genera `latest.json`
6. **Creare release GitHub** con gli artefatti e `latest.json`:
   `gh release create v0.2.0 --title "v0.2.0" --notes-file RELEASE_NOTES.md <artefatti>`
