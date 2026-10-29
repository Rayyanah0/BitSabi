//! Reading view for a single lesson and its one-question quiz.

use dioxus::prelude::*;

use crate::{
    components::{
        ActionLink, ArrowLeftIcon, ButtonVariant, CheckIcon, ClockIcon, LessonQuiz, ProgressBar,
    },
    education::{find_lesson, use_progress, LessonStatus, LESSONS},
    routes::Route,
};

#[component]
pub fn LessonDetail(slug: String) -> Element {
    let progress = use_progress();

    use_effect(use_reactive((&slug,), move |(slug,)| {
        if let Some(lesson) = find_lesson(&slug) {
            progress.mark_in_progress(lesson.slug);
        }
    }));

    let Some(lesson) = find_lesson(&slug) else {
        return rsx! {
            div { class: "page page--narrow",
                header { class: "page-header",
                    p { class: "eyebrow", "Lesson not found" }
                    h1 { "We could not find that lesson" }
                    p { "The lesson link may be old or incomplete. All five lessons are available in the learning library." }
                }
                ActionLink {
                    to: Route::Learn {},
                    variant: ButtonVariant::Primary,
                    "Back to lessons"
                }
            }
        };
    };

    let completed = progress.status(lesson.slug) == LessonStatus::Completed;
    let completed_count = progress.completed_count();
    let total = LESSONS.len();

    rsx! {
        div { class: "page lesson-page",
            Link { class: "back-link", to: Route::Learn {},
                ArrowLeftIcon {}
                "All lessons"
            }

            header { class: "lesson-header",
                p { class: "eyebrow", "Lesson {lesson.number} of {total}" }
                h1 { "{lesson.title}" }
                p { class: "lesson-header__summary", "{lesson.summary}" }
                div { class: "lesson-header__meta",
                    ClockIcon {}
                    span { "{lesson.minutes} min read" }
                    span { class: "lesson-header__dot", "•" }
                    span { "{lesson.sections.len()} short sections" }
                }
                ProgressBar { completed: completed_count, total, label: "Course progress" }
            }

            if completed {
                div { class: "completion-banner", role: "status",
                    div { class: "completion-banner__icon", CheckIcon {} }
                    div {
                        strong { "Lesson complete" }
                        p { "Your progress is saved on this device." }
                    }
                    Link { class: "text-link", to: Route::Learn {}, "All lessons" }
                }
            }

            section { class: "lesson-content", aria_label: "Lesson content",
                for section in lesson.sections {
                    section { class: "lesson-section",
                        h2 { "{section.heading}" }
                        for paragraph in section.body {
                            p { "{paragraph}" }
                        }
                        if let Some(example) = section.example {
                            aside { class: "lesson-example",
                                span { class: "lesson-example__label", "Keep this in mind" }
                                p { "{example}" }
                            }
                        }
                    }
                }
            }

            LessonQuiz { lesson: *lesson, completed }
        }
    }
}
