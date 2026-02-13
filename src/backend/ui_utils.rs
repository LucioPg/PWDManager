//! Modulo per le utilità UI relative all'avatar utente.
//!
//! Fornisce funzioni per:
//! - Aprire i file dialog nativi per selezionare immagini
//! - Gestire i segnali Dioxus per loading/stato/errore
//! - Elaborare e scalare le immagini selezionate

use crate::backend::utils::scale_avatar;
use dioxus::prelude::*; // o i tuoi import specifici
use rfd::FileDialog;
use std::fs;
use std::path::Path;
use tokio::task::spawn_blocking;

/// Apre un dialog di selezione file, processa l'immagine selezionata e aggiorna i signal forniti.
///
/// Questa funzione gestisce l'intero flusso di selezione e processamento avatar:
/// 1. Apre il dialog nativo per la selezione del file
/// 2. Verifica che il file esista
/// 3. Legge i bytes del file
/// 4. Esegue lo scaling dell'immagine
/// 5. Aggiorna i signal per l'UI (loading, errore, immagine processata)
///
/// # Signal
///
/// - `img_signal`: Aggiornato con l'immagine processata (Vec<u8>) o None
/// - `is_loading_signal`: Impostato a true durante il processamento, false al termine
/// - `is_picking_signal`: Impostato a true mentre il dialog è aperto, false quando chiuso
/// - `err_signal`: Contiene messaggi di errore se qualcosa va storto
///
/// # Esempi
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// #
/// // Nel componente:
/// let mut is_loading = use_signal(|| false);
/// let mut is_picking = use_signal(|| false);
/// let mut error = use_signal(|| None::<String>);
/// let mut new_avatar = use_signal(|| None::<Vec<u8>>);
/// #
/// let pick_image = move |_| {
///     if is_loading() || is_picking() {
///         return;
///     }
///     spawn(pick_and_process_avatar(
///         new_avatar,
///         is_loading,
///         is_picking,
///         error,
///     ));
/// };
/// ```
///
/// # Ritorna
///
/// Non restituisce nulla (i signal vengono aggiornati internamente)
pub async fn pick_and_process_avatar(
    mut img_signal: Signal<Option<Vec<u8>>>,
    mut is_loading_signal: Signal<bool>,
    mut is_picking_signal: Signal<bool>, // ← Nuovo parametro per tracciare il dialog
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

    let path;
    match file_result {
        Ok(Some(p)) => {
            path = p;
        }
        Ok(None) => {
            // Utente ha annullato il dialog
            is_picking_signal.set(false); // Resetta per permettere nuovi tentativi
            return;
        }
        Err(e) => {
            // Errore nel task (es. panic, cancellation)
            err_signal.set(Some(format!("Errore apertura dialog: {}", e)));
            is_picking_signal.set(false); // Resetta anche in caso di errore
            return;
        }
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
                img_signal.set(Some(scaled));
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
