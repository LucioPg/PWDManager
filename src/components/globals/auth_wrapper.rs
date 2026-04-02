// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::Route;
use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_settings;
use crate::backend::settings_types::{AutoLogoutSettings, AutoUpdate, Theme};
use crate::backend::updater::check_for_update;
use crate::backend::updater_types::{UpdateManifest, UpdateState};
use crate::backend::vault_utils::fetch_vaults_by_user;
use dioxus::desktop::use_wry_event_handler;
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::time::{Duration, Instant};

/// Stato condiviso del vault attivo, accessibile via `use_context`.
#[derive(Clone, Copy, Default)]
pub struct ActiveVaultState(pub Signal<Option<i64>>);

#[component]
pub fn AuthWrapper() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    #[allow(unused_mut)]
    let mut app_theme = use_context::<Signal<Theme>>();
    #[allow(unused_mut)]
    let mut auto_update = use_context::<Signal<AutoUpdate>>();
    #[allow(unused_mut)]
    let mut auto_logout_settings = use_context::<Signal<Option<AutoLogoutSettings>>>();
    // Stato del vault attivo
    let mut active_vault = use_context_provider(|| ActiveVaultState(Signal::new(None)));
    // Flag per fetch unico dei settings
    #[allow(unused_mut)]
    let mut theme_fetched = use_signal(|| false);
    // Flag per fetch unico dei settings di autoupdate
    #[allow(unused_mut)]
    let mut auto_update_fetched = use_signal(|| false);
    // Flag per fetch unico dei settings di autoupdate
    #[allow(unused_mut)]
    let mut auto_logout_settings_fetched = use_signal(|| false);
    // Leggi Signal<UpdateState> fornito da App() — NON dentro use_effect!
    let update_state = use_context::<Signal<UpdateState>>();
    let update_manifest = use_context::<Signal<Option<UpdateManifest>>>();
    // Guardia: evita check multipli concorrenti
    let mut update_check_started = use_signal(|| false);

    // --- Activity Tracking per Auto-Logout ---
    let mut last_activity = use_signal(Instant::now);

    // Intercetta ogni evento finestra (mouse, tastiera, focus) a livello nativo tao
    use_wry_event_handler(move |event, _| {
        if let dioxus::desktop::tao::event::Event::WindowEvent { .. } = event {
            last_activity.set(Instant::now());
        }
    });

    // Reset del timer quando le settings di auto-logout cambiano
    use_effect(move || {
        let _ = *auto_logout_settings.read();
        last_activity.set(Instant::now());
    });

    // Timer periodico per auto-logout
    let mut auto_logout_started = use_signal(|| false);
    let auth_for_timer = auth_state.clone();
    use_effect(move || {
        if auto_logout_started() {
            return;
        }
        let settings = *auto_logout_settings.read();
        if settings.is_none() {
            return; // Auto-logout disabilitato
        }
        auto_logout_started.set(true);

        let last = last_activity;
        let logout_settings = auto_logout_settings;
        let mut auth = auth_for_timer.clone();
        let mut theme = app_theme;
        let nav = nav;

        spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;

                if !auth.is_logged_in() {
                    return;
                }

                let settings = *logout_settings.read();
                let Some(timeout) = settings.map(|s| s.duration()) else {
                    continue;
                };

                if last.read().elapsed() >= timeout {
                    auth.logout();
                    theme.set(Theme::Light);
                    nav.push(Route::LandingPage);
                    return;
                }
            }
        });
    });

    if !auth_state.is_logged_in() {
        nav.push(Route::LandingPage);
    }

    let user_id = auth_state.get_user_id();

    use_resource(move || {
        let pool = pool.clone();
        let mut app_theme = app_theme;
        let mut theme_fetched = theme_fetched;
        let mut auto_update_fetched = auto_update_fetched;
        let mut auto_update = auto_update;
        let mut auto_logout_settings_fetched = auto_logout_settings_fetched;
        let mut auto_logout_settings = auto_logout_settings;
        let mut active_vault = active_vault;
        let user_id = user_id;
        async move {
            if (theme_fetched() && auto_update_fetched() && auto_logout_settings_fetched())
                || user_id <= 0
            {
                return;
            }
            if let Ok(Some(settings)) = fetch_user_settings(&pool, user_id).await {
                app_theme.set(settings.theme);
                auto_update.set(settings.auto_update);
                auto_logout_settings.set(settings.auto_logout_settings);

                // Carica il vault attivo
                if let Some(vault_id) = settings.active_vault_id {
                    active_vault.0.set(Some(vault_id));
                } else if let Ok(vaults) = fetch_vaults_by_user(&pool, user_id).await {
                    // Default al primo vault per created_at
                    if let Some(first) = vaults.first() {
                        active_vault.0.set(first.id);
                    }
                }
            }
            theme_fetched.set(true);
            auto_update_fetched.set(true);
            auto_logout_settings_fetched.set(true);
        }
    });

    // Trigger check aggiornamenti quando AutoUpdate viene letto dal DB
    use_effect(move || {
        let auto_update_enabled = auto_update.read().0;
        if !auto_update_enabled || update_check_started() {
            return;
        }

        update_check_started.set(true);
        let mut update_state_clone = update_state;
        let mut update_manifest_clone = update_manifest;

        spawn(async move {
            // Attendi 3 secondi dopo il login
            tokio::time::sleep(Duration::from_secs(3)).await;

            update_state_clone.set(UpdateState::Checking);

            match check_for_update().await {
                Ok(Some(manifest)) => {
                    let version = manifest.version.clone();
                    let notes = manifest.notes.clone();
                    let pub_date = manifest.pub_date.clone();
                    // Salva il manifest per il download
                    update_manifest_clone.set(Some(manifest));
                    update_state_clone.set(UpdateState::Available {
                        version,
                        notes,
                        pub_date,
                    });
                }
                Ok(None) => {
                    update_state_clone.set(UpdateState::UpToDate);
                    // Auto-clear dopo 1 secondo come da spec
                    let mut state_for_clear = update_state_clone;
                    spawn(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        state_for_clear.set(UpdateState::Idle);
                    });
                }
                Err(e) => {
                    update_state_clone.set(UpdateState::Error(e));
                }
            }
        });
    });

    rsx! {
        Outlet::<Route> {}
    }
}
