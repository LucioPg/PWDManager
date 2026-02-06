
use crate::Route;
use dioxus::prelude::*;
#[component]
pub fn NavBar() -> Element {
    rsx! {
        div { id: "title",
            Link {to: Route::Dashboard,
                h1 {"Dashboard! 🌭" }
            }
            Link {to: Route::Settings, id: "heart", "♥️"}
        }
        Outlet::<Route> {}
    }
}
