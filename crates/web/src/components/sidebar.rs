use dioxus::prelude::*;

use crate::{
    components::{BookIcon, Brand, HomeIcon, NairaIcon, SettingsIcon, ShieldIcon, WalletIcon},
    routes::Route,
};

#[component]
pub fn Sidebar() -> Element {
    let route = use_route::<Route>();
    let home_active = matches!(&route, Route::Home {});
    let learn_active = matches!(&route, Route::Learn {} | Route::LessonDetail { .. });
    let wallet_active = matches!(&route, Route::Wallet {});
    let payments_active = matches!(&route, Route::Payments {});
    let settings_active = matches!(&route, Route::Settings {});

    rsx! {
        aside { class: "sidebar",
            Brand { compact: false }

            nav { class: "side-nav", aria_label: "Primary navigation",
                Link {
                    class: if home_active { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                    to: Route::Home {},
                    aria_current: if home_active { "page" } else { "false" },
                    HomeIcon {}
                    span { "Home" }
                }
                Link {
                    class: if learn_active { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                    to: Route::Learn {},
                    aria_current: if learn_active { "page" } else { "false" },
                    BookIcon {}
                    span { "Learn" }
                }
                Link {
                    class: if wallet_active { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                    to: Route::Wallet {},
                    aria_current: if wallet_active { "page" } else { "false" },
                    WalletIcon {}
                    span { "Wallet" }
                }
                Link {
                    class: if payments_active { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                    to: Route::Payments {},
                    aria_current: if payments_active { "page" } else { "false" },
                    NairaIcon {}
                    span { "Payments" }
                }
            }

            div { class: "sidebar__footer",
                div { class: "sidebar__note",
                    ShieldIcon {}
                    p {
                        strong { "Practice mode" }
                        span { "Testnet and simulated money only." }
                    }
                }
                Link {
                    class: if settings_active { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                    to: Route::Settings {},
                    aria_current: if settings_active { "page" } else { "false" },
                    SettingsIcon {}
                    span { "Settings" }
                }
            }
        }
    }
}
