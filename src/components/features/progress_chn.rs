use dioxus::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
#[allow(non_snake_case)]
pub fn ProgressChn() -> Element {
    let mut progress = use_signal(|| 0);
    let mut running = use_signal(|| false);

    rsx! {
        div {
            button {
                class: "btn btn-primary",
                disabled: running(),
                onclick: move |_| {
                    running.set(true);

                    // 1. Crea il canale per comunicare tra thread
                    let (tx, mut rx) = mpsc::channel(100);

                    // 2. Task asincrono per ricevere i dati e aggiornare la UI
                    spawn(async move {
                        while let Some(val) = rx.recv().await {
                            progress.set(val);
                        }
                        running.set(false);
                    });

                    // 3. Task bloccante per il calcolo pesante (Rayon)
                                    tokio::task::spawn_blocking(move || {
                    let total_items = 100;
                    let items = vec![0; total_items];

                    // Usiamo un contatore atomico condiviso tra i thread di Rayon
                    let completed = Arc::new(AtomicUsize::new(0));

                    items.into_par_iter().for_each(|_| {
                        // --- SIMULAZIONE LAVORO PESANTE ---
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        // Incrementa il contatore atomico in modo sicuro
                        let current = completed.fetch_add(1, Ordering::SeqCst) + 1;

                        // Invia il numero effettivo di elementi completati
                        let _ = tx.blocking_send(current);
                    });
                });
                },
                "Avvia Elaborazione Rayon"
            }
            p { "Progresso: {progress}%" }
            progress { value: "{progress}", max: "100" }
        }
    }
}
