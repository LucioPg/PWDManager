//! Windows Hello authentication module.
//!
//! Provides biometric/PIN verification via UserConsentVerifier
//! and master password storage via OS keyring.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use tracing::{debug, info, warn};

use crate::backend::db_key::keyring_service_name;

/// Service name for keyring entries — matches existing DB key convention in db_key.rs.
/// Note: db_key.rs uses "PWDManager" / "PWDManager-dev" for dev separation.
/// This module reuses the same service to keep all entries under the same keyring namespace.
/// Dev/prod separation is handled by reusing db_key::keyring_service_name().
const KEYRING_AUTOLOGIN_PREFIX: &str = "autologin_";

/// Result of a Windows Hello verification attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelloResult {
    /// User successfully verified
    Success,
    /// User cancelled the prompt
    Cancelled,
    /// Hardware not available or policy disabled
    NotAvailable,
    /// Windows Hello not configured for this user
    NotEnrolled,
    /// Other failure with description
    Failed(String),
}

/// Builds the keyring entry name for a given username.
fn keyring_entry(username: &str) -> String {
    format!("{}{}", KEYRING_AUTOLOGIN_PREFIX, username)
}

// ── Platform-specific implementation ──────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use tracing::error;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use windows::{
        core::HSTRING,
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::Foundation::{BOOL, HWND, LPARAM, RPC_E_CHANGED_MODE},
        Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
        Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow,
            SW_RESTORE,
        },
    };

    /// Initialize COM for WinRT calls (best-effort, tolerant of re-init).
    fn ensure_com_initialized() {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_err() && hr != RPC_E_CHANGED_MODE {
                warn!("COM initialization returned unexpected error: {:?}", hr);
            }
        }
    }

    /// Bring the first visible window of our process to the foreground.
    fn bring_app_to_foreground() {
        let found = AtomicPtr::new(std::ptr::null_mut());

        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&found as *const _ as isize),
            );
        }

        let hwnd = HWND(found.load(Ordering::SeqCst));
        if !hwnd.0.is_null() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let found = &*(lparam.0 as *const AtomicPtr<std::ffi::c_void>);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));

            if pid == std::process::id() && IsWindowVisible(hwnd).as_bool() {
                found.store(hwnd.0, Ordering::SeqCst);
                BOOL(0)
            } else {
                BOOL(1)
            }
        }
    }

    /// Check if Windows Hello is available and enrolled.
    #[allow(unused_unsafe)]
    pub fn is_hello_available() -> bool {
        ensure_com_initialized();
        match unsafe {
            UserConsentVerifier::CheckAvailabilityAsync()
                .ok()
                .and_then(|op| op.get().ok())
        } {
            Some(UserConsentVerifierAvailability::Available) => true,
            other => {
                debug!("Hello availability: {:?}", other);
                false
            }
        }
    }

    /// Request Windows Hello verification from the user.
    #[allow(unused_unsafe)]
    pub fn request_verification(message: &str) -> HelloResult {
        ensure_com_initialized();
        bring_app_to_foreground();

        let prompt = HSTRING::from(message);

        let result = unsafe {
            UserConsentVerifier::RequestVerificationAsync(&prompt)
                .ok()
                .and_then(|op| op.get().ok())
        };

        match result {
            Some(UserConsentVerificationResult::Verified) => {
                info!("Windows Hello verification succeeded");
                HelloResult::Success
            }
            Some(UserConsentVerificationResult::Canceled) => {
                info!("Windows Hello verification cancelled by user");
                HelloResult::Cancelled
            }
            Some(UserConsentVerificationResult::NotConfiguredForUser) => {
                warn!("Windows Hello not configured for user");
                HelloResult::NotEnrolled
            }
            Some(UserConsentVerificationResult::DisabledByPolicy) => {
                warn!("Windows Hello disabled by policy");
                HelloResult::NotAvailable
            }
            Some(UserConsentVerificationResult::DeviceNotPresent) => {
                warn!("Windows Hello device not present");
                HelloResult::NotAvailable
            }
            Some(UserConsentVerificationResult::RetriesExhausted) => {
                warn!("Windows Hello retries exhausted");
                HelloResult::Failed("Tentativi esauriti".to_string())
            }
            other => {
                error!("Windows Hello returned unexpected result: {:?}", other);
                HelloResult::Failed("Risultato inatteso".to_string())
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::Command;
    use tracing::debug;

    /// Check if any authentication method is available (fingerprint OR system password).
    pub fn is_hello_available() -> bool {
        is_fingerprint_available() || is_system_password_available()
    }

    /// Request user verification — tries fingerprint first, falls back to system password.
    pub fn request_verification(message: &str) -> HelloResult {
        if is_fingerprint_available() {
            let username = match std::env::var("USER") {
                Ok(u) if !u.is_empty() => u,
                _ => return HelloResult::NotAvailable,
            };
            return verify_fingerprint(&username);
        }

        if is_system_password_available() {
            return verify_with_system_password(message);
        }

        HelloResult::NotAvailable
    }

    // ── Fingerprint (fprintd) ──

    fn is_fingerprint_available() -> bool {
        let username = match std::env::var("USER") {
            Ok(u) if !u.is_empty() => u,
            _ => return false,
        };

        match Command::new("fprintd-list").arg(&username).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{stdout}{stderr}");
                debug!(
                    "fprintd-list exit={}, stdout='{}', stderr='{}'",
                    output.status.code().unwrap_or(-1),
                    stdout.trim(),
                    stderr.trim()
                );

                if !output.status.success() {
                    return false;
                }
                if combined.contains("NoSuchDevice")
                    || combined.contains("No devices")
                    || combined.contains("NoDevices")
                {
                    return false;
                }
                combined.contains("Right") || combined.contains("Left") || combined.contains("Thumb")
            }
            Err(e) => {
                debug!("fprintd-list not available: {}", e);
                false
            }
        }
    }

    fn verify_fingerprint(username: &str) -> HelloResult {
        match Command::new("fprintd-verify").arg(username).output() {
            Ok(output) if output.status.success() => {
                info!("Fingerprint verification succeeded");
                HelloResult::Success
            }
            Ok(output) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );

                if combined.contains("NoSuchDevice")
                    || combined.contains("No devices")
                    || combined.contains("Impossible to verify")
                {
                    debug!("No fingerprint device available");
                    HelloResult::NotAvailable
                } else if combined.contains("no fingers") || combined.contains("No fingers") {
                    warn!("No fingers enrolled for {}", username);
                    HelloResult::NotEnrolled
                } else if combined.contains("timed out") || combined.contains("Timeout") {
                    info!("Fingerprint verification timed out (cancelled)");
                    HelloResult::Cancelled
                } else {
                    debug!("Fingerprint verification failed: {}", combined.trim());
                    HelloResult::Failed("Impronta non riconosciuta".to_string())
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("fprintd-verify not found");
                HelloResult::NotAvailable
            }
            Err(e) => {
                warn!("fprintd-verify error: {}", e);
                HelloResult::Failed(format!("Errore verifica: {}", e))
            }
        }
    }

    // ── System password (zenity dialog + PAM direct) ──

    fn is_system_password_available() -> bool {
        Command::new("which")
            .arg("zenity")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Show native GTK password dialog via zenity and verify via PAM.
    fn verify_with_system_password(message: &str) -> HelloResult {
        let username = match std::env::var("USER") {
            Ok(u) if !u.is_empty() => u,
            _ => return HelloResult::NotAvailable,
        };

        let dialog_text = if message.is_empty() {
            "Authenticate to unlock PWDManager".to_string()
        } else {
            message.to_string()
        };

        let zenity = match Command::new("zenity")
            .args([
                "--password",
                &format!("--title=PWDManager"),
                &format!("--text={}", dialog_text),
            ])
            .output()
        {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return HelloResult::NotAvailable;
            }
            Err(e) => {
                return HelloResult::Failed(format!("Errore dialogo: {}", e));
            }
        };

        if !zenity.status.success() {
            return HelloResult::Cancelled;
        }

        // Trim trailing newlines/carriage returns from zenity output
        let raw = &zenity.stdout;
        let end = raw
            .iter()
            .rposition(|&b| b != b'\n' && b != b'\r' && b != b'\0')
            .map_or(0, |i| i + 1);

        if end == 0 {
            return HelloResult::Cancelled;
        }

        let password = match std::str::from_utf8(&raw[..end]) {
            Ok(s) => s,
            Err(_) => return HelloResult::Failed("Password non valida".to_string()),
        };

        verify_password_pam(&username, password)
    }

    fn verify_password_pam(username: &str, password: &str) -> HelloResult {
        use pam::Client;

        let mut client = match Client::with_password("login") {
            Ok(c) => c,
            Err(e) => {
                debug!("Failed to init PAM client: {}", e);
                return HelloResult::NotAvailable;
            }
        };

        client.conversation_mut().set_credentials(username, password);

        match client.authenticate() {
            Ok(()) => {
                info!("PAM authentication succeeded for '{}'", username);
                HelloResult::Success
            }
            Err(e) => {
                debug!("PAM authentication failed for '{}': {}", username, e);
                HelloResult::Failed("Password non corretta".to_string())
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod platform {
    use super::*;

    pub fn is_hello_available() -> bool {
        false
    }

    pub fn request_verification(_message: &str) -> HelloResult {
        HelloResult::NotAvailable
    }
}

// ── Public API ────────────────────────────────────────────────────────

/// Check if Windows Hello is available and enrolled on this device.
pub fn is_hello_available() -> bool {
    platform::is_hello_available()
}

/// Request Windows Hello verification from the user.
pub fn request_verification(message: &str) -> HelloResult {
    platform::request_verification(message)
}

/// Store the master password in the OS keyring (base64 encoded).
///
/// # Errors
/// Returns an error if the keyring operation fails.
pub fn store_master_password(username: &str, password: &str) -> Result<(), String> {
    let entry_name = keyring_entry(username);
    let encoded = BASE64.encode(password);

    let entry = keyring::Entry::new(keyring_service_name(), &entry_name)
        .map_err(|e| format!("Impossibile creare entry keyring: {}", e))?;

    entry
        .set_password(&encoded)
        .map_err(|e| format!("Impossibile salvare nel keyring: {}", e))?;

    debug!("Master password salvata nel keyring per '{}'", username);
    Ok(())
}

/// Load the master password from the OS keyring (base64 decoded).
///
/// # Errors
/// Returns an error if the keyring entry doesn't exist or operation fails.
pub fn load_master_password(username: &str) -> Result<String, String> {
    let entry_name = keyring_entry(username);

    let entry = keyring::Entry::new(keyring_service_name(), &entry_name)
        .map_err(|e| format!("Impossibile creare entry keyring: {}", e))?;

    let encoded = entry
        .get_password()
        .map_err(|e| format!("Impossibile leggere dal keyring: {}", e))?;

    let decoded = BASE64
        .decode(&encoded)
        .map_err(|e| format!("Impossibile decodificare master password: {}", e))?;

    String::from_utf8(decoded)
        .map_err(|e| format!("Master password non valida UTF-8: {}", e))
}

/// Remove the master password from the OS keyring.
///
/// Missing entries are silently treated as success.
pub fn clear_master_password(username: &str) -> Result<(), String> {
    let entry_name = keyring_entry(username);

    let entry = keyring::Entry::new(keyring_service_name(), &entry_name)
        .map_err(|e| format!("Impossibile creare entry keyring: {}", e))?;

    match entry.delete_credential() {
        Ok(()) => {
            debug!("Master password rimossa dal keyring per '{}'", username);
            Ok(())
        }
        Err(e) => {
            let err_str = e.to_string();
            // Silently succeed for "not found" errors
            if err_str.contains("not found")
                || err_str.contains("missing")
                || err_str.contains("No entry")
            {
                warn!("Keyring entry non trovata per '{}', trattata come successo", username);
                Ok(())
            } else {
                Err(format!("Impossibile cancellare dal keyring: {}", e))
            }
        }
    }
}
