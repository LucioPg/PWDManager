
use dioxus::prelude::*;
use crate::backend::db_backend::{fetch_user_data, save_user};
use sqlx::SqlitePool;
use tracing::instrument;
use rfd::FileDialog;
use std::fs;
use std::path::Path;
use crate::backend::utils::{scale_avatar, get_user_avatar_with_default};

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
                    return
                }
                if let Ok(bytes) = fs::read(path) {
                    match bytes {
                        bytes if !bytes.is_empty() => {
                            let scaled = scale_avatar(bytes.as_slice());
                            match scaled {
                                Ok(scaled) => {
                                    println!("Avatar scaled successfully");
                                    img_signal.set(Some(scaled))
                                },
                                Err(e) => {
                                    println!("Errore scaling avatar: {}", e);
                                    err_signal.set(Some(format!("Errore scaling avatar: {}", e)));
                                    return
                                }
                            }
                        }
                        _ => {
                            err_signal.set(Some("File vuoto, fallback to default".to_string()));
                            return
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
                    Ok(_) => { println!("Successo!");
                        nav.push("/login");
                    },
                    Err(e) => { println!("Errore: {}", e.clone());
                        _error.set(Some(e.to_string()));
                    }
                }
            });
        }
        else {
            _error.set(Some("Le password non coincidono".to_string()));
            println!("Le password non coincidono")
        }

    };

    rsx! {
        div { class: "register-form flex flex-col gap-4",
            div { class: "avatar-container flex flex-row gap-1",
                img {
                    class: "avatar-selection  rounded-full object-cover border-2 border-white shadow-sm",
                    style: "width: 128px; height: 128px;",
                    src: "{get_user_avatar_with_default(selected_image.read().clone())}"
                }
                button { r#type: "button", onclick: pick_image, "Select Avatar" }
            }
            form { onsubmit: on_submit,
                input { oninput: move |e| username.set(e.value()), placeholder: "Username" }
                input {
                    r#type: "password",
                    oninput: move |e| password.set(e.value()),
                    placeholder: "Password"
                }
                input {
                    r#type: "password",
                    oninput: move |e| repassword.set(e.value()),
                    placeholder: "Re-type Password"
                }
                button {class: "btn-primary", r#type: "submit", "Register" }
                button {class: "btn-secondary", r#type: "button", onclick: move |_| {nav.push("/login");} ,"Login"}
            }
        }

    }
}