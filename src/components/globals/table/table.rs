use crate::auth::AuthState;
use crate::backend::db_backend::fetch_user_auth_from_id;
use crate::components::{Spinner, SpinnerSize, show_toast_error, use_toast};
use custom_errors::DBError;
use dioxus::prelude::*;
use sqlx::SqlitePool;
use std::ops::Deref;

#[component]
pub fn Table() -> Element {
    let auth_state = use_context::<AuthState>();
    let pool = use_context::<SqlitePool>();
    let mut error = use_signal(|| <Option<DBError>>::None);
    let user_id_option = auth_state.user.cloned().map(|u| u.id); // Questo è un Option<i64>

    let toast = use_toast();
    let Some(user_id) = user_id_option else {
        error.set(Some(DBError::new_select_error("User not logged in".into())));
        return rsx! {
            Spinner { size: SpinnerSize::Large, color_class: "spinner-error"}
        };
    };
    let user_auth = use_resource(move || {
        let pool_clone = pool.clone();
        async move {
            // La chiamata asincrona deve stare dentro il blocco async
            match fetch_user_auth_from_id(&pool_clone, user_id).await {
                Ok(data) => {
                    println!("Dati ricevuti: {:?}", data);
                    Some(data)
                }
                Err(e) => {
                    println!("Errore nel fetch: {}", e);
                    // Se 'error' è un Signal (use_signal), puoi aggiornarlo qui
                    error.set(Some(e));
                    None
                }
            }
        }
    });

    use_effect(move || {
        if let Some(e) = error.read().deref() {
            show_toast_error(format!("Error fetching user data: {}", e), toast.clone());
        }
    });

    match &*user_auth.read() {
        Some(user_auth) => {
            rsx! {
            // todo table with user stored data
            }
        }
        _ => {
            rsx! {
                Spinner { size: SpinnerSize::Large, }
            }
        }
    }
}
