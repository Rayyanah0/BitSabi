use dioxus::prelude::*;

use crate::{
    components::{Brand, SettingsIcon},
    routes::Route,
};

#[component]
pub fn AppHeader() -> Element {
    let route = use_route::<Route>();
    let settings_active = matches!(&route, Route::Settings {});
    let title = page_title(&route);

    rsx! {
        header { class: "app-header",
            div { class: "app-header__inner",
                Brand { compact: true }
                span { class: "app-header__title", "{title}" }
                Link {
                    class: if settings_active { "header-action header-action--active" } else { "header-action" },
                    to: Route::Settings {},
                    aria_label: "Settings",
                    title: "Settings",
                    SettingsIcon {}
                    span { class: "header-action__label", "Settings" }
                }
            }
        }
    }
}

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Home {} => "Home",
        Route::Learn {} => "Learn",
        Route::LessonDetail { .. } => "Lesson",
        Route::Wallet {} => "Wallet",
        Route::Payments {} => "Payments",
        Route::Settings {} => "Settings",
        Route::NotFound { .. } => "Page not found",
    }
}
