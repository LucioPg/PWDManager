use dioxus::prelude::*;

#[component]
pub fn LandingPage() -> Element {
    rsx! {
        div { class: "hero-section",
            div { class: "hero-content",
                p { id: "slogan", class: "slogan-text", "One for rule them all!" }
            }
        }
    }
}
