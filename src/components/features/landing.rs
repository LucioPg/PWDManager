use dioxus::prelude::*;



#[component]
pub fn LandingPage() -> Element {
    let nav = use_navigator();
    // const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo.png");
    // let logo_data: String = format!("data:image/png;base64,{}", base64_encode(LOGO_BYTES));

    let nav_login = nav.clone();
    let nav_register = nav.clone();
    rsx! {
        // Contenitore principale RELATIVE
        div { class: "relative h-screen w-screen overflow-hidden flex items-center justify-center",

            // 1. IL LOGO (Background)
            // div {
            //     class: "absolute inset-0 flex items-center justify-center -z-10 opacity-20",
            //     img {
            //         src: logo_data, // Assicurati che sia nella cartella assets
            //         class: "max-w-full max-h-full object-contain"
            //     }
            // }

            // 2. IL CONTENUTO (Foreground)
            div { class: "z-10 flex flex-col items-center gap-8 animate-fade-in",

                // Title
                div { class: "text-center",
                    h1 { class: "text-5xl font-bold text-neutral-900 mb-4", "PWDManager" }
                    p { class: "text-xl text-neutral-600", "Secure password management for everyone" }
                }

                // Buttons
                div { class: "flex gap-4",
                    button {
                        class: "px-8 py-3 bg-primary-600 text-white font-semibold rounded-lg hover:bg-primary-700 hover:shadow-md transition-all duration-200",
                        onclick: move |_| {nav_login.push("/login");},
                        "Login"
                    }
                    button {
                        class: "px-8 py-3 border-2 border-primary-600 text-primary-600 font-semibold rounded-lg hover:bg-primary-50 transition-all duration-200",
                        onclick: move |_| {nav_register.push("/register");},
                        "Register"
                    }
                }
            }
        }
    }
}