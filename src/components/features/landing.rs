use dioxus::prelude::*;



#[component]
pub fn LandingPage() -> Element {
    let nav = use_navigator();
    // const LOGO_BYTES: &[u8] = include_bytes!("../../../assets/logo.png");
    // let logo_data: String = format!("data:image/png;base64,{}", base64_encode(LOGO_BYTES));

    let nav_login = nav.clone();
    let nav_register = nav.clone();
    rsx! {
        // Contenitore principale - contenuto in alto
        div { class: "relative h-screen w-screen overflow-hidden flex items-start justify-center pt-20",

            // IL CONTENUTO (Foreground) - senza card, solo testo stilizzato
            div { class: "z-10 animate-fade-in text-center",

                // Subtitle - font più grande e contrastante
                p { id:"slogan", class: "text-xl font-medium text-neutral-800 mb-8 drop-shadow-md", "One for rule them all!" }

                // Buttons
                // div { class: "flex gap-4",
                //     button {
                //         class: "btn-primary",
                //         onclick: move |_| {nav_login.push("/login");},
                //         "Login"
                //     }
                //     button {
                //         class: "btn-secondary",
                //         onclick: move |_| {nav_register.push("/register");},
                //         "Register"
                //     }
                // }
            }
        }
    }
}