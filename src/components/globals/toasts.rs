use std::default::Default;
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ToastMessage {
    pub id: usize,
    pub message: String,
    pub duration: usize,
    pub toast_type: ToastType
}

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            id: Default::default(),
            message: Default::default(),
            duration: 3,
            toast_type: ToastType::Info
        }
    }
}


#[derive(Clone, Default, Debug)]
pub struct ToastsState {
    messages: Vec<ToastMessage>,
    counter: usize,
}

impl ToastsState {
    pub fn push(&mut self, message: String, duration: usize, toast_type: ToastType) -> usize{
        let id = self.counter;
        let toast = ToastMessage { id, message, duration, toast_type };
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
    let toast_type = |ts: &ToastType| {
        match ts {
            ToastType::Success => "toast-success",
            ToastType::Error => "toast-error",
            ToastType::Warning => "toast-warning",
            ToastType::Info => "toast-info",
        }
    };
    rsx! {
        div { class: "fixed bottom-4 right-4 z-9999 flex flex-col gap-2",
            for toast in state.read().messages.iter() {
                div {
                    id: "{toast.id}",
                    class: "{toast_type(&toast.toast_type)}",
                    "{toast.message}"
                }
            }
        }
    }
}

pub fn add_toast(message: String, duration: usize, toast_type: ToastType, state: &mut Signal<ToastsState>) {
    let id =state.write().push(message, duration, toast_type);
    state.write().counter += 1;
    let mut state = state.clone();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(duration as u64)).await;
        state.write().remove(id);
    });
}