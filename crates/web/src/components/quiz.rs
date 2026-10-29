use dioxus::prelude::*;

use crate::{
    components::{ArrowRightIcon, CheckIcon, PrimaryButton, SecondaryButton},
    education::{use_progress, Lesson},
    routes::Route,
};

#[component]
pub fn LessonQuiz(lesson: Lesson, completed: bool) -> Element {
    let progress = use_progress();
    let mut selected = use_signal(|| None::<usize>);
    let selected_value = selected();
    let is_correct = selected_value == Some(lesson.quiz.answer);

    rsx! {
        section { class: "quiz", aria_labelledby: "quiz-heading",
            p { class: "eyebrow", "Quick check" }
            h2 { id: "quiz-heading", "{lesson.quiz.question}" }
            p { class: "quiz__hint", "Choose one answer." }

            div { class: "quiz__options",
                for (index, option) in lesson.quiz.options.iter().enumerate() {
                    button {
                        class: option_class(selected_value, lesson.quiz.answer, index),
                        r#type: "button",
                        aria_pressed: if selected_value == Some(index) { "true" } else { "false" },
                        disabled: is_correct,
                        onclick: move |_| {
                            if !is_correct {
                                selected.set(Some(index));
                            }
                        },
                        span { class: "quiz-option__letter", "{option_letter(index)}" }
                        span { class: "quiz-option__text", "{option}" }
                    }
                }
            }

            if let Some(answer_index) = selected_value {
                div {
                    class: if is_correct { "quiz-feedback quiz-feedback--success" } else { "quiz-feedback quiz-feedback--error" },
                    role: "status",
                    div { class: "quiz-feedback__title",
                        if is_correct {
                            CheckIcon {}
                            strong { "Correct." }
                        } else {
                            strong { "Not quite." }
                        }
                    }
                    p { "{lesson.quiz.explanation}" }
                    div { class: "quiz-feedback__actions",
                        if is_correct {
                            if completed {
                                if let Some(next) = progress.next_lesson() {
                                    Link {
                                        class: "button button--primary",
                                        to: Route::LessonDetail { slug: next.slug.to_owned() },
                                        "Next lesson"
                                        ArrowRightIcon {}
                                    }
                                } else {
                                    Link {
                                        class: "button button--primary",
                                        to: Route::Learn {},
                                        "Back to lessons"
                                        ArrowRightIcon {}
                                    }
                                }
                            } else {
                                PrimaryButton {
                                    onclick: move |_| progress.mark_completed(lesson.slug),
                                    "Complete lesson"
                                }
                            }
                        } else {
                            SecondaryButton {
                                onclick: move |_| selected.set(None),
                                "Try again"
                            }
                        }
                    }
                    if answer_index != lesson.quiz.answer {
                        p { class: "quiz-feedback__note",
                            "The correct answer is highlighted above. Read the explanation, then try again."
                        }
                    }
                }
            }
        }
    }
}

fn option_letter(index: usize) -> &'static str {
    match index {
        0 => "A",
        1 => "B",
        _ => "C",
    }
}

fn option_class(selected: Option<usize>, answer: usize, index: usize) -> String {
    let mut class = String::from("quiz-option");

    if selected == Some(index) {
        if index == answer {
            class.push_str(" quiz-option--correct");
        } else {
            class.push_str(" quiz-option--wrong");
        }
    } else if selected.is_some() && index == answer {
        class.push_str(" quiz-option--answer");
    }

    class
}
