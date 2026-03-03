use crate::components::MigrationData;
use dioxus::prelude::*;
use rayon::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

#[allow(non_snake_case)]
#[component]
pub fn ProgressMigrationChn(
    /// Callback quando l'utente conferma la cancellazione
    on_completed: Signal<bool>,

    /// Callback quando l'utente annulla
    on_failed: Signal<bool>,
) -> Element {
    let mut progress = use_signal(|| 0);
    let mut running = use_signal(|| false);
    let context = use_context::<Signal<MigrationData>>();
    let pool = use_context::<SqlitePool>();

    rsx! {
        div {
            button {
                class: "btn btn-primary",
                disabled: running(),
                onclick: move |_| {
                    running.set(true);
                    let mut on_completed = on_completed.clone();
                    let context = context.clone();
                    println!("context: {:?}", context());
                    let (tx, mut rx) = mpsc::channel(100);
                    spawn(async move {
                        while let Some(val) = rx.recv().await {
                            progress.set(val);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        running.set(false);
                        on_completed.set(true);
                    });
                    tokio::task::spawn_blocking(move || {
                        let total_items = 10000;
                        let items = vec![0; total_items];
                        let completed = Arc::new(AtomicUsize::new(0));
                        items
                            .into_par_iter()
                            .for_each(|_| {
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
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
