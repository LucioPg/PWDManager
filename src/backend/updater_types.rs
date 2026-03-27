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

#[derive(Debug, Clone, Deserialize)]
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
    Available {
        version: String,
        notes: String,
        pub_date: String,
    },
    Downloading { progress: u8 },
    Installing,
    UpToDate,
    Error(String),
}
