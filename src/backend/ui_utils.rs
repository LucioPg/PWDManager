use crate::backend::utils::{get_user_avatar_with_default, scale_avatar};
use dioxus::prelude::*; // o i tuoi import specifici
use rfd::FileDialog;
use std::fs;
use std::path::Path;
use tokio::task::spawn_blocking;

pub async fn pick_and_process_avatar(
    mut img_signal: Signal<Option<Vec<u8>>>,
    mut is_loading_signal: Signal<bool>,
    mut err_signal: Signal<Option<String>>,
) {
    let file = FileDialog::new()
        .add_filter("Image Files", &["png", "jpg", "jpeg"])
        .set_directory("/")
        .pick_file();

    let Some(path) = file else { return };

    if !Path::new(&path).exists() {
        err_signal.set(Some("File non trovato".to_string()));
        return;
    }

    if let Ok(bytes) = fs::read(path) {
        if bytes.is_empty() {
            err_signal.set(Some("File vuoto".to_string()));
            return;
        }

        is_loading_signal.set(true);

        // Esegui lo scaling in un thread separato
        let bytes_cloned = bytes.clone();
        let scaled_result = spawn_blocking(move || scale_avatar(&bytes_cloned)).await;

        match scaled_result {
            Ok(Ok(scaled)) => {
                img_signal.set((Some(scaled)));
            }
            Ok(Err(e)) => {
                err_signal.set(Some(format!("Errore scaling: {}", e)));
            }
            Err(e) => {
                err_signal.set(Some(format!("Errore thread: {}", e)));
            }
        }
        is_loading_signal.set(false);
    }
}
