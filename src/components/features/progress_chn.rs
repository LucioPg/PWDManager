use dioxus::prelude::*;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
#[allow(non_snake_case)]
#[component]
pub fn ProgressChn(
    /// Callback quando l'utente conferma la cancellazione
    on_completed: Signal<bool>,

    /// Callback quando l'utente annulla
    on_failed: Signal<bool>,
) -> Element {
    let mut progress = use_signal(|| 0);
    let mut running = use_signal(|| false);

    rsx! {
        div {
            button {
                class: "btn btn-primary",
                disabled: running(),
                onclick: move |_| {
                    running.set(true);

                    // Clona i signal per la closure async
                    let mut on_completed = on_completed.clone();

                    // 1. Crea il canale per comunicare tra thread
                    let (tx, mut rx) = mpsc::channel(100);

                    // 2. Task asincrono per ricevere i dati e aggiornare la UI
                    spawn(async move {
                        while let Some(val) = rx.recv().await {
                            progress.set(val);
                        }
                        // Piccola pausa per mostrare il 100% prima di chiudere
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        running.set(false);
                        on_completed.set(true);
                    });

                    // 3. Task bloccante per il calcolo pesante (Rayon)
                    tokio::task::spawn_blocking(move || {
                        let total_items = 10000;
                        let items = vec![0; total_items];

                        // Usiamo un contatore atomico condiviso tra i thread di Rayon
                        let completed = Arc::new(AtomicUsize::new(0));

                        items.into_par_iter().for_each(|_| {
                            // --- SIMULAZIONE LAVORO PESANTE ---
                            std::thread::sleep(std::time::Duration::from_millis(10));

                            // Incrementa il contatore atomico in modo sicuro
                            let current = completed.fetch_add(1, Ordering::SeqCst) + 1;

                            // Calcola e invia la percentuale (0-100)
                            let percentage = (current * 100) / total_items;
                            let _ = tx.blocking_send(percentage as usize);
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
