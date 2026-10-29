use bitsabi_shared::{PRODUCT_NAME, TAGLINE};
use dioxus::prelude::*;

use crate::routes::Route;

#[component]
pub fn Brand(compact: bool) -> Element {
    rsx! {
        Link {
            class: if compact { "brand brand--compact" } else { "brand" },
            to: Route::Home {},
            span { class: "brand__mark", aria_hidden: "true", "₿" }
            span { class: "brand__text",
                span { class: "brand__name", "{PRODUCT_NAME}" }
                span { class: "brand__tagline", "{TAGLINE}" }
            }
        }
    }
}
