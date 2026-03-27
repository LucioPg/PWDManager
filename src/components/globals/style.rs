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
#[component]
pub fn Style() -> Element {
    // assets need to be used to be exported as files
    let _ = format!("{FUTURISTIC_COMMON_FONT}");
    let _ = format!("{FUTURISTIC_FONT}");
    let _ = format!("{COMMON_FONT}");

    let fonts_css = format!(
        "@font-face {{ font-family: 'pwd-common'; src: url('{COMMON_FONT}') format('truetype'); }} \
         @font-face {{ font-family: 'pwd-futuristic'; src: url('{FUTURISTIC_FONT}') format('truetype'); }} \
         @font-face {{ font-family: 'pwd-futuristic-common'; src: url('{FUTURISTIC_COMMON_FONT}') format('truetype'); }}"
    );

    rsx! {
        if cfg!(debug_assertions) {
            document::Stylesheet { href: asset!("/assets/tailwind.css") }
            document::Stylesheet { href: asset!("/assets/main.css") }
        } else {
            document::Style { {include_str!("../../../assets/tailwind.css")} }
            document::Style { {include_str!("../../../assets/main.css")} }
            document::Style { {fonts_css} }
        }
    }
}
