
use crate::Route;
use dioxus::prelude::*;
#[component]
pub fn NavBar() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let nav = use_navigator();
    if auth_state.is_logged_in() {
        let avatar = auth_state.get_avatar();
        rsx! {
        nav { id: "nav",
                class: "nav flex flex-row justify-between",
            Link {to: Route::Dashboard,
                h3 {"Dashboard" }
            }
            div { id: "user-info",
                    class: "flex flex-row items-center gap-2",
            Link {to: Route::Settings, id: "settings",
                        img {id: "little-avatar",
                            class: "w-10 h-10 rounded-full object-cover border-2 border-white shadow-sm",
                            src: "https://api.dicebear.com", // O il tuo base64/URL
                            alt: "User Avatar"
            }
                    }
            Link {to: Route::Logout, id: "logout", "Logout"}
                }
        }
        Outlet::<Route> {}
        }
    }
    else {
        nav.push("/landing");
        rsx! {}
    }

}
