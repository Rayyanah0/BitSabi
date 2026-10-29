//! Settings and transparency information.

use bitsabi_shared::PRODUCT_NAME;
use dioxus::prelude::*;

use crate::{
    components::{ProgressBar, ShieldIcon},
    education::{use_progress, LESSONS},
    routes::Route,
};

#[component]
pub fn Settings() -> Element {
    let progress = use_progress();
    let completed = progress.completed_count();
    let total = LESSONS.len();
    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "page settings-page",
            header { class: "page-header",
                p { class: "eyebrow", "Settings" }
                h1 { "Your preferences and safety" }
                p {
                    "BitSabi keeps the first version deliberately simple. There are no accounts, no tracking pixels, and no wallet secrets stored on the server."
                }
            }

            div { class: "settings-grid",
                section { class: "settings-card",
                    h2 { "Learning progress" }
                    p { "Your lesson progress is saved in this browser only." }
                    ProgressBar { completed, total, label: "Lessons completed" }
                    Link { class: "text-link", to: Route::Learn {}, "Go to lessons" }
                }

                section { class: "settings-card",
                    h2 { "Wallet and network" }
                    p {
                        "Wallet features are not enabled yet. When they arrive, BitSabi will use Bitcoin testnet only and keep keys inside your browser."
                    }
                    div { class: "settings-card__row",
                        span { "Network" }
                        strong { "Testnet planned" }
                    }
                    div { class: "settings-card__row",
                        span { "Wallet status" }
                        strong { "Not enabled" }
                    }
                    Link { class: "text-link", to: Route::Wallet {}, "View the wallet plan" }
                }

                section { class: "settings-card",
                    h2 { "Privacy" }
                    div { class: "settings-card__row",
                        span { "Lesson progress" }
                        strong { "This device" }
                    }
                    div { class: "settings-card__row",
                        span { "Recovery phrases" }
                        strong { "Never sent to the API" }
                    }
                    p { class: "settings-card__note",
                        "A browser wallet cannot remove every risk. We will document its limits before any real-money use."
                    }
                }

                section { class: "settings-card settings-card--about",
                    div { class: "settings-card__about-icon", ShieldIcon {} }
                    div {
                        h2 { "{PRODUCT_NAME}" }
                        p { "Version {version} · Learning project" }
                        p {
                            "No real money is handled in this version. Testnet bitcoin and simulated naira results will always be labelled clearly."
                        }
                    }
                }
            }
        }
    }
}
