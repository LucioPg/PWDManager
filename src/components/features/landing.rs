// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use dioxus::prelude::*;
use sqlx::SqlitePool;

use crate::backend::db_backend::has_any_user;

/// Landing page component.
/// Note: Logo and slogan are now rendered in RouteWrapper for proper positioning.
/// Future landing page content (CTAs, features, etc.) can be added here.
#[component]
pub fn LandingPage() -> Element {
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();

    // First-launch detection: redirect to /welcome if no users exist.
    // OnceLock ensures the check runs only once per process lifetime.
    use_effect(move || {
        static INIT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if INIT.get().is_some() {
            return;
        }
        let _ = INIT.set(true);

        let pool = pool.clone();
        let nav = nav;
        spawn(async move {
            match has_any_user(&pool).await {
                Ok(false) => {
                    nav.push("/welcome");
                }
                Ok(true) => {} // Users exist — show landing
                Err(e) => {
                    tracing::error!("Failed to check users: {}", e);
                }
            }
        });
    });

    rsx! {
        // Container per futuro contenuto della landing page
        // (call-to-action, features, ecc.)
        div { class: "pwd-landing-content",
            // TODO: Aggiungere contenuto futuro qui
        }
    }
}
