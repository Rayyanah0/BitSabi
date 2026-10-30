//! HTTP route handlers.
//!
//! Route handlers receive only public testnet addresses and signed
//! transactions. Wallet recovery phrases, seeds, and private keys never reach
//! this API.

use std::{collections::HashSet, str::FromStr};

use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bitcoin::{
    address::Address,
    consensus::encode::{deserialize_hex, serialize},
    Amount, Network, Transaction, Txid,
};
use bitsabi_shared::{
    ApiErrorResponse, BroadcastTransactionRequest, BroadcastTransactionResponse, HealthResponse,
    WalletSnapshot, WalletSnapshotRequest, MAX_RAW_TRANSACTION_BYTES, MAX_SNAPSHOT_ADDRESSES,
};
use tracing::warn;

use crate::chain::{ChainClient, ChainError};

/// Build the API router.
pub fn router(chain: ChainClient) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/testnet/wallet/snapshot", post(wallet_snapshot))
        .route("/api/testnet/broadcast", post(broadcast_transaction))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_RAW_TRANSACTION_BYTES * 2 + 1024))
        .with_state(chain)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::healthy())
}

async fn wallet_snapshot(
    State(chain): State<ChainClient>,
    Json(request): Json<WalletSnapshotRequest>,
) -> Result<Json<WalletSnapshot>, ApiError> {
    let addresses = validate_addresses(&request.addresses)?;
    let snapshot = chain
        .wallet_snapshot(&addresses)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(snapshot))
}

async fn broadcast_transaction(
    State(chain): State<ChainClient>,
    Json(request): Json<BroadcastTransactionRequest>,
) -> Result<Json<BroadcastTransactionResponse>, ApiError> {
    let (raw_transaction, expected_txid) = validate_raw_transaction(&request.raw_transaction_hex)?;
    let provider_txid = chain
        .broadcast(&raw_transaction)
        .await
        .map_err(ApiError::from)?;
    let provider_txid = Txid::from_str(provider_txid.trim())
        .map_err(|_| ApiError::Chain(ChainError::InvalidResponse))?;

    if provider_txid != expected_txid {
        return Err(ApiError::Chain(ChainError::InvalidResponse));
    }

    Ok(Json(BroadcastTransactionResponse {
        txid: provider_txid.to_string(),
    }))
}

fn validate_addresses(raw_addresses: &[String]) -> Result<Vec<String>, ApiError> {
    if raw_addresses.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Add at least one testnet wallet address.".to_owned(),
        ));
    }
    if raw_addresses.len() > MAX_SNAPSHOT_ADDRESSES {
        return Err(ApiError::InvalidRequest(format!(
            "A snapshot can include at most {MAX_SNAPSHOT_ADDRESSES} addresses."
        )));
    }

    let mut seen = HashSet::with_capacity(raw_addresses.len());
    let mut addresses = Vec::with_capacity(raw_addresses.len());

    for raw_address in raw_addresses {
        let address = Address::from_str(raw_address.trim()).map_err(|_| {
            ApiError::InvalidRequest(
                "Every wallet address must be a valid Bitcoin testnet address.".to_owned(),
            )
        })?;
        let address = address.require_network(Network::Testnet).map_err(|_| {
            ApiError::InvalidRequest("Only Bitcoin testnet addresses are supported.".to_owned())
        })?;
        let address = address.to_string();

        if seen.insert(address.clone()) {
            addresses.push(address);
        }
    }

    if addresses.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Add at least one testnet wallet address.".to_owned(),
        ));
    }

    Ok(addresses)
}

