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
    /// Browser origin allowed to call the API.
    pub frontend_origin: HeaderValue,
    /// Current deployment environment.
    pub environment: Environment,
}

impl Config {
    /// Read configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
            .parse::<SocketAddr>()
            .map_err(ConfigError::BindAddress)?;

        let raw_origin =
            std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:8080".to_owned());
        let frontend_origin =
            HeaderValue::from_str(&raw_origin).map_err(ConfigError::FrontendOrigin)?;

        let environment = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "development".to_owned())
            .parse()?;

        Ok(Self {
            bind_addr,
            frontend_origin,
            environment,
        })
    }

    /// Deterministic configuration for tests.
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
            frontend_origin: HeaderValue::from_static("http://localhost:8080"),
            environment: Environment::Test,
        }
    }
}

/// Problems found while reading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("BIND_ADDR must be a valid IP address and port")]
    BindAddress(#[source] std::net::AddrParseError),
    #[error("FRONTEND_ORIGIN must be a single valid HTTP origin")]
    FrontendOrigin(#[source] axum::http::header::InvalidHeaderValue),
    #[error("APP_ENV must be development, test, or production (got {0})")]
    Environment(String),
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
}
