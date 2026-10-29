//! Wallet page preview. The testnet wallet arrives in Milestones 6 and 7.

use dioxus::prelude::*;

#[component]
pub fn Wallet() -> Element {
    rsx! {
        div { class: "page",
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "My Wallet" }
                    span { class: "pill pill--orange", "Testnet only" }
                }
                h1 { "Your keys, your bitcoin" }
                p {
                    "BitSabi will help you create and restore a Bitcoin wallet without sending your recovery phrase or private keys to a server."
                }
            }

            div { class: "notice notice--warning",
                strong { "The wallet is not enabled yet." }
                p {
                    "No wallet exists on this page, no address has been created, and no funds can be received. Wallet creation begins in Milestone 6."
                }
            }

            div { class: "two-column",
                section { class: "panel",
                    h2 { "What is planned" }
                    ul { class: "check-list",
                        li { "Create a new testnet wallet with a recovery phrase" }
                        li { "Import a testnet wallet from a recovery phrase" }
                        li { "Show a receiving address and QR code" }
                        li { "Estimate fees before sending" }
                        li { "Track transaction status and history" }
                    }
                }

                aside { class: "panel panel--soft",
                    h2 { "Safety first" }
                    p {
                        "Keys will be generated and used only inside your browser. The server may help fetch testnet chain data, but it will never receive a recovery phrase or private key."
                    }
                    p {
                        "A browser wallet cannot protect you from every risk. Before using real bitcoin, we will document the limitations and require an independent security review."
                    }
                }
            }
        }
    }
}
