use dioxus::prelude::*;

pub const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo.png");

// Asset CSS di Tailwind only in dev
// #[cfg(debug_assertions)]
// static TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
// static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
// #[cfg(debug_assertions)]
// static MAIN_CSS: &str = include_str!("../assets/main.css");
// static MAIN_CSS: Asset = asset!("/assets/main.css");
#[component]
pub fn Style() -> Element {
    rsx! {
        if cfg!(debug_assertions) {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        document::Stylesheet {href: asset!("/assets/main.css")}
    }
    else {
        document::Style { { include_str!("../../../assets/tailwind.css") } }
        document::Style { {include_str!("../../../assets/main.css") } }
    }
        }
}
