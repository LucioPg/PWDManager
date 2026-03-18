use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::Theme;
use crate::auth::AuthState;
use crate::Route;
use dioxus::prelude::*;
use sqlx::SqlitePool;

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    let mut app_theme = use_context::<Signal<Theme>>();

    // Flag per fetch unico dei settings
    let mut theme_fetched = use_signal(|| false);

    if !auth_state.is_logged_in() {
        nav.push(Route::LandingPage);
    }

    let user_id = auth_state.get_user_id();

    use_resource(move || {
        let pool = pool.clone();
        let mut app_theme = app_theme.clone();
        let mut theme_fetched = theme_fetched.clone();
        let user_id = user_id.clone();
        async move {
            if theme_fetched() || user_id <= 0 {
                return;
            }
            if let Ok(Some(settings)) = fetch_user_settings(&pool, user_id).await {
                app_theme.set(settings.theme);
            }
            theme_fetched.set(true);
        }
    });

    rsx! {
        Outlet::<Route> {}
    }
}
