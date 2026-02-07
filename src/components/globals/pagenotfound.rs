use dioxus::prelude::*;
use crate::Route;

#[component]
pub fn PageNotFound(segments: Vec<String>) -> Element {
    let nav = use_navigator();
    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen px-6 text-center",
            p { class: "text-9xl font-bold text-neutral-200 mb-4", "404" }
            h1 { class: "text-3xl font-bold text-neutral-900 mb-2", "Page Not Found" }
            p { class: "text-neutral-600 mb-8", "The page you're looking for doesn't exist or has been moved." }
            button {
                class: "px-8 py-3 bg-primary-600 text-white font-semibold rounded-lg hover:bg-primary-700 hover:shadow-md transition-all",
                onclick: move |_| {nav.push(Route::Login);},
                "Go Home"
            }
        }
    }
}