fn validate_raw_transaction(raw_hex: &str) -> Result<(String, Txid), ApiError> {
    let raw_hex = raw_hex.trim();
    if raw_hex.is_empty() {
        return Err(ApiError::InvalidRequest(
            "A signed transaction is required.".to_owned(),
        ));
    }
    if raw_hex.len() > MAX_RAW_TRANSACTION_BYTES * 2 {
        return Err(ApiError::InvalidRequest(
            "The signed transaction is too large.".to_owned(),
        ));
    }
    if raw_hex.len() & 1 == 1 {
        return Err(ApiError::InvalidRequest(
            "The signed transaction is not valid hex.".to_owned(),
        ));
    }

    let transaction = deserialize_hex::<Transaction>(raw_hex).map_err(|_| {
        ApiError::InvalidRequest("The signed transaction could not be decoded.".to_owned())
    })?;

    if transaction.input.is_empty() || transaction.output.is_empty() {
        return Err(ApiError::InvalidRequest(
            "The signed transaction has no inputs or outputs.".to_owned(),
        ));
    }
    if transaction.is_coinbase() {
        return Err(ApiError::InvalidRequest(
            "Coinbase transactions cannot be broadcast.".to_owned(),
        ));
    }
    if serialize(&transaction).len() > MAX_RAW_TRANSACTION_BYTES {
        return Err(ApiError::InvalidRequest(
            "The signed transaction is too large.".to_owned(),
        ));
    }

    let total_out = transaction.output.iter().try_fold(0_u64, |total, output| {
        total.checked_add(output.value.to_sat())
    });
    if total_out.is_none_or(|total| total > Amount::MAX_MONEY.to_sat()) {
        return Err(ApiError::InvalidRequest(
            "The signed transaction has invalid output values.".to_owned(),
        ));
    }

    Ok((raw_hex.to_owned(), transaction.compute_txid()))
}

#[derive(Debug)]
enum ApiError {
    InvalidRequest(String),
    Chain(ChainError),
}

impl From<ChainError> for ApiError {
    fn from(error: ChainError) -> Self {
        warn!(%error, "testnet chain request failed");
        Self::Chain(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::InvalidRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Chain(ChainError::BroadcastRejected(message)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, message)
            }
            Self::Chain(_) => (
                StatusCode::BAD_GATEWAY,
                "Testnet chain data is temporarily unavailable. Please try again.".to_owned(),
            ),
        };

