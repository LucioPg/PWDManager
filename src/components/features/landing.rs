use dioxus::prelude::*;



#[component]
pub fn LandingPage() -> Element {

    rsx! {
        // Contenitore principale - contenuto centrato e spostato sotto il logo
        div { class: "overflow-hidden flex items-center justify-center mt-64",

            // IL CONTENUTO (Foreground) - senza card, solo testo stilizzato
            div { class: "z-10 animate-fade-in text-center",

                // Subtitle - font più grande e contrastante
                p { id:"slogan", class: "text-xl font-medium text-neutral-800 mb-8 drop-shadow-md pt-10", "One for rule them all!" }

            }
        }
    }
}