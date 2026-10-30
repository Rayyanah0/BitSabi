//! Environment-based server configuration.
//!
//! Configuration lives in environment variables so that credentials are never
//! committed to Git. See `.env.example` for the expected variables.

use std::{net::SocketAddr, str::FromStr};

use axum::http::HeaderValue;
use thiserror::Error;

/// The environment the API is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    /// Lowercase name used in logs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Production => "production",
        }
    }
}

impl FromStr for Environment {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" => Ok(Self::Production),
            other => Err(ConfigError::Environment(other.to_owned())),
        }
    }
}

/// Configuration required to start the API.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind_addr: SocketAddr,
    /// Browser origins allowed to call the API.
    pub frontend_origins: Vec<HeaderValue>,
    /// Current deployment environment.
    pub environment: Environment,
    /// Bitcoin network. This build accepts only `testnet`.
    pub bitcoin_network: String,
    /// Primary Esplora-compatible testnet API base URL.
    pub testnet_api_url: String,
    /// Optional fallback Esplora-compatible testnet API base URL.
    pub testnet_api_fallback_url: Option<String>,
}

impl Config {
    /// Read configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
            .parse::<SocketAddr>()
            .map_err(ConfigError::BindAddress)?;

        let raw_origins = std::env::var("FRONTEND_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:8080,http://127.0.0.1:8080".to_owned());
        let frontend_origins = parse_origins(&raw_origins)?;

        let environment = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_owned())
            .parse()?;

        let bitcoin_network =
            std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "testnet".to_owned());
        if bitcoin_network != "testnet" {
            return Err(ConfigError::UnsupportedNetwork(bitcoin_network));
        }

        let testnet_api_url = std::env::var("TESTNET_API_URL")
            .unwrap_or_else(|_| "https://blockstream.info/testnet/api".to_owned());
        validate_http_url("TESTNET_API_URL", &testnet_api_url)?;

        let testnet_api_fallback_url = std::env::var("TESTNET_API_FALLBACK_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        if let Some(url) = testnet_api_fallback_url.as_deref() {
            validate_http_url("TESTNET_API_FALLBACK_URL", url)?;
        }

        Ok(Self {
            bind_addr,
            frontend_origins,
            environment,
            bitcoin_network,
            testnet_api_url: testnet_api_url.trim_end_matches('/').to_owned(),
            testnet_api_fallback_url: testnet_api_fallback_url
                .map(|url| url.trim_end_matches('/').to_owned()),
        })
    }

    /// Deterministic configuration for tests.
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
            frontend_origins: vec![
                HeaderValue::from_static("http://localhost:8080"),
                HeaderValue::from_static("http://127.0.0.1:8080"),
            ],
            environment: Environment::Test,
            bitcoin_network: "testnet".to_owned(),
            testnet_api_url: "http://127.0.0.1:1".to_owned(),
            testnet_api_fallback_url: None,
        }
    }
}

fn parse_origins(raw: &str) -> Result<Vec<HeaderValue>, ConfigError> {
    let mut origins = Vec::new();
    for origin in raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        origins.push(HeaderValue::from_str(origin).map_err(ConfigError::FrontendOrigin)?);
    }

    if origins.is_empty() {
        return Err(ConfigError::NoFrontendOrigins);
    }

    Ok(origins)
}

fn validate_http_url(name: &'static str, value: &str) -> Result<(), ConfigError> {
    let url = reqwest::Url::parse(value).map_err(|_| ConfigError::ApiUrl(name))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ConfigError::ApiUrl(name));
    }
    Ok(())
}

/// Problems found while reading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("BIND_ADDR must be a valid IP address and port")]
    BindAddress(#[source] std::net::AddrParseError),
    #[error("FRONTEND_ORIGIN must contain at least one valid HTTP origin")]
    FrontendOrigin(#[source] axum::http::header::InvalidHeaderValue),
    #[error("FRONTEND_ORIGIN must contain at least one origin")]
    NoFrontendOrigins,
    #[error("APP_ENV must be development, test, or production (got {0})")]
    Environment(String),
    #[error("BITCOIN_NETWORK must be testnet (got {0})")]
    UnsupportedNetwork(String),
    #[error("{0} must be a valid HTTP or HTTPS URL")]
    ApiUrl(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_parsing_accepts_known_values() {
        assert_eq!(
            "development".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!("test".parse::<Environment>().unwrap(), Environment::Test);
        assert_eq!(
            "production".parse::<Environment>().unwrap(),
            Environment::Production
        );
    }

    #[test]
    fn environment_parsing_rejects_unknown_values() {
        assert!("staging".parse::<Environment>().is_err());
    }

    #[test]
    fn origins_accept_a_comma_separated_allow_list() {
        let origins = parse_origins("http://localhost:8080, http://127.0.0.1:8080,,").unwrap();

        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn origins_reject_an_empty_allow_list() {
        assert!(matches!(
            parse_origins(" , "),
            Err(ConfigError::NoFrontendOrigins)
        ));
    }

    #[test]
    fn api_urls_reject_non_http_values() {
        assert!(validate_http_url("TESTNET_API_URL", "file:///tmp/api").is_err());
        assert!(validate_http_url("TESTNET_API_URL", "not a url").is_err());
        assert!(validate_http_url("TESTNET_API_URL", "https://example.com/api").is_ok());
    }
}
