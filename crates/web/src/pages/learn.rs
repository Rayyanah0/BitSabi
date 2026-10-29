//! Learning library with real, device-local progress.

use dioxus::prelude::*;

use crate::{
    components::{LessonCard, ProgressBar, ShieldIcon},
    education::{use_progress, LESSONS},
};

#[component]
pub fn Learn() -> Element {
    let progress = use_progress();
    let completed = progress.completed_count();
    let total = LESSONS.len();

    rsx! {
        div { class: "page learn-page",
            header { class: "page-header",
                p { class: "eyebrow", "Learn Bitcoin" }
                h1 { "Short lessons, no jargon" }
                p {
                    "Five guided lessons cover the basics, staying safe, and sending or receiving bitcoin. Each one takes about three minutes and ends with a single quiz question."
                }
            }

            section { class: "course-overview", aria_label: "Course progress",
                div { class: "course-overview__copy",
                    h2 {
                        if completed == total {
                            "All five lessons complete"
                        } else if completed == 0 {
                            "Ready when you are"
                        } else {
                            "Pick up where you left off"
                        }
                    }
                    p {
                        "Progress is saved in this browser only. You can review a completed lesson at any time."
                    }
                }
                div { class: "course-overview__progress",
                    ProgressBar { completed, total, label: "Course progress" }
                }
            }

            section { class: "section", aria_labelledby: "lesson-list-heading",
                div { class: "section__heading",
                    h2 { id: "lesson-list-heading", "Lessons" }
                    p { "Work through the list in order, or revisit any topic." }
                }

                div { class: "lesson-grid",
                    for lesson in LESSONS {
                        LessonCard {
                            key: "{lesson.slug}",
                            lesson,
                            status: progress.status(lesson.slug),
                        }
                    }
                }
            }

            aside { class: "learn-note",
                ShieldIcon {}
                div {
                    strong { "Learning first, money later." }
                    p {
                        "BitSabi does not ask for a recovery phrase to teach a lesson. Wallet practice will use testnet bitcoin only."
                    }
                }
            }
        }
    }
}
