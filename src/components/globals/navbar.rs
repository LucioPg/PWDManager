// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

use crate::Route;
use crate::components::{ActionButton, ButtonSize, ButtonType, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn NavBar() -> Element {
    let auth_state = use_context::<crate::auth::AuthState>();
    let nav = use_navigator();
    if auth_state.is_logged_in() {
        let avatar = auth_state.get_avatar();
        rsx! {
            nav { id: "nav", class: "navbar",
                div { class: "flex flex-rowjustify-between",
                    Link { to: Route::Dashboard, class: "navbar-brand",
                        h3 { class: "navbar-brand-text", "Dashboard" }
                    }
                    div { class: "pwd-navbar-separator", "|" }
                    Link { to: Route::MyVaults, class: "navbar-brand",
                        h3 { class: "navbar-brand-text", "My Vaults" }
                    }
                }
                div { id: "user-info", class: "navbar-user",
                    Link { to: Route::Settings, id: "settings",
                        img {
                            id: "little-avatar",
                            class: "avatar-md avatar-hover",
                            src: "{avatar}",
                            alt: "User Avatar",
                        }
                    }
                    Link {
                        to: Route::Logout,
                        id: "logout",
                        class: "navbar-link text-error",
                        "Logout"
                    }
                }
            }
            Outlet::<Route> {}
        }
    } else {
        let nav_login = nav;
        rsx! {
            nav { id: "nav", class: "navbar",
                div { class: "nav-logo-container w-full h-full",
                    Link { to: Route::LandingPage, class: "navbar-brand" }
                }

                div { class: "navbar-nav",
                    ActionButton {
                        text: "Login".to_string(),
                        variant: ButtonVariant::Primary,
                        button_type: ButtonType::Button,
                        size: ButtonSize::Normal,
                        on_click: move |_| {
                            nav_login.push(Route::Login);
                        },
                    }
                }
            }
            Outlet::<Route> {}
        }
    }
}
