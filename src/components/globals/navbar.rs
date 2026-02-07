
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
                class: "bg-white/90 backdrop-blur-sm border-b border-neutral-200 px-6 py-4 flex justify-between items-center sticky top-0 z-50 shadow-sm",
            Link {to: Route::Dashboard,
                class: "flex items-center gap-3",
                h3 { class: "text-xl font-bold text-neutral-800", "Dashboard" }
            }
            div { id: "user-info",
                    class: "flex flex-row items-center gap-4 pl-4 border-l border-neutral-200",
            Link {to: Route::Settings, id: "settings",
                        img {id: "little-avatar",
                            class: "w-10 h-10 rounded-full object-cover border-2 border-neutral-200 hover:border-primary-500 transition-colors cursor-pointer",
                            src: "{avatar}",
                            alt: "User Avatar"
            }
                    }
            Link {to: Route::Logout, id: "logout", class: "px-4 py-2 text-error-600 font-medium rounded-lg hover:bg-error-50 transition-colors duration-150", "Logout"}
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
