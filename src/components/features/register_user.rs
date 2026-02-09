use crate::backend::db_backend::{fetch_user_data, save_user};
use crate::backend::utils::{get_user_avatar_with_default, scale_avatar};
use dioxus::prelude::*;
use rfd::FileDialog;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use tracing::instrument;

#[component]
#[instrument]
pub fn RegisterUser() -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut repassword = use_signal(|| String::new());
    let selected_image = use_signal(|| None::<Vec<u8>>);
    let mut _error = use_signal(|| Option::<String>::None);
    let pool = use_context::<SqlitePool>();
    let nav = use_navigator();
    let pick_image = move |_evt: MouseEvent| {
        let mut err_signal = _error;
        let mut img_signal = selected_image;
        spawn(async move {
            let file = FileDialog::new()
                .add_filter("Image Files", &["png", "jpg", "jpeg"])
                .set_directory("/")
                .pick_file();

            if let Some(path) = file {
                if !Path::new(&path).exists() {
                    println!("File non trovato");
                    err_signal.set(Some("File non trovato".to_string()));
                    return;
                }
                if let Ok(bytes) = fs::read(path) {
                    match bytes {
                        bytes if !bytes.is_empty() => {
                            let scaled = scale_avatar(bytes.as_slice());
                            match scaled {
                                Ok(scaled) => {
                                    println!("Avatar scaled successfully");
                                    img_signal.set(Some(scaled))
                                }
                                Err(e) => {
                                    println!("Errore scaling avatar: {}", e);
                                    err_signal.set(Some(format!("Errore scaling avatar: {}", e)));
                                    return;
                                }
                            }
                        }
                        _ => {
                            err_signal.set(Some("File vuoto, fallback to default".to_string()));
                            return;
                        }
                    }
                }
            }
        });
    };

    let on_submit = move |_| {
        let pool = pool.clone();
        let u = username.read().clone();
        let p = password.read().clone();
        let rp = repassword.read().clone();
        let a = selected_image.read().clone();
        if p == rp {
            spawn(async move {
                // La tua funzione check_user ora ha il pool!
                let result = save_user(&pool, u, p, a).await;
                match result {
                    Ok(_) => {
                        println!("Successo!");
                        nav.push("/login");
                    }
                    Err(e) => {
                        println!("Errore: {}", e.clone());
                        _error.set(Some(e.to_string()));
                    }
                }
            });
        } else {
            _error.set(Some("Le password non coincidono".to_string()));
            println!("Le password non coincidono")
        }
    };

    rsx! {
        div { class: "page-centered",
            div { class: "auth-form card card-md form-container-lg animate-scale-in overflow-y-auto",
                h1 { class: "text-h2 text-center", "Create Account" }
                p { class: "text-body mb-4 text-center", "Sign up to get started with your account" }
                div { class: "flex flex-col items-center gap-3 mb-4",
                    img {
                        class: "avatar-lg avatar-bordered shadow-lg",
                        src: "{get_user_avatar_with_default(selected_image.read().clone())}"
                    }
                    button {
                        class: "btn-ghost btn-sm",
                        r#type: "button",
                        onclick: pick_image,
                        "Select Avatar"
                    }
                }
                form { onsubmit: on_submit, class: "flex flex-col gap-3 w-full",
                    div {
                        label { class: "input-label", "Username" }
                        input {
                            class: "input-base",
                            oninput: move |e| username.set(e.value()),
                            placeholder: "Choose a username"
                        }
                    }
                    div {
                        label { class: "input-label", "Password" }
                        input {
                            class: "input-base",
                            r#type: "password",
                            oninput: move |e| password.set(e.value()),
                            placeholder: "Create a password"
                        }
                    }
                    div {
                        label { class: "input-label", "Confirm Password" }
                        input {
                            class: "input-base",
                            r#type: "password",
                            oninput: move |e| repassword.set(e.value()),
                            placeholder: "Confirm your password"
                        }
                    }
                    button {
                        class: "btn-primary btn-block",
                        r#type: "submit",
                        "Register"
                    }
                    button {
                        class: "btn-secondary btn-block",
                        r#type: "button",
                        onclick: move |_| {nav.push("/login");},
                        "Login"
                    }
                }
            }
        }
    }
}
