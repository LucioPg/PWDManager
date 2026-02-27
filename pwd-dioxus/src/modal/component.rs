use dioxus::prelude::*;

/// Posizionamento del modal
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ModalVariant {
    #[default]
    Middle,
    Top,
    Bottom,
}

impl ModalVariant {
    pub fn as_class(&self) -> &'static str {
        match self {
            ModalVariant::Middle => "modal-middle",
            ModalVariant::Top => "modal-top",
            ModalVariant::Bottom => "modal-bottom",
        }
    }
}

#[component]
pub fn BaseModal(
    /// Controlla la visibilità del modal
    open: Signal<bool>,

    /// Callback quando il modal deve chiudersi (cancel/backdrop click)
    #[props(default)]
    on_close: EventHandler<()>,

    /// Posizionamento del modal
    #[props(default)]
    variant: ModalVariant,

    /// Classe CSS aggiuntiva per modal-box
    #[props(default)]
    class: String,

    /// Contenuto del modal
    children: Element,
) -> Element {
    let placement_class = variant.as_class();

    // DaisyUI richiede la classe 'modal-open' per attivare il modal
    let open_class = if open() { "modal-open" } else { "" };

    rsx! {
        div {
            class: "modal {open_class} {placement_class}",

            // Backdrop - click fuori chiude il modal
            div {
                class: "modal-backdrop",
                onclick: move |e| {
                    e.stop_propagation();
                    on_close.call(());
                    open.set(false);
                }
            }

            // Contenuto del modal - renderizza solo se aperto
            if open() {
                div {
                    class: "modal-box {class}",
                    {children}
                }
            }
        }
    }
}
