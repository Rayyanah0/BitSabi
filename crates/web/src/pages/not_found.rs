//! Fallback page for unknown URLs.

use dioxus::prelude::*;

use crate::{
    components::{ActionLink, ButtonVariant},
    routes::Route,
};

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = format!("/{}", segments.join("/"));

    rsx! {
        div { class: "page page--narrow",
            header { class: "page-header",
                p { class: "eyebrow", "Page not found" }
                h1 { "That page does not exist" }
                p { "The address {path} is not part of BitSabi." }
            }
            ActionLink {
                to: Route::Home {},
                variant: ButtonVariant::Primary,
                "Return home"
            }
        }
    }
}
