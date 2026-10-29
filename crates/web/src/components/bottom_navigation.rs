use dioxus::prelude::*;

use crate::{
    components::{BookIcon, HomeIcon, NairaIcon, WalletIcon},
    routes::Route,
};

#[component]
pub fn BottomNavigation() -> Element {
    let route = use_route::<Route>();
    let home_active = matches!(&route, Route::Home {});
    let learn_active = matches!(&route, Route::Learn {} | Route::LessonDetail { .. });
    let wallet_active = matches!(&route, Route::Wallet {});
    let payments_active = matches!(&route, Route::Payments {});

    rsx! {
        nav { class: "bottom-nav", aria_label: "Primary mobile navigation",
            Link {
                class: if home_active { "bottom-nav__link bottom-nav__link--active" } else { "bottom-nav__link" },
                to: Route::Home {},
                aria_current: if home_active { "page" } else { "false" },
                HomeIcon {}
                span { "Home" }
            }
            Link {
                class: if learn_active { "bottom-nav__link bottom-nav__link--active" } else { "bottom-nav__link" },
                to: Route::Learn {},
                aria_current: if learn_active { "page" } else { "false" },
                BookIcon {}
                span { "Learn" }
            }
            Link {
                class: if wallet_active { "bottom-nav__link bottom-nav__link--active" } else { "bottom-nav__link" },
                to: Route::Wallet {},
                aria_current: if wallet_active { "page" } else { "false" },
                WalletIcon {}
                span { "Wallet" }
            }
            Link {
                class: if payments_active { "bottom-nav__link bottom-nav__link--active" } else { "bottom-nav__link" },
                to: Route::Payments {},
                aria_current: if payments_active { "page" } else { "false" },
                NairaIcon {}
                span { "Payments" }
            }
        }
    }
}
