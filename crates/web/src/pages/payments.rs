//! NGN payments preview. The mock quote appears in Milestone 5.

use dioxus::prelude::*;

#[component]
pub fn Payments() -> Element {
    rsx! {
        div { class: "page",
            header { class: "page-header",
                div { class: "page-header__meta",
                    p { class: "eyebrow", "NGN Payments" }
                    span { class: "pill", "Simulated" }
                }
                h1 { "Understand the quote before you pay" }
                p {
                    "BitSabi will explain how bitcoin is converted to naira, what fees apply, and the final amount you would receive."
                }
            }

            div { class: "notice notice--warning",
                strong { "This is not a real payment service." }
                p {
                    "No provider is connected, no naira is held, and no payment is processed. The mock quote and fee example arrive in Milestone 5."
                }
            }

            section { class: "section", aria_labelledby: "payment-steps-heading",
                h2 { id: "payment-steps-heading", "How the demo will work" }
                ol { class: "step-list",
                    li {
                        span { class: "step-list__number", "1" }
                        div {
                            h3 { "Enter an amount" }
                            p { "Choose a bitcoin amount or a naira target amount." }
                        }
                    }
                    li {
                        span { class: "step-list__number", "2" }
                        div {
                            h3 { "Review the quote" }
                            p { "See the rate, provider fee, network fee, and final amount in one place." }
                        }
                    }
                    li {
                        span { class: "step-list__number", "3" }
                        div {
                            h3 { "Know what is real" }
                            p { "Every simulated result will be clearly labelled. A frontend message will never be treated as proof of payment." }
                        }
                    }
                }
            }

            aside { class: "panel panel--soft",
                h2 { "Before real payments" }
                p {
                    "Real NGN payments require a licensed provider, verified webhooks, settlement rules, and legal and compliance review. None of that is part of the MVP."
                }
            }
        }
    }
}
