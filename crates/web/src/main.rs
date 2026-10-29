//! BitSabi browser app.
//!
//! The wallet will live entirely in this WebAssembly app in a later milestone.
//! No recovery phrase or private key may ever be sent to the API.

use dioxus::prelude::*;

mod components;
mod education;
mod pages;
mod routes;

use education::provide_progress;
use routes::Route;

static MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    provide_progress();

    rsx! {
        document::Title { "BitSabi - Understand Bitcoin. Take Control." }
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}
