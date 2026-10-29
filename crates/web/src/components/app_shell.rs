//! Shared application frame: desktop sidebar, compact header, and mobile nav.

use dioxus::prelude::*;

use crate::{
    components::{AppHeader, BottomNavigation, Sidebar},
    routes::Route,
};

#[component]
pub fn AppShell() -> Element {
    rsx! {
        a { class: "skip-link", href: "#main-content", "Skip to content" }

        div { class: "app-shell",
            Sidebar {}

            div { class: "app-frame",
                AppHeader {}

                main { id: "main-content", class: "app-main",
                    div { class: "app-content",
                        Outlet::<Route> {}
                    }
                }

                footer { class: "site-footer",
                    p {
                        "BitSabi is a learning project. Wallet features are testnet-only, and NGN payments are simulated."
                    }
                    p { class: "site-footer__muted", "No real money is handled in this version." }
                }

                BottomNavigation {}
            }
        }
    }
}
