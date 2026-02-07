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
                    h1 { class: "text-display mb-4", "PWDManager" }
                    p { class: "text-body-lg", "Secure password management for everyone" }
                }

                // Buttons
                div { class: "flex gap-4",
                    button {
                        class: "btn-primary",
                        onclick: move |_| {nav_login.push("/login");},
                        "Login"
                    }
                    button {
                        class: "btn-secondary",
                        onclick: move |_| {nav_register.push("/register");},
                        "Register"
                    }
                }
            }
        }
    }
}