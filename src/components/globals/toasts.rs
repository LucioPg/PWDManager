use std::default::Default;
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct ToastMessage {
    pub id: usize,
    pub message: String,
    pub duration: usize,
}

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            id: Default::default(),
            message: Default::default(),
            duration: 3,
        }
    }
}


#[derive(Clone, Default, Debug)]
pub struct ToastsState {
    messages: Vec<ToastMessage>,
    counter: usize,
}

impl ToastsState {
    pub fn push(&mut self, message: String, duration: usize) -> usize{
        let id = self.counter;
        let toast = ToastMessage { id, message, duration };
        self.messages.push(toast);
        id
    }

    pub fn remove(&mut self, id: usize) {
        self.messages.retain(|m| m.id != id);
    }
}

#[component]
pub fn ToastContainer() -> Element {
    let state = use_context::<Signal<ToastsState>>();
    rsx! {
        div { class: "fixed bottom-4 right-4 z-9999 flex flex-col gap-2",
            for toast in state.read().messages.iter() {
                div {
                    id: "{toast.id}",
                    class: "bg-slate-800 text-white px-4 py-2 rounded shadow-lg animate-in fade-in slide-in-from-right-5",
                    "{toast.message}"
                }
            }
        }
    }
}

pub fn add_toast(message: String, duration: usize, state: &mut Signal<ToastsState>) {
    let id =state.write().push(message, duration);
    state.write().counter += 1;
    let mut state = state.clone();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(duration as u64)).await;
        state.write().remove(id);
    });
}