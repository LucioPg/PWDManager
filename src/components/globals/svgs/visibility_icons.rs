use super::base_icon::SvgIcon;
use dioxus::prelude::*;

/// Icona occhio aperto - indica che la password è nascosta
#[component]
pub fn EyeIcon(
    #[props(default = "20".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

/// Icona occhio chiuso/sbarrato - indica che la password è visibile
#[component]
pub fn EyeOffIcon(
    #[props(default = "20".to_string())] size: String,
    #[props(default = "2".to_string())] stroke_width: String,
    #[props(default)] class: Option<String>,
) -> Element {
    rsx! {
        SvgIcon {
            size: size,
            stroke_width: stroke_width,
            class: class,
            path { d: "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" }
            line { x1: "1", y1: "1", x2: "23", y2: "23" }
        }
    }
}
