use dioxus::prelude::*;

pub const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo.png");
#[used]
static COMMON_FONT: Asset = asset!(
    "/assets/fonts/Lexend-VariableFont_wght.ttf",
    AssetOptions::builder().with_hash_suffix(false)
);
static FUTURISTIC_FONT: Asset = asset!(
    "/assets/fonts/BrunoAceSC-Regular.ttf",
    AssetOptions::builder().with_hash_suffix(false)
);

static FUTURISTIC_COMMON_FONT: Asset = asset!(
    "/assets/fonts/BrunoAce-Regular.ttf",
    AssetOptions::builder().with_hash_suffix(false)
);
// Asset CSS di Tailwind only in dev
// #[cfg(debug_assertions)]
// static TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
// static TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
// #[cfg(debug_assertions)]
// static MAIN_CSS: &str = include_str!("../assets/main.css");
// static MAIN_CSS: Asset = asset!("/assets/main.css");
#[component]
pub fn Style() -> Element {
    let futuristic_common_font_content = format!("{FUTURISTIC_COMMON_FONT}");
    let futuristic_font_content = format!("{FUTURISTIC_FONT}");
    let common_font_content = format!("{COMMON_FONT}");
    // let fonts_css_content = format!(
    //     "@font-face {{
    //         font-family: 'pwd-futuristic';
    //         src: url('{FUTURISTIC_FONT}');
    //     }};
    //     @font-face {{
    //         font-family: 'pwd-common';
    //         src: url('{COMMON_FONT}');
    //     }}
    //     "
    // );
    rsx! {
        if cfg!(debug_assertions) {
            document::Stylesheet { href: asset!("/assets/tailwind.css") }
            document::Stylesheet { href: asset!("/assets/main.css") }
        } else {
            document::Style { {include_str!("../../../assets/tailwind.css")} }
            document::Style { {include_str!("../../../assets/main.css")} }
        }
    }
}
