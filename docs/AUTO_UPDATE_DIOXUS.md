# Auto-Update in Dioxus 0.7.3 (Desktop)

How the auto-update system works in PWDManager, a Dioxus 0.7.3 desktop application. Dioxus uses Tauri 2.x internally for the desktop renderer, but does not expose the Tauri plugin system. The updater is therefore implemented directly in Rust using `reqwest`, `minisign-verify`, `zip`, and `semver`, without any dependency on `tauri-plugin-updater`.

## Architecture

```
GitHub Releases (same repo)
  - latest.json (version, notes, platform URLs, base64-encoded minisign signature)
  - .nsis.zip (signed installer archive)
        |
        |  GET /releases/latest/download/latest.json
        v
Updater module (src/backend/updater.rs)
  - Fetch and parse latest.json
  - Compare versions (semver)
  - Download with streaming + progress
  - Verify minisign signature against embedded public key
  - Extract zip and launch NSIS installer (/S silent mode)
        |
        |  Signal<UpdateState>
        v
Update UI (src/components/features/update_notification.rs)
  - Checking spinner
  - Available notification (version, date, changelog, update/dismiss buttons)
  - Download progress bar
  - Installing spinner
  - Error display
```

## Files

```
src/backend/updater.rs            Check, download, verify, install logic
src/backend/updater_types.rs      UpdateManifest, PlatformInfo, UpdateState
src/components/features/update_notification.rs  UI component
src/components/globals/auth_wrapper.rs          Triggers update check after login
keys/update-public.key           Embedded minisign public key
scripts/build-updater-artifacts.sh  Release script: sign + generate latest.json
```

## Dependencies

```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
semver = "1"
zip = "2"
minisign-verify = "0.2"
base64 = "0.22.1"
```

The `reqwest` `stream` feature is required for chunk-by-chunk download progress reporting.

## Types

### UpdateManifest

Deserialized from `latest.json`. The `version` field may have a `v` prefix (e.g. `"v0.3.0"`). The updater trims it before semver comparison.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: HashMap<String, PlatformInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformInfo {
    pub signature: String,   // base64-encoded minisign signature file
    pub url: String,
}
```

### UpdateState

Driven by a `Signal<UpdateState>` provided via Dioxus context from `App()`. The UI component matches on this signal to render the appropriate state.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available { version: String, notes: String, pub_date: String },
    Downloading { progress: u8 },
    Installing,
    UpToDate,
    Error(String),
}
```

## State Machine

```
Idle ──> Checking
           |
      +────+────+
      |         |
   Available  UpToDate ──> Idle (auto-clear after 1s)
      |
      +──────────────+
      |              |
   Idle          Downloading
  (dismiss)          |
                 Installing
                     |
              NSIS /S (app exits)
```

Transitions:
- `Idle` -> `Checking`: triggered by `AuthWrapper` after login if auto-update is enabled
- `Checking` -> `Available`: newer version found; manifest stored in `Signal<Option<UpdateManifest>>`
- `Checking` -> `UpToDate`: current version is the latest; auto-clears to `Idle` after 1 second
- `Checking` -> `Error`: network failure, JSON parse error, or invalid version
- `Available` -> `Downloading`: user clicks "Update now"
- `Available` -> `Idle`: user clicks "Later" (dismisses until next login)
- `Downloading` -> `Installing`: download complete, signature verified, zip extracted
- `Downloading` -> `Error`: download, signature, or extraction failure
- `Installing` -> process exit: NSIS installer runs `/S` (silent), replaces the application

There is no "skip version" feature. The notification reappears on every login if the update is still available.

## Update Check Trigger

The check is not triggered at app startup. It is triggered by `AuthWrapper` after a successful login, and only if the `AutoUpdate` setting is `true`.

The flow in `AuthWrapper`:

1. `use_resource` fetches `UserSettings` from the database (table `user_settings`, column `auto_update`)
2. The `AutoUpdate` value is propagated via `Signal<AutoUpdate>` context
3. A `use_effect` watches the `auto_update` signal. When it becomes `true` and the check has not started yet:
   - Waits 3 seconds after login
   - Sets state to `Checking`
   - Calls `check_for_update()`
   - On success, stores the manifest and sets `Available` with version, notes, and pub_date
   - If already up to date, briefly shows `UpToDate` then returns to `Idle`

The `update_check_started` signal acts as a guard to prevent concurrent or repeated checks.

## Download and Install

`download_and_install()` takes the manifest and the update state signal:

1. Resolves platform to `windows-x86_64` (only Windows is supported)
2. Creates `%TEMP%/pwdmanager_update/` directory
3. Downloads the zip archive using `bytes_stream()` with chunk-by-chunk progress
4. Updates `Downloading { progress }` from 0% to 95% during download
5. At 96%: calls `verify_update_signature()`
6. At 98%: extracts the zip
7. Finds the first `.exe` in the extracted directory
8. Sets state to `Installing`
9. Launches the installer with `/S` flag (NSIS silent mode)
10. The current application exits when the installer takes over

### Signature Verification

The public key is embedded at compile time from `keys/update-public.key` using `include_str!`. The `keys/update-public.key` file contains the standard minisign format (untrusted comment line + base64 key).

The verification process in `verify_update_signature()`:

