use dioxus::prelude::*;

use crate::{
    components::{
        ArrowRightIcon, BitcoinIcon, CheckIcon, ClockIcon, NetworkIcon, SendReceiveIcon,
        ShieldIcon, StatusBadge, WalletIcon,
    },
    education::{Lesson, LessonIcon, LessonStatus},
    routes::Route,
};

#[component]
pub fn LessonIconView(icon: LessonIcon) -> Element {
    match icon {
        LessonIcon::Bitcoin => rsx! { BitcoinIcon {} },
        LessonIcon::Network => rsx! { NetworkIcon {} },
        LessonIcon::Wallet => rsx! { WalletIcon {} },
        LessonIcon::Shield => rsx! { ShieldIcon {} },
        LessonIcon::SendReceive => rsx! { SendReceiveIcon {} },
    }
}

#[component]
pub fn LessonCard(lesson: Lesson, status: LessonStatus) -> Element {
    let action = match status {
        LessonStatus::NotStarted => "Start lesson",
        LessonStatus::InProgress => "Continue lesson",
        LessonStatus::Completed => "Review lesson",
    };

    rsx! {
        article { class: "lesson-card",
            div { class: "lesson-card__top",
                div { class: "lesson-card__icon", LessonIconView { icon: lesson.icon } }
                StatusBadge { status }
            }
            div { class: "lesson-card__heading",
                p { class: "lesson-card__number", "Lesson {lesson.number}" }
                h3 { "{lesson.title}" }
            }
            p { class: "lesson-card__summary", "{lesson.summary}" }
            div { class: "lesson-card__footer",
                span { class: "lesson-card__meta",
                    ClockIcon {}
                    "{lesson.minutes} min read"
                }
                Link {
                    class: "lesson-card__action",
                    to: Route::LessonDetail { slug: lesson.slug.to_owned() },
                    aria_label: "{action}: {lesson.title}",
                    if status == LessonStatus::Completed {
                        CheckIcon {}
                    }
                    span { "{action}" }
                    if status != LessonStatus::Completed {
                        ArrowRightIcon {}
                    }
                }
            }
        }
    }
}
