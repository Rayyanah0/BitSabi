//! BitSabi API library.
//!
//! The API currently exposes health information only. It must never receive,
//! log, or store wallet recovery phrases or private keys.

pub mod api;
pub mod config;

use axum::{
    http::{header, HeaderValue, Method},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::config::Config;

/// Build the full API application.
pub fn app(config: &Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(config.frontend_origin.clone()))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    api::router()
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(TraceLayer::new_for_http())
}
