use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona warning/triangolo - alert di pericolo
#[component]
pub fn WarningIcon(
    #[props(default = "24".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
        }
    }
}

/// Icona logout - freccia di uscita
#[component]
pub fn LogoutIcon(
    #[props(default = "64".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" }
        }
    }
}
