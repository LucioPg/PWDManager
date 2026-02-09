use crate::components::{ActionButtons, ActionButtonsVariant};
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
                class: "navbar",
            Link {to: Route::Dashboard,
                class: "navbar-brand",
                h3 { class: "navbar-brand-text", "Dashboard" }
            }
            div { id: "user-info",
                    class: "navbar-user",
            Link {to: Route::Settings, id: "settings",
                        img {id: "little-avatar",
                            class: "avatar-md avatar-hover",
                            src: "{avatar}",
                            alt: "User Avatar"
            }
                    }
            Link {to: Route::Logout, id: "logout", class: "navbar-link text-error", "Logout"}
                }
        }
        Outlet::<Route> {}
        }
    } else {
        let nav_login = nav.clone();
        let nav_register = nav.clone();
        rsx! {
            nav { id: "nav",
                class: "navbar",
                div { class: "nav-logo-container w-full h-full",
                    Link {to: Route::LandingPage,
                    class: "navbar-brand",
                }
                }

                div { class: "navbar-nav",
                    ActionButtons {
                        primary_text: "Login".to_string(),
                        secondary_text: "Register".to_string(),
                        primary_on_click: move |_| { nav_login.push(Route::Login); },
                        secondary_on_click: move |_| { nav_register.push(Route::RegisterUser); },
                        variant: ActionButtonsVariant::Nav,
                    }
                }
            }
            Outlet::<Route> {}
        }
    }
}
