// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::backend::updater_types::{UpdateManifest, UpdateState};
use base64::Engine;
use dioxus::prelude::*;
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

    let current =
        Version::parse(CURRENT_VERSION).map_err(|e| format!("Invalid current version: {}", e))?;
    let available = Version::parse(manifest.version.trim_start_matches('v'))
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
fn verify_update_signature(signature_b64: &str, file_path: &Path) -> Result<(), String> {
    // PublicKey::decode() accetta il formato file completo (con riga "untrusted comment")
    let pk = PublicKey::decode(PUBLIC_KEY).map_err(|e| format!("Invalid public key: {}", e))?;

    // Il campo signature in latest.json è base64-encoded: decodifichiamo prima
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("Invalid signature base64: {}", e))?;
    let sig_text =
        String::from_utf8(sig_bytes).map_err(|e| format!("Signature not valid UTF-8: {}", e))?;
    let sig =
        Signature::decode(&sig_text).map_err(|e| format!("Invalid signature format: {}", e))?;

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
    mut update_state: Signal<UpdateState>,
) -> Result<(), String> {
    // Determina la piattaforma - per ora solo Windows
    let platform_key = "windows-x86_64";

    let platform_info = manifest
        .platforms
        .get(platform_key)
        .ok_or_else(|| format!("No update for platform: {}", platform_key))?;

    let temp_dir = std::env::temp_dir().join("pwdmanager_update");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Cannot create temp dir: {}", e))?;

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
            update_state.set(UpdateState::Downloading {
                progress: pct.min(95),
            });
        }
    }
    drop(file);

    update_state.set(UpdateState::Downloading { progress: 96 });

    // Verifica firma sul file zip prima dell'estrazione
    // signature è base64-encoded (contenuto del file .sig di minisign)
    verify_update_signature(&platform_info.signature, &archive_path)?;

    update_state.set(UpdateState::Downloading { progress: 98 });

    // Estrai lo zip — pulisci la cartella prima per rimuovere installer di vecchie versioni
    let extract_dir = temp_dir.join("extracted");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)
            .map_err(|e| format!("Cannot clean extract dir: {}", e))?;
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Cannot create extract dir: {}", e))?;

    let zip_file =
        std::fs::File::open(&archive_path).map_err(|e| format!("Cannot open archive: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Cannot read zip: {}", e))?;
    archive
        .extract(&extract_dir)
        .map_err(|e| format!("Extract error: {}", e))?;

    // Trova il file .exe nell'archivio estratto
    let installer = find_exe_in_dir(&extract_dir)?;

    // Lancia l'installer NSIS in modalita silenziosa con flag update
    update_state.set(UpdateState::Installing);
    std::process::Command::new(&installer)
        .arg("/S")
        .arg("/UPDATE")
        .spawn()
        .map_err(|e| format!("Cannot launch installer: {}", e))?;

    // Esci dall'app per permettere all'installer di sovrascrivere i file
    std::process::exit(0);
}

