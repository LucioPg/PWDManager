use base64::Engine;
use dioxus::prelude::*;
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
    mut update_state: Signal<UpdateState>,
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
