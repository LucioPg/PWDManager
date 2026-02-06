
use dioxus::prelude::*;
use crate::backend::db_backend::{check_user, fetch_user_data};
use crate::auth::AuthState;
use sqlx::SqlitePool;
use tracing::{debug, instrument};
#[component]
#[instrument]
pub fn Login() -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut _error = use_signal(|| Option::<String>::None);
    let pool = use_context::<SqlitePool>();
    let auth_state = use_context::<AuthState>();
    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let mut auth_state = auth_state.clone();
        spawn(async move {
            // La tua funzione check_user ora ha il pool!
            match check_user(&pool, &u, &p).await {
                Ok(true) => {
                    println!("Successo!");
                    let result = fetch_user_data(&pool, &u).await;
                    match result {
                        Ok((id, username, created_at)) => {
                            debug!("Login {id} {username} {created_at}");
                            auth_state.login(id, username, created_at);
                        },
                        Err(e) => println!("Errore: {}", e)
                    }


                },
                _ => println!("Errore login"),
            }
        });
    };

    rsx! {
        form { onsubmit: on_submit,
            input { oninput: move |e| username.set(e.value()), placeholder: "Username" }
            input {
                r#type: "password",
                oninput: move |e| password.set(e.value()),
                placeholder: "Password"
            }
            button { r#type: "submit", "Login" }
        }
    }
}