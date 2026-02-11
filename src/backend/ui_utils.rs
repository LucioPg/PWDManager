use crate::backend::utils::{get_user_avatar_with_default, scale_avatar};
use dioxus::prelude::*; // o i tuoi import specifici
use rfd::FileDialog;
use std::fs;
use std::path::Path;
use tokio::task::spawn_blocking;

pub async fn pick_and_process_avatar(
    mut img_signal: Signal<Option<Vec<u8>>>,
    mut is_loading_signal: Signal<bool>,
    mut is_picking_signal: Signal<bool>,  // ← Nuovo parametro per tracciare il dialog
    mut err_signal: Signal<Option<String>>,
) {
    // Imposta "picking" immediatamente per prevenire click multipli
    is_picking_signal.set(true);

    // Esegui FileDialog in spawn_blocking per non bloccare il thread UI
    let file_result = spawn_blocking(|| {
        FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg"])
            .set_directory("/")
            .pick_file()
    })
    .await;

    let Ok(Some(path)) = file_result else {
        return;
    };

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

    // Resetta is_picking anche se l'operazione è stata annullata
    is_picking_signal.set(false);
}
