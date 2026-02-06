use dioxus::prelude::*;
// use crate::backend::utils::CUSTOM_B64_ENGINE;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;

#[component]
pub fn LandingPage() -> Element {
    let nav = use_navigator();
    const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo.png");
    let logo_data: String = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(LOGO_BYTES));

    let nav_login = nav.clone();
    let nav_register = nav.clone();
    rsx! {
        // Contenitore principale RELATIVE
        div { class: "relative h-screen w-screen overflow-hidden flex items-center justify-center",

            // 1. IL LOGO (Background)
            div {
                class: "absolute inset-0 flex items-center justify-center -z-10 opacity-20",
                img {
                    src: logo_data, // Assicurati che sia nella cartella assets
                    class: "max-w-[10%] max-h-[10%] object-contain"
                }
            }

            // 2. IL CONTENUTO (Foreground)
            div { class: "z-10 flex flex-col items-center gap-6",
                p { class: "text-4xl font-bold", "Welcome to PWD Manager" }

                div { class: "flex gap-4",
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded",
                        onclick: move |_| {nav_login.push("/login");},
                        "Login"
                    }
                    button {
                        class: "px-4 py-2 border border-blue-600 rounded",
                        onclick: move |_| {nav_register.push("/register");},
                        "Register"
                    }
                }
            }
        }
    }
}