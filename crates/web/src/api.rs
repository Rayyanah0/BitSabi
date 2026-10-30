//! Browser client for the BitSabi testnet proxy.
//!
//! Only public addresses and signed transactions are sent to the API.

use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::marker::PhantomData;

#[cfg(any(target_arch = "wasm32", test))]
use bitsabi_shared::ApiErrorResponse;
use bitsabi_shared::{
    BroadcastTransactionRequest, BroadcastTransactionResponse, WalletSnapshot,
    WalletSnapshotRequest, MAX_SNAPSHOT_ADDRESSES,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(target_arch = "wasm32")]
const DEFAULT_API_BASE: &str = "http://127.0.0.1:3000";
const SNAPSHOT_PATH: &str = "/api/testnet/wallet/snapshot";
const BROADCAST_PATH: &str = "/api/testnet/broadcast";

/// Broad category for a safe API failure shown in the wallet UI.
#[cfg_attr(
    not(any(target_arch = "wasm32", test)),
    allow(dead_code, reason = "responses are parsed in browser builds")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// The API could not be reached.
    Network,
    /// The API answered with an error status.
    Server,
    /// The API answered with unreadable data.
    InvalidResponse,
}

/// Safe API failure shown in the wallet UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    kind: ApiErrorKind,
    message: String,
}

impl ApiError {
    fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// User-facing error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Failure category used to choose an appropriate UI state.
    #[must_use]
    pub fn kind(&self) -> ApiErrorKind {
        self.kind
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

/// Fetch chain data for the supplied public wallet addresses.
///
/// Large wallets are split into batches that respect the server's address
/// limit. Batches are merged into one snapshot for the UI.
pub async fn fetch_wallet_snapshot(addresses: Vec<String>) -> Result<WalletSnapshot, ApiError> {
    let mut snapshots = Vec::new();
    for chunk in addresses.chunks(MAX_SNAPSHOT_ADDRESSES) {
        snapshots.push(
            post_json(
                SNAPSHOT_PATH,
                &WalletSnapshotRequest {
                    addresses: chunk.to_vec(),
                },
            )
            .await?,
        );
    }

    merge_snapshots(snapshots).ok_or_else(|| {
        ApiError::new(
            ApiErrorKind::InvalidResponse,
            "The testnet service returned no wallet data. Please try again.",
        )
    })
}

fn merge_snapshots(snapshots: Vec<WalletSnapshot>) -> Option<WalletSnapshot> {
    let mut snapshots = snapshots.into_iter();
    let mut merged = snapshots.next()?;

    for snapshot in snapshots {
        merged.tip_height = merged.tip_height.max(snapshot.tip_height);
        merged.fetched_at = merged.fetched_at.max(snapshot.fetched_at);
        if merged.fees.is_none() {
            merged.fees = snapshot.fees;
        }
        merged.addresses.extend(snapshot.addresses);
    }

    Some(merged)
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
fn parse_api_response<T: DeserializeOwned>(status: u16, body: &str) -> Result<T, ApiError> {
    if !(200..300).contains(&status) {
        let message = serde_json::from_str::<ApiErrorResponse>(body)
            .map(|error| error.error)
            .unwrap_or_else(|_| {
                "The testnet service returned an unexpected error. Please try again.".to_owned()
            });
        return Err(ApiError::new(ApiErrorKind::Server, message));
    }

    serde_json::from_str::<T>(body).map_err(|_| {
        ApiError::new(
            ApiErrorKind::InvalidResponse,
            "The testnet service returned an unreadable response. Please try again.",
        )
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
        .map_err(|_| {
            ApiError::new(
                ApiErrorKind::InvalidResponse,
                "The request could not be prepared.",
            )
        })?;
    let response = request.send().await.map_err(|_| {
        ApiError::new(
            ApiErrorKind::Network,
            "Could not reach the BitSabi testnet service. Check your connection and try again.",
        )
    })?;
    let status = response.status();
    let body = response.text().await.map_err(|_| {
        ApiError::new(
            ApiErrorKind::InvalidResponse,
            "The testnet service response could not be read. Please try again.",
        )
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
        ApiErrorKind::Network,
        "Network requests are available in the browser wallet.",
    ))
}

/// Whether the browser reports that it is offline.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn browser_is_offline() -> bool {
    web_sys::window().is_some_and(|window| !window.navigator().on_line())
}

/// Whether the browser reports that it is offline.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn browser_is_offline() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use bitsabi_shared::{AddressSnapshot, FeeEstimates};

    use super::*;

    fn snapshot(
        tip_height: u64,
        fetched_at: u64,
        fees: Option<FeeEstimates>,
        address: Option<&str>,
    ) -> WalletSnapshot {
        WalletSnapshot {
            provider: "test".to_owned(),
            tip_height,
            fetched_at,
            fees,
            addresses: address
                .map(|address| AddressSnapshot {
                    address: address.to_owned(),
                    stats: Default::default(),
                    utxos: Vec::new(),
                    transactions: Vec::new(),
                    history_truncated: false,
                })
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn batched_snapshots_merge_addresses_and_optional_metadata() {
        let fees = FeeEstimates {
            fastest_sat_vb: 3,
            half_hour_sat_vb: 2,
            hour_sat_vb: 2,
            economy_sat_vb: 1,
            minimum_sat_vb: 1,
        };
        let merged = merge_snapshots(vec![
            snapshot(100, 10, None, Some("a")),
            snapshot(102, 12, Some(fees), Some("b")),
        ])
        .unwrap();

        assert_eq!(merged.tip_height, 102);
        assert_eq!(merged.fetched_at, 12);
        assert_eq!(merged.fees, Some(fees));
        assert_eq!(merged.addresses.len(), 2);
    }

    #[test]
    fn empty_batches_are_rejected() {
        assert!(merge_snapshots(Vec::new()).is_none());
    }

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
        assert_eq!(error.kind(), ApiErrorKind::Server);
    }

    #[test]
    fn malformed_errors_and_responses_are_safe() {
        let error = parse_api_response::<WalletSnapshot>(500, "<html>").unwrap_err();
        assert!(error.message().contains("unexpected error"));
        assert_eq!(error.kind(), ApiErrorKind::Server);

        let error = parse_api_response::<WalletSnapshot>(200, "not json").unwrap_err();
        assert!(error.message().contains("unreadable"));
        assert_eq!(error.kind(), ApiErrorKind::InvalidResponse);
    }

    #[test]
    fn offline_detection_is_disabled_outside_the_browser() {
        assert!(!browser_is_offline());
    }
}
