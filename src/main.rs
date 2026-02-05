mod backend;

use gui_launcher::launch_desktop;
use dioxus::prelude::*;
// use backend::{list_users, init_db};
use backend::db_backend::init_db;

// fn create_table() -> Result<(), Box<dyn std::error::Error>>{
//     let query  = "\
//     CREATE TABLE IF NOT EXISTS users (username Text, password Text);";
//     let conn = rusqlite::open("database.db")?;
//     let _ = conn.execute(query)?;
//     Ok(())
//
// }

#[component]
fn app() -> Element {
    // Il resource ora conterrà un Result
    let mut db_resource = use_resource(move || async move { init_db().await });
    let resource_value = db_resource.read();
    match &*resource_value {
        Some(Ok(pool)) => {
            // Se il pool è pronto, lo forniamo al resto dell'app
            use_context_provider(|| pool.clone());
            rsx! {  div { class: "main-container",
                    h1 { "Ciao Lucio" }
                    p { "Database creato correttamente" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                } }
        }
        Some(Err(e)) => {
            // Mostriamo l'errore all'utente in modo elegante
            rsx! {
                div { class: "error-container",
                    h1 { "Errore critico del Database" }
                    p { "{e}" }
                    button { onclick: move |_| db_resource.restart(), "Riprova" }
                }
            }
        }
        None => rsx! { "Inizializzazione database in corso..." }
    }
}
fn main() {
    // println!("Creating database and table...");
    // let db_creation: Result<(), Box<dyn std::error::Error>> = create_table();
    // match  db_creation {
    //     Ok(()) => println!("Database created!"),
    //     Err(e) => println!("An error occurred while creating the database: {e}")
    // };
    launch_desktop!(app);

}
