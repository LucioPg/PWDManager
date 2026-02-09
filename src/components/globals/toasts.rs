use dioxus::prelude::*;
use std::default::Default;

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
    pub toast_type: ToastType,
    pub is_leaving: bool,
}

impl Default for ToastMessage {
    fn default() -> Self {
        Self {
            id: Default::default(),
            message: Default::default(),
            duration: 3,
            toast_type: ToastType::Info,
            is_leaving: false,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct ToastsState {
    messages: Vec<ToastMessage>,
    counter: usize,
}

impl ToastsState {
    pub fn push(&mut self, message: String, duration: usize, toast_type: ToastType) -> usize {
        let id = self.counter;
        let toast = ToastMessage {
            id,
            message,
            duration,
            toast_type,
            is_leaving: false,
        };
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
    let toast_type = |ts: &ToastType| match ts {
        ToastType::Success => "toast-success",
        ToastType::Error => "toast-error",
        ToastType::Warning => "toast-warning",
        ToastType::Info => "toast-info",
    };
    rsx! {
        div { class: "toast-container",
            for toast in state.read().messages.iter() {
                {let transition_class = if toast.is_leaving { "toast-out"}
                else { "toast-in" };
                    rsx! {
                        div
                         {
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

pub fn add_toast(
    message: String,
    duration: usize,
    toast_type: ToastType,
    state: &mut Signal<ToastsState>,
) {
    let mut state_transition = state.clone();
    let id = state.write().push(message, duration, toast_type);
    state.write().counter += 1;

    let mut state = state.clone();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(duration as u64)).await;
        if let Some(toast) = state_transition.write()
            .messages
            .iter_mut()
            .find(|m| m.id == id)
        {
            toast.is_leaving = true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(duration as u64)).await;
        state.write().remove(id);
    });
}
