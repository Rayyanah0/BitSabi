//! BitSabi API library.
//!
//! The API proxies public testnet chain data and broadcast requests. It must
//! never receive, log, or store wallet recovery phrases, seeds, or private
//! keys.

pub mod api;
pub mod chain;
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

use crate::{chain::ChainClient, config::Config};

/// Build the full API application.
pub fn app(config: &Config) -> Router {
    let chain = ChainClient::new(
        &config.testnet_api_url,
        config.testnet_api_fallback_url.as_deref(),
    );

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(config.frontend_origins.clone()))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    api::router(chain)
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
