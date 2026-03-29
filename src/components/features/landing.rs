// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;

/// Landing page component.
/// Note: Logo and slogan are now rendered in RouteWrapper for proper positioning.
/// Future landing page content (CTAs, features, etc.) can be added here.
#[component]
pub fn LandingPage() -> Element {
    rsx! {
        // Container per futuro contenuto della landing page
        // (call-to-action, features, ecc.)
        div { class: "pwd-landing-content",
            // TODO: Aggiungere contenuto futuro qui
        }
    }
}
