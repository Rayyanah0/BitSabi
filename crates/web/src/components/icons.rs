//! Small inline SVG icons. They inherit the surrounding text colour.

use dioxus::prelude::*;

#[component]
pub fn HomeIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M3.5 10.5 12 3.5l8.5 7" }
            path { d: "M5.5 9.5v10h13v-10" }
            path { d: "M9.75 19.5v-5.5h4.5v5.5" }
        }
    }
}

#[component]
pub fn BookIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15.5H6.5A2.5 2.5 0 0 0 4 21V5.5Z" }
            path { d: "M4 18.5A2.5 2.5 0 0 1 6.5 16H20" }
            path { d: "M8 7.5h8" }
            path { d: "M8 11h5" }
        }
    }
}

#[component]
pub fn WalletIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M4 7.5A2.5 2.5 0 0 1 6.5 5H18a2 2 0 0 1 2 2v1" }
            rect { x: "3", y: "7", width: "18", height: "12", rx: "2.5" }
            path { d: "M16 13h3" }
        }
    }
}

#[component]
pub fn NairaIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            rect { x: "2.5", y: "5", width: "19", height: "14", rx: "2.5" }
            path { d: "M9 15.5v-7l6 7v-7" }
            path { d: "M7.5 10.5h9" }
            path { d: "M7.5 13h9" }
        }
    }
}

#[component]
pub fn BitcoinIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "8.5" }
            path { d: "M9.5 7.5h3.25a2.25 2.25 0 1 1 0 4.5H9.5h4a2.25 2.25 0 1 1 0 4.5H9.5v-9Z" }
            path { d: "M11 6v1.5M11 16.5V18M13.25 6v1.5M13.25 16.5V18" }
        }
    }
}

#[component]
pub fn NetworkIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "5", r: "2.25" }
            circle { cx: "5", cy: "18", r: "2.25" }
            circle { cx: "19", cy: "18", r: "2.25" }
            path { d: "M10.8 7 6.2 15.8M13.2 7l4.6 8.8M7.25 18h9.5" }
        }
    }
}

#[component]
pub fn ShieldIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 3.5 19 6v5.2c0 4.2-2.7 7.5-7 9.3-4.3-1.8-7-5.1-7-9.3V6l7-2.5Z" }
            path { d: "m9 12 2 2 4-4" }
        }
    }
}

#[component]
pub fn SendReceiveIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M7 7h11l-3-3M17 17H6l3 3" }
            path { d: "M18 7 7 18" }
        }
    }
}

#[component]
pub fn ClockIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "8.5" }
            path { d: "M12 7.5V12l3 2" }
        }
    }
}

#[component]
pub fn ArrowRightIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M4.5 12h15M13.5 6l6 6-6 6" }
        }
    }
}

#[component]
pub fn ArrowLeftIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M19.5 12h-15M10.5 6l-6 6 6 6" }
        }
    }
}

#[component]
pub fn SettingsIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "3" }
            path { d: "M19.2 13.5a7.7 7.7 0 0 0 0-3l1.5-1.2-1.8-3.1-1.8.7a7.5 7.5 0 0 0-2.6-1.5L14.2 3h-4.4l-.3 2.4a7.5 7.5 0 0 0-2.6 1.5l-1.8-.7-1.8 3.1L4.8 10.5a7.7 7.7 0 0 0 0 3l-1.5 1.2 1.8 3.1 1.8-.7a7.5 7.5 0 0 0 2.6 1.5l.3 2.4h4.4l.3-2.4a7.5 7.5 0 0 0 2.6-1.5l1.8.7 1.8-3.1-1.5-1.2Z" }
        }
    }
}

#[component]
pub fn CheckIcon() -> Element {
    rsx! {
        svg {
            class: "icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "m5 12.5 4.5 4.5L19 7" }
        }
    }
}
