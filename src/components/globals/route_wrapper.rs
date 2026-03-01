use crate::Route;
use crate::backend::utils::base64_encode;
use crate::components::globals::style::LOGO_BYTES;
use dioxus::prelude::*;

#[component]
pub fn RouteWrapper() -> Element {
    let route = use_route::<Route>();

    // Determiniamo se siamo nella landing page
    let is_landing = matches!(route, Route::LandingPage {});

    let logo_data: String = format!("data:image/png;base64,{}", base64_encode(LOGO_BYTES));
    // Se è landing, opacità 100%, altrimenti 30%
    let bg_opacity = if is_landing { "1.0" } else { "0.3" };
    // Classe per visibilità sottotesto (opacity + pointer-events per evitare shuttering)
    let slogan_visibility = if is_landing { "pwd-slogan-visible" } else { "pwd-slogan-hidden" };

    rsx! {
        div { class: "relative min-h-screen w-full",
            // Container fisso per logo + sottotesto (mantengono relazione posizionale)
            div { class: "pwd-bg-container",
                // Logo con proporzioni corrette (object-contain invece di bg-cover)
                img {
                    class: "pwd-bg-logo",
                    src: "{logo_data}",
                    style: "opacity: {bg_opacity}",
                    alt: "PWDManager Logo",
                }
                // Sottotesto - visibilità condizionata senza shuttering
                div { class: "pwd-slogan-wrapper {slogan_visibility}",
                    p { class: "pwd-slogan-text", "One for rule them all!" }
                }
            }

            // Contenuto dell'app (le pagine vere e proprie)
            main { class: "relative z-10",
                Outlet::<Route> {}
            }
        }
    }
}
