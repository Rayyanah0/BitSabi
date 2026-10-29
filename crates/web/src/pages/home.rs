//! Home dashboard: welcome, real learning progress, and clear next actions.

use bitsabi_shared::{APP_DESCRIPTION, TAGLINE};
use dioxus::prelude::*;

use crate::{
    components::{
        ActionLink, ArrowRightIcon, BitcoinIcon, BookIcon, ButtonVariant, NairaIcon, ProgressBar,
        SendReceiveIcon, ShieldIcon, WalletIcon,
    },
    education::{use_progress, LESSONS},
    routes::Route,
};

#[component]
pub fn Home() -> Element {
    let progress = use_progress();
    let completed = progress.completed_count();
    let total = LESSONS.len();
    let next = progress.next_lesson();

    rsx! {
        div { class: "page home-page",
            section { class: "home-hero",
                div { class: "home-hero__copy",
                    p { class: "eyebrow", "Bitcoin made simple for Nigeria" }
                    h1 { class: "home-hero__title", "{TAGLINE}" }
                    p { class: "home-hero__lead", "{APP_DESCRIPTION}" }
                    div { class: "hero__actions",
                        if completed == 0 {
                            ActionLink {
                                to: Route::Learn {},
                                variant: ButtonVariant::Primary,
                                "Start learning"
                            }
                        } else if let Some(next) = next {
                            ActionLink {
                                to: Route::LessonDetail { slug: next.slug.to_owned() },
                                variant: ButtonVariant::Primary,
                                "Continue learning"
                            }
                        } else {
                            ActionLink {
                                to: Route::Learn {},
                                variant: ButtonVariant::Primary,
                                "Review lessons"
                            }
                        }
                        ActionLink {
                            to: Route::Wallet {},
                            variant: ButtonVariant::Secondary,
                            "See the wallet plan"
                        }
                    }
                }

                aside { class: "progress-card", aria_label: "Learning progress",
                    span { class: "pill pill--orange", "Your course" }
                    h2 { "Your learning path" }
                    p {
                        if completed == total {
                            "You have completed all five lessons. Revisit any lesson whenever you want."
                        } else if completed == 0 {
                            "Start with lesson one. Each lesson takes about three minutes and ends with one question."
                        } else {
                            "Keep going. Your progress is saved on this device, so you can pick up where you left off."
                        }
                    }
                    ProgressBar { completed, total, label: "Lessons completed" }
                    if let Some(next) = next {
                        div { class: "progress-card__next",
                            span { class: "progress-card__label", "Up next" }
                            strong { "{next.title}" }
                            span { "Lesson {next.number} · {next.minutes} min" }
                        }
                        Link {
                            class: "text-link",
                            to: Route::LessonDetail { slug: next.slug.to_owned() },
                            "Open next lesson"
                            ArrowRightIcon {}
                        }
                    } else {
                        Link { class: "text-link", to: Route::Learn {}, "Review all lessons" }
                    }
                }
            }

            section { class: "section", aria_labelledby: "quick-actions-heading",
                div { class: "section__heading",
                    h2 { id: "quick-actions-heading", "What do you want to do?" }
                    p { "Choose a starting point. Wallet and payment features are clearly marked while they are being built." }
                }

                div { class: "quick-grid",
                    Link { class: "quick-card", to: Route::Learn {},
                        div { class: "quick-card__icon", BookIcon {} }
                        div { class: "quick-card__copy",
                            span { class: "quick-card__badge", "5 short lessons" }
                            h3 { "Learn Bitcoin" }
                            p { "Build confidence with plain-English lessons and simple quizzes." }
                        }
                        ArrowRightIcon {}
                    }

                    Link { class: "quick-card", to: Route::Wallet {},
                        div { class: "quick-card__icon", WalletIcon {} }
                        div { class: "quick-card__copy",
                            span { class: "quick-card__badge", "Testnet only" }
                            h3 { "Receive Bitcoin" }
                            p { "See how a receiving address will work in the testnet wallet." }
                        }
                        ArrowRightIcon {}
                    }

                    Link { class: "quick-card", to: Route::Wallet {},
                        div { class: "quick-card__icon", SendReceiveIcon {} }
                        div { class: "quick-card__copy",
                            span { class: "quick-card__badge", "Testnet only" }
                            h3 { "Send Bitcoin" }
                            p { "Review the safety checks before sending bitcoin in practice mode." }
                        }
                        ArrowRightIcon {}
                    }

                    Link { class: "quick-card", to: Route::Payments {},
                        div { class: "quick-card__icon", NairaIcon {} }
                        div { class: "quick-card__copy",
                            span { class: "quick-card__badge", "Simulated" }
                            h3 { "NGN Payments" }
                            p { "Preview how a bitcoin-to-naira quote and fees will be explained." }
                        }
                        ArrowRightIcon {}
                    }
                }
            }

            aside { class: "safety-note",
                div { class: "safety-note__icon", ShieldIcon {} }
                div {
                    h2 { "No real money. No fake balances." }
                    p {
                        "BitSabi will use testnet bitcoin for wallet practice and simulated results for NGN quotes. We will never show a balance or payment as real when it is not."
                    }
                }
                div { class: "safety-note__points",
                    span { BitcoinIcon {} "Keys stay in your browser" }
                    span { "Testnet and simulations only" }
                }
            }
        }
    }
}
