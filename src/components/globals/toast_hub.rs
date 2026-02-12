//! # ToastHub - Sistema Centralizzato per Toast
//!
//! Questo modulo fornisce un sistema centralizzato per gestire i toast
//! che sopravvivono alla navigazione tra le pagine, senza usare parametri URL.

use dioxus::prelude::*;
use std::default::Default;
use std::time::Instant;

// ============================================================================
// TYPES
// ============================================================================

#[derive(Clone, PartialEq, Debug)]
pub enum ToastType {
    Success,
    Error,
    #[allow(dead_code)]
    Warning,
    Info,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ToastMessage {
    pub id: usize,
    pub message: String,
    pub duration: usize,
    pub toast_type: ToastType,
    pub is_leaving: bool,
    pub created_at: Instant,
}

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            id: Default::default(),
            message: Default::default(),
            duration: 3,
            toast_type: ToastType::Info,
            is_leaving: false,
            created_at: Instant::now(),
        }
    }
}

// ============================================================================
// STATE
// ============================================================================

#[derive(Clone, Default, Debug)]
pub struct ToastHubState {
    messages: Vec<ToastMessage>,
    counter: usize,
    // Toast schedulati che verranno mostrati al prossimo rendering
    pending: Vec<ToastMessage>,
}

impl ToastHubState {
    fn push(&mut self, message: String, duration: usize, toast_type: ToastType) -> usize {
        let id = self.counter;
        let toast = ToastMessage {
            id,
            message,
            duration,
            toast_type,
            is_leaving: false,
            created_at: Instant::now(),
        };
        self.messages.push(toast);
        self.counter += 1;
        id
    }

    fn remove(&mut self, id: usize) {
        self.messages.retain(|m| m.id != id);
    }

    // Aggiunge un toast pending che verrà mostrato al prossimo ciclo di rendering
    fn schedule(&mut self, message: String, duration: usize, toast_type: ToastType) {
        let id = self.counter;
        let toast = ToastMessage {
            id,
            message,
            duration,
            toast_type,
            is_leaving: false,
            created_at: Instant::now(),
        };
        self.pending.push(toast);
        self.counter += 1;
    }

    // Sposta i toast pending in quelli attivi
    fn flush_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for toast in pending {
            self.messages.push(toast);
        }
    }

    // Aggiorna lo stato dei toast basandosi sul tempo trascorso
    // Restituisce true se ci sono stati cambiamenti
    pub fn update_timeouts(&mut self) -> bool {
        let now = Instant::now();
        let mut changed = false;
        let mut to_remove = Vec::new();

        for toast in &mut self.messages {
            if !toast.is_leaving {
                let elapsed = now.duration_since(toast.created_at).as_secs() as usize;
                if elapsed >= toast.duration {
                    toast.is_leaving = true;
                    changed = true;
                }
            } else {
                let elapsed = now.duration_since(toast.created_at).as_secs() as usize;
                let leave_duration = toast.duration;
                if elapsed >= toast.duration + leave_duration {
                    to_remove.push(toast.id);
                    changed = true;
                }
            }
        }

        // Rimuovi i toast scaduti
        for id in to_remove {
            self.remove(id);
        }

        changed
    }
}

// ============================================================================
// HOOK - API principale per i componenti
// ============================================================================

/// Hook per usare il ToastHub in un componente.
///
/// Restituisce un `Signal<ToastHubState>` che può essere usato con le funzioni helper.
///
/// # Esempio
///
/// ```rust
/// let toast = use_toast();
///
/// // Mostra un toast immediatamente
/// show_toast_success("Operazione completata!".to_string(), toast);
///
/// // Mostra un toast di errore
/// show_toast_error("Si è verificato un errore".to_string(), toast);
///
/// // Schedula un toast che verrà mostrato dopo la navigazione
/// schedule_toast_success("Utente creato!".to_string(), toast);
/// nav.push("/dashboard");
/// ```
pub fn use_toast() -> Signal<ToastHubState> {
    use_context::<Signal<ToastHubState>>()
}

// ============================================================================
// HELPER FUNCTIONS - Funzioni per mostrare toast
// ============================================================================

// Funzioni per mostrare toast immediatamente

pub fn show_toast_success(message: String, mut state: Signal<ToastHubState>) {
    state.write().push(message, 3, ToastType::Success);
}

pub fn show_toast_error(message: String, mut state: Signal<ToastHubState>) {
    state.write().push(message, 4, ToastType::Error);
}

// Funzioni per schedulare toast (per navigazione)

pub fn schedule_toast_success(message: String, mut state: Signal<ToastHubState>) {
    state.write().schedule(message, 3, ToastType::Success);
}

// ============================================================================
// COMPONENT - Container per visualizzare i toast
// ============================================================================

#[component]
pub fn ToastContainer() -> Element {
    let mut state = use_context::<Signal<ToastHubState>>();

    // Flush pending toast ad ogni rendering
    use_effect(move || {
        state.write().flush_pending();
    });

    // Timer per aggiornare i timeout dei toast
    use_effect(move || {
        let mut state = state.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                state.write().update_timeouts();
            }
        });
    });

    let toast_type = |ts: &ToastType| match ts {
        ToastType::Success => "toast-success",
        ToastType::Error => "toast-error",
        ToastType::Warning => "toast-warning",
        ToastType::Info => "toast-info",
    };

    rsx! {
        div { class: "toast-container",
            for toast in state.read().messages.iter() {
                {let transition_class = if toast.is_leaving { "toast-out" }
                else { "toast-in" };
                    rsx! {
                        div {
                            key: "{toast.id}",
                            class: "{toast_type(&toast.toast_type)} {transition_class}",
                            "{toast.message}"
                        }
                    }
                }
            }
        }
    }
}
