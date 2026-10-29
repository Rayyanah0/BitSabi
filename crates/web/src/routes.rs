//! Application routes.

use dioxus::prelude::*;

use crate::{
    components::AppShell,
    pages::{Home, Learn, LessonDetail, NotFound, Payments, Settings, Wallet},
};

#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
        #[route("/")]
        Home {},
        #[route("/learn")]
        Learn {},
        #[route("/learn/:slug")]
        LessonDetail { slug: String },
        #[route("/wallet")]
        Wallet {},
        #[route("/payments")]
        Payments {},
        #[route("/settings")]
        Settings {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}
