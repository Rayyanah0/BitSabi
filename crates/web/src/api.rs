//! Browser client for the BitSabi testnet proxy.
//!
//! Only public addresses and signed transactions are sent to the API.

use std::{fmt, marker::PhantomData};

use bitsabi_shared::{
    ApiErrorResponse, BroadcastTransactionRequest, BroadcastTransactionResponse, WalletSnapshot,
    WalletSnapshotRequest,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(target_arch = "wasm32")]
const DEFAULT_API_BASE: &str = "http://127.0.0.1:3000";
const SNAPSHOT_PATH: &str = "/api/testnet/wallet/snapshot";
const BROADCAST_PATH: &str = "/api/testnet/broadcast";

/// Safe API failure shown in the wallet UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// User-facing error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

/// Fetch chain data for the supplied public wallet addresses.
pub async fn fetch_wallet_snapshot(addresses: Vec<String>) -> Result<WalletSnapshot, ApiError> {
    post_json(
        SNAPSHOT_PATH,
        &WalletSnapshotRequest { addresses },
    )
    .await
}

/// Broadcast a locally signed testnet transaction.
pub async fn broadcast_transaction(
    raw_transaction_hex: String,
) -> Result<BroadcastTransactionResponse, ApiError> {
    post_json(
        BROADCAST_PATH,
        &BroadcastTransactionRequest {
            raw_transaction_hex,
        },
    )
    .await
}

#[cfg(target_arch = "wasm32")]
fn api_base() -> String {
    option_env!("BITSABI_API_URL")
        .unwrap_or(DEFAULT_API_BASE)
        .trim_end_matches('/')
        .to_owned()
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_api_response<T: DeserializeOwned>(
    status: u16,
    body: &str,
) -> Result<T, ApiError> {
    if !(200..300).contains(&status) {
        let message = serde_json::from_str::<ApiErrorResponse>(body)
            .map(|error| error.error)
            .unwrap_or_else(|_| {
                "The testnet service returned an unexpected error. Please try again.".to_owned()
            });
        return Err(ApiError::new(message));
    }

    serde_json::from_str::<T>(body).map_err(|_| {
        ApiError::new("The testnet service returned an unreadable response. Please try again.")
    })
}

#[cfg(target_arch = "wasm32")]
async fn post_json<S, T>(path: &str, body: &S) -> Result<T, ApiError>
where
    S: Serialize,
    T: DeserializeOwned,
{
    let url = format!("{}{path}", api_base());
    let request = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|_| ApiError::new("The request could not be prepared."))?;
    let response = request.send().await.map_err(|_| {
        ApiError::new(
            "Could not reach the BitSabi testnet service. Check that the API is running and try again.",
        )
    })?;
    let status = response.status();
    let body = response.text().await.map_err(|_| {
        ApiError::new("The testnet service response could not be read. Please try again.")
    })?;

    parse_api_response(status, &body)
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json<S, T>(_path: &str, _body: &S) -> Result<T, ApiError>
where
    S: Serialize,
    T: DeserializeOwned,
{
    let _ = PhantomData::<T>;
    Err(ApiError::new(
        "Network requests are available in the browser wallet.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_responses_are_deserialized() {
        let response: BroadcastTransactionResponse =
            parse_api_response(200, r#"{"txid":"abc"}"#).unwrap();
        assert_eq!(response.txid, "abc");
    }

    #[test]
    fn api_errors_use_the_server_message() {
        let error = parse_api_response::<WalletSnapshot>(
            422,
            r#"{"error":"The transaction was rejected."}"#,
        )
        .unwrap_err();

        assert_eq!(error.message(), "The transaction was rejected.");
    }

    #[test]
    fn malformed_errors_and_responses_are_safe() {
        let error = parse_api_response::<WalletSnapshot>(500, "<html>").unwrap_err();
        assert!(error.message().contains("unexpected error"));

        let error = parse_api_response::<WalletSnapshot>(200, "not json").unwrap_err();
        assert!(error.message().contains("unreadable"));
    }
}
