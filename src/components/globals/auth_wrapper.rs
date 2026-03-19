use crate::Route;
use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::{AutoUpdate, Theme};
use dioxus::prelude::*;
use sqlx::SqlitePool;

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    #[allow(unused_mut)]
    let mut app_theme = use_context::<Signal<Theme>>();
    #[allow(unused_mut)]
    let mut auto_update = use_context::<Signal<AutoUpdate>>();

    // Flag per fetch unico dei settings
    #[allow(unused_mut)]
    let mut theme_fetched = use_signal(|| false);
    // Flag per fetch unico dei settings di autoupdate
    #[allow(unused_mut)]
    let mut auto_update_fetched = use_signal(|| false);

    if !auth_state.is_logged_in() {
        nav.push(Route::LandingPage);
    }

    let user_id = auth_state.get_user_id();

    use_resource(move || {
        let pool = pool.clone();
        let mut app_theme = app_theme.clone();
        let mut theme_fetched = theme_fetched.clone();
        let mut auto_update_fetched = auto_update_fetched.clone();
        let mut auto_update = auto_update.clone();
        let user_id = user_id.clone();
        async move {
            if (theme_fetched() && auto_update_fetched()) || user_id <= 0 {
                return;
            }
            if let Ok(Some(settings)) = fetch_user_settings(&pool, user_id).await {
                app_theme.set(settings.theme);
                auto_update.set(settings.auto_update);
            }
            theme_fetched.set(true);
            auto_update_fetched.set(true);
        }
    });

    rsx! {
        Outlet::<Route> {}
    }
}