        (status, Json(ApiErrorResponse { error })).into_response()
    }
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ApiErrorResponse {
            error: "Route not found".to_owned(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        extract::Path,
        http::{Request, StatusCode},
    };
    use bitcoin::{
        absolute::LockTime,
        hashes::Hash,
        transaction::Version,
        transaction::{OutPoint, TxIn, TxOut},
        ScriptBuf, Sequence, Witness,
    };
    use bitsabi_shared::{ApiErrorResponse, WalletSnapshot};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::config::Config;

    const TESTNET_ADDRESS: &str = "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl";
    const MAINNET_ADDRESS: &str = "bc1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl";

    fn unsigned_transaction() -> Transaction {
        Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::all_zeros(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(1_000),
                script_pubkey: ScriptBuf::new(),
            }],
        }
    }

    #[test]
    fn addresses_must_be_testnet_and_are_deduplicated() {
        let addresses =
            validate_addresses(&[TESTNET_ADDRESS.to_owned(), format!(" {TESTNET_ADDRESS} ")])
                .unwrap();

        assert_eq!(addresses, vec![TESTNET_ADDRESS.to_owned()]);
    }

    #[test]
    fn addresses_reject_mainnet_and_empty_inputs() {
        assert!(validate_addresses(&[]).is_err());
        assert!(validate_addresses(&[MAINNET_ADDRESS.to_owned()]).is_err());
        assert!(validate_addresses(&["not-an-address".to_owned()]).is_err());
    }

    #[test]
    fn raw_transaction_validation_returns_the_computed_txid() {
        let transaction = unsigned_transaction();
        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        let (validated_hex, txid) = validate_raw_transaction(&raw_hex).unwrap();

        assert_eq!(validated_hex, raw_hex);
        assert_eq!(txid, transaction.compute_txid());
    }

    #[test]
    fn raw_transaction_validation_rejects_bad_hex_and_empty_transactions() {
        assert!(validate_raw_transaction("").is_err());
        assert!(validate_raw_transaction("abc").is_err());

        let mut transaction = unsigned_transaction();
        transaction.output.clear();
        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(validate_raw_transaction(&raw_hex).is_err());
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let config = Config::for_test();
        let response = crate::app(&config)
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "ok");
        assert_eq!(health.service, "bitsabi-api");
    }

    #[tokio::test]
    async fn snapshot_rejects_mainnet_addresses_without_calling_a_provider() {
        let config = Config::for_test();
        let request_body = serde_json::json!({ "addresses": [MAINNET_ADDRESS] });

        let response = crate::app(&config)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/wallet/snapshot")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let error: ApiErrorResponse = serde_json::from_slice(&body).unwrap();
        assert!(error.error.contains("testnet"));
    }

    #[tokio::test]
    async fn snapshot_and_broadcast_use_the_configured_provider() {
        let transaction = unsigned_transaction();
        let txid = transaction.compute_txid().to_string();
        let provider_url = spawn_mock_provider(txid.clone()).await;

        let mut config = Config::for_test();
        config.testnet_api_url = provider_url;
        let app = crate::app(&config);

        let snapshot_body = serde_json::json!({ "addresses": [TESTNET_ADDRESS] });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/wallet/snapshot")
                    .header("content-type", "application/json")
                    .body(Body::from(snapshot_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let snapshot: WalletSnapshot = serde_json::from_slice(&body).unwrap();
        assert_eq!(snapshot.tip_height, 850_000);
        assert_eq!(snapshot.fees.unwrap().fastest_sat_vb, 2);
        assert_eq!(snapshot.addresses.len(), 1);
        assert_eq!(snapshot.addresses[0].utxos[0].value, 40_000);
        assert_eq!(snapshot.addresses[0].transactions[0].net_value, 40_000);

        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let broadcast_body = serde_json::json!({ "raw_transaction_hex": raw_hex });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/broadcast")
                    .header("content-type", "application/json")
                    .body(Body::from(broadcast_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let broadcast: BroadcastTransactionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(broadcast.txid, txid);
    }

    #[tokio::test]
    async fn unknown_route_returns_json_404() {
        let config = Config::for_test();
        let response = crate::app(&config)
            .oneshot(
                Request::builder()
                    .uri("/api/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    async fn spawn_mock_provider(txid: String) -> String {
        async fn stats() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "chain_stats": { "funded_txo_sum": 50_000, "spent_txo_sum": 10_000, "tx_count": 1 },
                "mempool_stats": { "funded_txo_sum": 2_000, "spent_txo_sum": 0, "tx_count": 1 }
            }))
        }

        async fn utxos() -> Json<serde_json::Value> {
            Json(serde_json::json!([{
                "txid": "1111111111111111111111111111111111111111111111111111111111111111",
                "vout": 0,
                "value": 40_000,
                "status": { "confirmed": true, "block_height": 849_990, "block_time": 1_700_000_000 }
            }]))
        }

        async fn transactions(Path(address): Path<String>) -> Json<serde_json::Value> {
            Json(serde_json::json!([{
                "txid": "2222222222222222222222222222222222222222222222222222222222222222",
                "fee": 200,
                "status": { "confirmed": true, "block_height": 849_990, "block_time": 1_700_000_000 },
                "vin": [{
                    "prevout": { "scriptpubkey_address": "tb1qexternal", "value": 60_000 }
                }],
                "vout": [{
                    "scriptpubkey_address": address,
                    "value": 40_000
                }]
            }]))
        }

        async fn fees() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "1": 1.01, "3": 1.4, "6": 1.2, "144": 1.0, "1008": 1.0
            }))
        }

        async fn tip() -> &'static str {
            "850000"
        }

        async fn broadcast(State(txid): State<String>) -> String {
            txid
        }

        let app = Router::new()
            .route("/address/{address}", get(stats))
            .route("/address/{address}/utxo", get(utxos))
            .route("/address/{address}/txs", get(transactions))
            .route("/fee-estimates", get(fees))
            .route("/blocks/tip/height", get(tip))
            .route("/tx", post(broadcast))
            .with_state(txid);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        format!("http://{address}")
    }
}
