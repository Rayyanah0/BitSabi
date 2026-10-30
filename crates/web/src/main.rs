//! BitSabi browser app.
//!
//! The testnet wallet runs entirely in this WebAssembly app.
//! No recovery phrase, seed, or private key may ever be sent to the API.

use dioxus::prelude::*;

mod api;
mod components;
mod education;
mod pages;
mod routes;
pub mod wallet;
pub mod wallet_data;

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
