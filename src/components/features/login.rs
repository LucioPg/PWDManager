use crate::auth::AuthState;
use crate::backend::db_backend::{check_user, fetch_user_data};
use dioxus::prelude::*;
use sqlx::SqlitePool;
use tracing::{debug, instrument};
#[component]
#[instrument]
pub fn Login() -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut _error = use_signal(|| Option::<String>::None);
    let nav = use_navigator();
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
                Ok(()) => {
                    println!("Successo!");
                    let result = fetch_user_data(&pool, &u).await;
                    match result {
                        Ok((id, username, created_at, avatar)) => {
                            debug!("Login {id} {username} {created_at}");
                            auth_state.login(id, username, created_at, avatar);
                            let nav_dashboard = nav.clone();
                            nav_dashboard.push("/dashboard");
                        }
                        Err(e) => println!("Errore: {}", e),
                    }
                }
                Err(e) => println!("Errore login: {e}"),
            }
        });
    };

    rsx! {
        div { class: "page-centered",
            div { class: "card card-md form-container animate-scale-in max-h-[90vh] overflow-y-auto",
                h1 { class: "text-h2 text-center", "Welcome Back" }
                p { class: "text-body mb-4 text-center", "Sign in to your account to continue" }
                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    div {
                        label { class: "input-label", "Username" }
                        input {
                            class: "input-base",
                            oninput: move |e| username.set(e.value()),
                            placeholder: "Enter your username"
                        }
                    }
                    div {
                        label { class: "input-label", "Password" }
                        input {
                            class: "input-base",
                            r#type: "password",
                            oninput: move |e| password.set(e.value()),
                            placeholder: "Enter your password"
                        }
                    }
                    button {
                        class: "btn-primary btn-block",
                        r#type: "submit",
                        "Login"
                    }
                    button {
                        class: "btn-secondary btn-block",
                        r#type: "button",
                        onclick: move |_| {nav.push("/register");},
                        "Register"
                    }
                }
            }
        }
    }
}