/// Cerca il file .exe con la versione semver più alta nella directory estratta.
/// Fallback al primo .exe se nessun nome contiene un numero di versione.
fn find_exe_in_dir(dir: &Path) -> Result<PathBuf, String> {
    let exe_files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read extract dir: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        })
        .map(|entry| entry.path())
        .collect();

    if exe_files.is_empty() {
        return Err("No .exe installer found in archive".to_string());
    }

    // Se c'è un solo exe, restituiscilo direttamente
    if exe_files.len() == 1 {
        return Ok(exe_files.into_iter().next().unwrap());
    }

    // Con più exe, seleziona quello con la versione semver più alta nel nome
    let best = exe_files
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            // Cerca pattern tipo "0.2.5" nel nome del file
            let version_str = name.split('_').find_map(|part| {
                Version::parse(part).ok().map(|v| (v, (*path).clone()))
            })?;
            Some(version_str)
        })
        .max_by(|(v1, _), (v2, _)| v1.cmp(v2))
        .map(|(_, path)| path);

    match best {
        Some(path) => Ok(path),
        None => {
            // Fallback: nessun exe con versione nel nome, prendi il primo
            // (comportamento originale retrocompatibile)
            let mut files = exe_files;
            Ok(files.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::updater_types::{UpdateManifest, UpdateState};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    // ================================================================
    // find_exe_in_dir
    // ================================================================

    #[test]
    fn finds_exe_in_dir() {
        let dir = tempfile();
        create_file(&dir, "readme.txt", "docs");
        create_file(&dir, "PWDManager-Setup.exe", "binary");
        create_file(&dir, "config.ini", "settings");

        let result = find_exe_in_dir(&dir);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().file_name().unwrap().to_str().unwrap(),
            "PWDManager-Setup.exe"
        );
    }

    #[test]
    fn finds_highest_version_exe_when_multiple() {
        let dir = tempfile();
        create_file(&dir, "PwdManager_0.2.2_x64-setup.exe", "a");
        create_file(&dir, "PwdManager_0.2.5_x64-setup.exe", "b");
        create_file(&dir, "PwdManager_0.2.3_x64-setup.exe", "c");

        let result = find_exe_in_dir(&dir).unwrap();
        assert!(
            result
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("0.2.5")
        );
    }

    #[test]
    fn errors_when_no_exe() {
        let dir = tempfile();
        create_file(&dir, "notes.md", "# notes");
        create_file(&dir, "data.json", "{}");

        let result = find_exe_in_dir(&dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No .exe"));
    }

    #[test]
    fn errors_on_nonexistent_dir() {
        let dir = PathBuf::from("/tmp/nonexistent_dir_xyz_12345");
        let result = find_exe_in_dir(&dir);
        assert!(result.is_err());
    }

    // ================================================================
    // UpdateManifest deserialization
    // ================================================================

    #[test]
    fn deserializes_valid_json() {
        let json = r#"{
            "version": "0.3.0",
            "notes": "Bug fixes and improvements",
            "pub_date": "2026-03-20T10:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "dGVzdA==",
                    "url": "https://example.com/update.zip"
                }
            }
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "0.3.0");
        assert_eq!(manifest.notes, "Bug fixes and improvements");
        assert_eq!(manifest.pub_date, "2026-03-20T10:00:00Z");

        let platform = manifest.platforms.get("windows-x86_64").unwrap();
        assert_eq!(platform.signature, "dGVzdA==");
        assert_eq!(platform.url, "https://example.com/update.zip");
    }

    #[test]
    fn deserializes_with_v_prefix() {
        let json = r#"{
            "version": "v0.3.0",
            "notes": "Release",
            "pub_date": "2026-01-01T00:00:00Z",
            "platforms": {}
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        // La deserializzazione non trimma la "v" — lo fa check_for_update()
        assert_eq!(manifest.version, "v0.3.0");
    }

    #[test]
    fn errors_on_missing_required_fields() {
        let json = r#"{
            "version": "0.3.0"
        }"#;

        let result = serde_json::from_str::<UpdateManifest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserializes_empty_notes_and_platforms() {
        let json = r#"{
            "version": "1.0.0",
            "notes": "",
            "pub_date": "2026-01-01T00:00:00Z",
            "platforms": {}
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.notes.is_empty());
        assert!(manifest.platforms.is_empty());
    }

    #[test]
    fn deserializes_with_is_breaking_true() {
        let json = r#"{
            "version": "0.3.0",
            "notes": "Breaking change",
            "pub_date": "2026-04-15T10:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "dGVzdA==",
                    "url": "https://example.com/update.zip"
                }
            },
            "is_breaking": true
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.is_breaking);
        assert_eq!(manifest.version, "0.3.0");
    }

    #[test]
    fn deserializes_is_breaking_default_false() {
        let json = r#"{
            "version": "0.2.9",
            "notes": "Bug fix",
            "pub_date": "2026-04-15T10:00:00Z",
            "platforms": {}
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        assert!(!manifest.is_breaking);
    }

    // ================================================================
    // UpdateState
    // ================================================================

    #[test]
    fn equality_between_variants() {
        assert_eq!(UpdateState::Idle, UpdateState::Idle);
        assert_eq!(UpdateState::Checking, UpdateState::Checking);
        assert_eq!(UpdateState::Installing, UpdateState::Installing);
        assert_eq!(UpdateState::UpToDate, UpdateState::UpToDate);

        assert_ne!(UpdateState::Idle, UpdateState::Checking);
        assert_ne!(
            UpdateState::Downloading { progress: 50 },
            UpdateState::Downloading { progress: 51 }
        );

        assert_eq!(
            UpdateState::Available {
                version: "0.3.0".to_string(),
                notes: "fix".to_string(),
                pub_date: "2026-03-20T10:00:00Z".to_string()
            },
            UpdateState::Available {
                version: "0.3.0".to_string(),
                notes: "fix".to_string(),
                pub_date: "2026-03-20T10:00:00Z".to_string()
            }
        );
    }

    #[test]
    fn finds_exe_with_uppercase_extension() {
        let dir = tempfile();
        create_file(&dir, "Setup.EXE", "binary");
        create_file(&dir, "changelog.md", "# changes");

        let result = find_exe_in_dir(&dir);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().file_name().unwrap().to_str().unwrap(),
            "Setup.EXE"
        );
    }

    #[test]
    fn falls_back_to_first_exe_when_no_version_in_name() {
        let dir = tempfile();
        create_file(&dir, "alpha.exe", "a");
        create_file(&dir, "beta.exe", "b");

        let result = find_exe_in_dir(&dir);
        assert!(result.is_ok());
        // Fallback: restituisce il primo .exe trovato (comportamento originale)
        assert!(
            result
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("alpha")
        );
    }

    // ================================================================
    // Helpers
    // ================================================================

    fn tempfile() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pwdmanager_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn create_file(dir: &PathBuf, name: &str, content: &str) {
        let path = dir.join(name);
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }
}
