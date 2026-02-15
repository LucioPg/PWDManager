use crate::Route;
use crate::backend::utils::base64_encode;
use crate::components::globals::style::LOGO_BYTES;
use dioxus::prelude::*;

#[component]
pub fn RouteWrapper() -> Element {
    let route = use_route::<Route>();

    // Determiniamo se siamo nella landing page
    // (Assumendo che Route::Landing sia la tua home)
    let is_landing = matches!(route, Route::LandingPage {});

    let logo_data: String = format!("data:image/png;base64,{}", base64_encode(LOGO_BYTES));
    // Se è landing, opacità 100%, altrimenti 30%
    let bg_opacity = if is_landing { "1.0" } else { "0.3" };

    rsx! {
        div { class: "relative min-h-screen w-full",
            // Layer dello sfondo fisso
            div {
                class: "fixed inset-0 -z-10  bg-cover bg-center  transition-opacity duration-500",
                style: "background-image: url('{logo_data}'); opacity: {bg_opacity}"
            }

            // Contenuto dell'app (le pagine vere e proprie)
            main { class: "relative z-10",
                Outlet::<Route> {}
            }
        }
    }
}
