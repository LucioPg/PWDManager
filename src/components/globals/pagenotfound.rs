use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn PageNotFound(segments: Vec<String>) -> Element {
    let nav = use_navigator();
    rsx! {
        div { class: "page-centered text-center",
            p { class: "text-9xl font-bold text-neutral-200 mb-4", "404" }
            h1 { class: "text-h2", "Page Not Found" }
            p { class: "text-body mb-8", "The page you're looking for doesn't exist or has been moved." }
            button {
                class: "btn-primary",
                onclick: move |_| {nav.push(Route::Login);},
                "Go Home"
            }
        }
    }
}
