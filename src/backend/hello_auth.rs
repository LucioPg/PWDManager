//! Windows Hello authentication module.
//!
//! Provides biometric/PIN verification via UserConsentVerifier
//! and master password storage via OS keyring.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use tracing::{debug, error, info, warn};

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
    use windows::{
        core::HSTRING,
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::Foundation::RPC_E_CHANGED_MODE,
        Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
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

    /// Check if Windows Hello is available and enrolled.
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
    pub fn request_verification(message: &str) -> HelloResult {
        ensure_com_initialized();

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

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    /// Hello is never available on non-Windows platforms.
    pub fn is_hello_available() -> bool {
        false
    }

    /// Hello is not available on non-Windows platforms.
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

    info!("Master password salvata nel keyring per '{}'", username);
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
            info!("Master password rimossa dal keyring per '{}'", username);
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
