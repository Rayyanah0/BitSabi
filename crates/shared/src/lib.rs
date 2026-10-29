//! Types and constants shared by the BitSabi API and web frontend.
//!
//! Keep this crate free of secrets and platform-specific code. It is compiled
//! for both the native Axum server and the WebAssembly browser app.

use serde::{Deserialize, Serialize};

/// Product name used throughout the interface.
pub const PRODUCT_NAME: &str = "BitSabi";

/// Short product tagline.
pub const TAGLINE: &str = "Understand Bitcoin. Take Control.";

/// One-sentence explanation for the home screen.
pub const APP_DESCRIPTION: &str =
    "A simple place to learn Bitcoin, try a self-custody wallet, and understand NGN payments.";

/// Response returned by `GET /api/health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `"ok"` while the API is able to answer requests.
    pub status: String,
    /// Name of the API service.
    pub service: String,
    /// Server crate version.
    pub version: String,
}

impl HealthResponse {
    /// Build a healthy response for the current build.
    #[must_use]
    pub fn healthy() -> Self {
        Self {
            status: "ok".to_owned(),
            service: "bitsabi-api".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}
