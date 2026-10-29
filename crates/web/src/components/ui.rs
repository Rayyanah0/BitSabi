use dioxus::prelude::*;

use crate::{education::LessonStatus, routes::Route};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
}

impl ButtonVariant {
    fn modifier(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
        }
    }
}

#[component]
pub fn ActionLink(
    to: Route,
    #[props(default)] variant: ButtonVariant,
    children: Element,
) -> Element {
    let class = format!("button button--{}", variant.modifier());

    rsx! {
        Link { class, to, {children} }
    }
}

#[component]
pub fn PrimaryButton(
    #[props(default)] disabled: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "button button--primary",
            r#type: "button",
            disabled,
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

#[component]
pub fn SecondaryButton(
    #[props(default)] disabled: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "button button--secondary",
            r#type: "button",
            disabled,
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

#[component]
pub fn ProgressBar(completed: usize, total: usize, label: &'static str) -> Element {
    let capped = completed.min(total);
    let percent = capped
        .checked_mul(100)
        .and_then(|value| value.checked_div(total))
        .unwrap_or(0);
    let style = format!("width: {percent}%");

    rsx! {
        div { class: "progress-block",
            div { class: "progress-block__meta",
                span { "{label}" }
                span { "{capped} of {total}" }
            }
            div {
                class: "progress",
                role: "progressbar",
                aria_valuemin: "0",
                aria_valuemax: "{total}",
                aria_valuenow: "{capped}",
                aria_label: "{label}",
                div { class: "progress__fill", style }
            }
        }
    }
}

#[component]
pub fn StatusBadge(status: LessonStatus) -> Element {
    let class = format!("status-badge status-badge--{}", status.modifier());

    rsx! {
        span { class, "{status.label()}" }
    }
}