1. Decode the public key: `PublicKey::decode(PUBLIC_KEY)` (full minisign file format)
2. Base64-decode the `signature` field from `latest.json` to get the raw `.sig` file content
3. Parse the signature: `Signature::decode(&sig_text)` (multi-line minisign format)
4. Read the downloaded zip file bytes
5. Verify: `pk.verify(&file_bytes, &sig, false)` (legacy signatures rejected)

If verification fails, the update is aborted and the state is set to `Error`.

## Signing (Release Process)

Artifacts are signed with minisign, not with Tauri's built-in signer.

### Key Generation

```bash
minisign -G -p keys/update-public.key -s keys/pwdmanager.key
```

- `keys/update-public.key`: committed to the repo, embedded in the binary at compile time
- `keys/pwdmanager.key`: private key, never committed

### Environment Variables

The private key password is stored in `.env` (not committed):

```env
DIOXUS_SIGNING_PRIVATE_KEY=<contents of pwdmanager.key>
DIOXUS_SIGNING_PRIVATE_KEY_PASSWORD=<key password>
```

### Release Script

`scripts/build-updater-artifacts.sh` automates the signing and manifest generation:

```bash
./scripts/build-updater-artifacts.sh <version> <bundle_output_dir>
```

The script:

1. Loads env vars from `.env`
2. Finds the NSIS `.exe` in the bundle output
3. Signs the exe with minisign: `minisign -Sm <exe> -t "PWDManager v<version>" -x <exe>.sig`
4. Creates `*.nsis.zip` containing the installer
5. Base64-encodes the `.sig` file
6. Generates `latest.json` with version, release notes (from `RELEASE_NOTES.md`), pub_date, and platform entries
7. Prints the `gh release create` command to upload artifacts

### Uploading to GitHub

After running the script:

```bash
gh release create v<version> \
    --title "v<version>" \
    --notes-file RELEASE_NOTES.md \
    "<nsis.zip>" \
    "<bundle_dir>/latest.json"
```

The `latest.json` must be a release asset so that the endpoint `/releases/latest/download/latest.json` resolves correctly.

## Update Notification UI

The `UpdateNotification` component is rendered in `App()` alongside the router and toast container. It is not a toast -- it is a dedicated overlay component with its own state and styling.

States rendered:
- `Idle` / `UpToDate`: nothing rendered
- `Checking`: spinner with "Checking for updates..." text
- `Available`: card showing version, date, changelog (rendered as `dangerous_inner_html` from the `notes` field), "Update now" and "Later" buttons
- `Downloading`: progress bar with percentage
- `Installing`: spinner with "Installing, the app will restart..."
- `Error`: error message with "Close" button

The changelog is displayed before the user decides to install. It is not persisted in the database because the update replaces the application binary -- the notification is inherently one-shot.

## Settings Integration

- The `AutoUpdate` toggle is in `GeneralSettings` (Settings page)
- It is stored as a boolean in the `user_settings` table (`auto_update` column, defaults to `true`)
- Toggling it updates the global `Signal<AutoUpdate>` immediately via context, but the change is only persisted to the database when the user clicks "Save"
- The toggle state is restored from the database on each login by `AuthWrapper`

## Dioxus.toml Configuration

```toml
[bundle]
name = "PWDManager"
version = "0.2.0"
identifier = "com.app.pwdmanager"
publisher = "Lucio Di Capua"
icon = ["icons/icon.ico", "icons/icon.png"]

[bundle.windows]
icon_path = "icons/icon.ico"
digest_algorithm = "sha256"

[webview_install_mode.EmbedBootstrapper]
silent = true

[bundle.windows.nsis]
template = "installer/custom-installer.nsi"
installer_hooks = "installer/nsis-hooks.nsh"
```

There is no `create_updater_artifacts` flag. The updater artifacts (signed zip and latest.json) are generated by the release script, not by the Dioxus bundler.

## Differences from Tauri Classic

| Aspect | Tauri Classic | PWDManager (Dioxus 0.7.3) |
|---|---|---|
| Signing tool | `tauri signer generate` | `minisign` |
| Updater plugin | `tauri-plugin-updater` | Custom Rust implementation with `reqwest` |
| Signature format | Tauri-specific | Standard minisign (base64-encoded in JSON) |
| Config | `tauri.conf.json` | `Dioxus.toml` bundle section |
| Relaunch | `tauri_plugin_process::relaunch()` | NSIS `/S` replaces the binary |
| State | React/Zustand | Dioxus `Signal<UpdateState>` |
| Persistence | localStorage | SQLite (`user_settings` table) |
| CI/CD | GitHub Actions | Manual with `scripts/build-updater-artifacts.sh` |
| Platform support | Windows, macOS, Linux | Windows x86_64 only |

## Troubleshooting

| Problem | Cause | Solution |
|---|---|---|
| `latest.json` returns 404 | File not uploaded as release asset | Ensure `latest.json` is in the release assets, not the body |
| No update detected | `CARGO_PKG_VERSION` >= version in `latest.json` | Bump version and create a new release |
| Download fails | Firewall, proxy, or URL mismatch | Verify the URL in `latest.json` manually |
| Signature verification fails | Public key mismatch or corrupted download | Regenerate keys and update `keys/update-public.key` |
| Progress bar stuck at 96% | Signature verification is slow or failing | Check the error message in the UI; verify minisign key pair |
| Installer does not launch | No `.exe` found in extracted zip | Check that the zip contains a single `.exe` at the root |
| `keys/update-public.key` is empty | Key file not populated | Run `minisign -G` and copy the public key to the file |
