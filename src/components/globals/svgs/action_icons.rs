use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona burger/hamburger - menu contestuale con tre linee
#[component]
pub fn BurgerIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            line { x1: "3", y1: "6", x2: "21", y2: "6" }
            line { x1: "3", y1: "12", x2: "21", y2: "12" }
            line { x1: "3", y1: "18", x2: "21", y2: "18" }
        }
    }
}

/// Icona ingranaggio - pulsante modifica/impostazioni
#[component]
pub fn EditIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

/// Icona cestino - pulsante elimina
#[component]
pub fn DeleteIcon(
    #[props(default = "18".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M3 6h18" }
            path { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" }
            path { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" }
            line { x1: "10", y1: "11", x2: "10", y2: "17" }
            line { x1: "14", y1: "11", x2: "14", y2: "17" }
        }
    }
}
