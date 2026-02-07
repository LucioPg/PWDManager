
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
                        },
                        Err(e) => println!("Errore: {}", e)
                    }


                },
                Err(e) => println!("Errore login: {e}"),
            }
        });
    };

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen gap-6 p-6",
            div { class: "bg-white rounded-xl shadow-lg p-8 max-w-md w-full animate-scale-in",
                h1 { class: "text-3xl font-bold text-neutral-900 mb-2 text-center", "Welcome Back" }
                p { class: "text-neutral-600 mb-8 text-center", "Sign in to your account to continue" }
                form { onsubmit: on_submit, class: "flex flex-col gap-4 w-full",
                    div { class: "input-group mb-4",
                        label { class: "block text-sm font-medium text-neutral-700 mb-2", "Username" }
                        input {
                            class: "w-full px-4 py-3 border border-neutral-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent transition-all",
                            oninput: move |e| username.set(e.value()),
                            placeholder: "Enter your username"
                        }
                    }
                    div { class: "input-group mb-4",
                        label { class: "block text-sm font-medium text-neutral-700 mb-2", "Password" }
                        input {
                            class: "w-full px-4 py-3 border border-neutral-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent transition-all",
                            r#type: "password",
                            oninput: move |e| password.set(e.value()),
                            placeholder: "Enter your password"
                        }
                    }
                    button {
                        class: "w-full px-6 py-3 bg-primary-600 text-white font-semibold rounded-lg hover:bg-primary-700 hover:shadow-md transition-all duration-200",
                        r#type: "submit",
                        "Login"
                    }
                    button {
                        class: "w-full px-6 py-3 border-2 border-primary-600 text-primary-600 font-semibold rounded-lg hover:bg-primary-50 transition-all duration-200",
                        r#type: "button",
                        onclick: move |_| {nav.push("/register");},
                        "Register"
                    }
                }
            }
        }
    }
}