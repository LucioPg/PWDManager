# Auto-Updater Design

Data: 2026-03-19
Stato: Approved

## Obiettivo

Implementare l'auto-update per PWDManager (Dioxus 0.7.3 desktop) che controlla GitHub Releases, scarica l'aggiornamento
firmato, e lancia il NSIS installer silenzioso. L'utente vede una notifica overlay con changelog e progress bar.

## Scope

- **Piattaforma:** Solo Windows (`.nsis.zip`)
- **Verifica firma:** minisign (inclusa)
- **Installer:** NSIS silenzioso (`/S`)
- **Skip version:** Non supportato (l'utente puo solo chiudere temporaneamente)
- **Persistenza toggle:** Gia presente nel DB (`user_settings.auto_update`)
- **CI/CD:** Release manuale (no GitHub Actions)

## Architettura

### Separazione Trigger / Consumer

```
AuthWrapper (trigger)              App (consumer)
┌─────────────────────┐           ┌──────────────────────┐
│ Legge AutoUpdate    │           │ Crea Signal<         │
│ dal DB via pool     │           │   UpdateState>       │
│                     │           │                      │
│ Se AutoUpdate(true) │──scrive──▶│ use_effect legge     │
│ avvia check_for_    │  Signal   │ Signal<UpdateState>  │
│ update() e aggiorna │           │                      │
│ lo Signal           │           │ Renderizza           │
└─────────────────────┘           │ UpdateNotification   │
                                  │ (overlay fisso)      │
                                  └──────────────────────┘
```

- **`AuthWrapper`**: sa quando `AutoUpdate` e `true` dal DB, triggera il check e aggiorna `Signal<UpdateState>`
- **`App()`**: crea il Signal, renderizza il componente overlay che reagisce allo stato

### Flusso Dati

1. `App()` crea `Signal<UpdateState>` (default `Idle`) e lo fornisce come context
2. `AuthWrapper` legge `Signal<AutoUpdate>` dal DB, se `true` avvia `check_for_update()` dopo 3s
3. `check_for_update()` → GET `latest.json` → parse `UpdateManifest` → strip `v` prefix dalla versione → confronto
   `semver`
4. Se disponibile → setta `UpdateState::Available { version, notes }`
5. `UpdateNotification` (in `App()`) reagisce al cambio di stato e mostra l'overlay
6. Utente clicca "Aggiorna" → `download_and_install()` con callback progress
7. Download chunk-by-chunk → verifica firma sul `.nsis.zip` → estrazione zip → lancia NSIS `/S`
8. App chiude (l'installer sostituisce la versione)

### Macchina a Stati

```
Idle ──[auto_update=true + 3s]──> Checking
                                    │
                            ┌───────┴───────┐
                            ▼               ▼
                       Available       UpToDate
                            │               │
                      [utente clicca   (svanisce
                       "Aggiorna"]       dopo 1s)
                            │
                            ▼
                   Downloading { progress: 0..95 }
                            │
                      [download completato]
                            │
                            ▼
                   Installing (progress: 100)
                            │
                      [lancia NSIS /S + exit]
```

## Nuovi File

```
src/backend/updater.rs              # check_for_update(), download_and_install(), verify_signature()
src/backend/updater_types.rs        # UpdateManifest, PlatformInfo, UpdateState
src/components/features/update_notification.rs  # Componente Dioxus overlay
keys/update-public.key              # Public key minisign (committato)
.env                                # TAURI_SIGNING_PRIVATE_KEY + password (NON committato)
```

## File Modificati

| File                                     | Modifica                                                                                                                                                     |
|------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Cargo.toml`                             | Aggiunta dipendenze runtime + build-dependencies                                                                                                             |
| `Dioxus.toml`                            | Sync versione da `0.1.0` a `0.2.0` (per allinearsi a `Cargo.toml`). Nota: non esiste `create_updater_artifacts`, si usa `--package-types "updater"` al build |
| `build.rs`                               | Aggiunta `dotenvy::dotenv().ok()` dopo le direttive `cargo:rerun-if-changed` (per evitare rebuild su cambio `.env`)                                          |
| `src/main.rs`                            | Creazione `Signal<UpdateState>` + provide come context, render `UpdateNotification`                                                                          |
| `src/components/globals/auth_wrapper.rs` | `use_effect` per trigger check aggiornamenti                                                                                                                 |
| `assets/input_main.css`                  | Classi `pwd-update-*` per il componente overlay                                                                                                              |
| `.gitignore`                             | Aggiunta `.env`                                                                                                                                              |

## Dipendenze

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
semver = "1"
zip = "2"
minisign-verify = "0.2"

[build-dependencies]
dotenvy = "0.15"
```

## Tipi

### UpdateManifest (deserializzato da latest.json)

```rust
#[derive(Debug, Deserialize)]
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
```

### UpdateState (enum reattivo)

```rust
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

## Componente UI: UpdateNotification

- **Posizione:** Overlay fisso, alto a destra (`fixed top-4 right-4`)
- **Non e un toast:** Componente persistente con proprio stato, non auto-dismissable
- **Contenuto:**
    - `Checking` → spinner + testo
    - `Available` → icona, versione, changelog, pulsanti "Aggiorna ora" + "Più tardi"
    - `Downloading` → progress bar + percentuale
    - `Installing` → spinner + testo
    - `Error` → messaggio + pulsante "Chiudi"
    - `Idle` / `UpToDate` → nessun render
- **Stili:** Prefisso `pwd-update-*` in `input_main.css`, dark mode via `[data-theme="dark"]`

## Verifica Firma

- Public key letta con `include_str!("../../keys/update-public.key")`
- Verifica con crate `minisign-verify` sul file `.nsis.zip` scaricato, **prima** dell'estrazione
- La private key sta nel `.env` (NON committato), usata dal bundler Tauri durante il build

## Note Implementative

- **Platform key in `latest.json`**: La key usata per lookup nel HashMap (es. `"windows-x86_64"`) dipende da cosa
  `tauri-bundler` genera con `--package-types "updater"`. Da verificare dopo il primo build.
- **Exit dopo NSIS**: L'app deve chiudersi dopo aver lanciato il NSIS installer. `std::process::exit()` funziona ma
  bypassa i cleanup di Dioxus. Valutare se usare `std::process::Command::new().spawn()` e poi uscire.

## Endpoint GitHub

```
https://github.com/LucioPg/PWDManager/releases/latest/download/latest.json
```

## Release Manuale

1. Aggiornare versione in `Cargo.toml` e `Dioxus.toml` (devono essere in sync)
2. Assicurarsi che `.env` contenga le chiavi firma
3. Build con updater artifacts: `dx bundle --desktop --package-types "nsis" --package-types "updater" --release`
4. `git tag v0.x.y && git push origin v0.x.y`
5. `gh release create v0.x.y --title "..." --notes "..." <artefatti>`
6. Verificare che `latest.json` sia raggiungibile

> **Nota:** `create_updater_artifacts = true` non esiste in `Dioxus.toml`. Dioxus 0.7 supporta `PackageType::Updater`
> che si attiva via flag `--package-types "updater"` nel comando `dx bundle`. Questo genera il file `.nsis.zip` e
`latest.json` necessari per l'auto-update.

## Cose NON incluse (YAGNI)

- Skip versione permanente (l'utente non puo ignorare un aggiornamento)
- Check periodico (solo al login)
- Persistenza stato update nel DB (scompare con l'aggiornamento stesso)
- Supporto Linux (da aggiungere in futuro se necessario)
- CI/CD automatico (release manuale)
