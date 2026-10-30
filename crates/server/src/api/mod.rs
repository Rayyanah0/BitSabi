//! HTTP route handlers.
//!
//! Route handlers receive only public testnet addresses and signed
//! transactions. Wallet recovery phrases, seeds, and private keys never reach
//! this API.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

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
    let state = AppState {
        chain,
        broadcasts: BroadcastTracker::default(),
    };

    Router::new()
        .route("/api/health", get(health))
        .route("/api/testnet/wallet/snapshot", post(wallet_snapshot))
        .route("/api/testnet/broadcast", post(broadcast_transaction))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_RAW_TRANSACTION_BYTES * 2 + 1024))
        .with_state(state)
}

#[derive(Clone)]
struct AppState {
    chain: ChainClient,
    broadcasts: BroadcastTracker,
}

const BROADCAST_MEMORY: Duration = Duration::from_secs(10 * 60);
const MAX_ACCEPTED_BROADCASTS: usize = 512;

/// In-memory tracker for broadcast duplicate protection.
///
/// This only protects one running server process. It is not a database and does
/// not provide permanent or cross-process duplicate protection.
#[derive(Clone, Default)]
struct BroadcastTracker {
    inner: Arc<Mutex<BroadcastTrackerInner>>,
}

#[derive(Default)]
struct BroadcastTrackerInner {
    in_flight: HashSet<Txid>,
    accepted: HashMap<Txid, Instant>,
    uncertain: HashMap<Txid, Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BroadcastRegistration {
    Start,
    InFlight,
    AlreadyAccepted,
    Uncertain,
}

impl BroadcastTracker {
    fn begin(&self, txid: Txid) -> BroadcastRegistration {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        inner
            .accepted
            .retain(|_, accepted_at| now.duration_since(*accepted_at) < BROADCAST_MEMORY);
        inner
            .uncertain
            .retain(|_, recorded_at| now.duration_since(*recorded_at) < BROADCAST_MEMORY);

        if inner.uncertain.contains_key(&txid) {
            return BroadcastRegistration::Uncertain;
        }
        if inner.accepted.contains_key(&txid) {
            return BroadcastRegistration::AlreadyAccepted;
        }
        if !inner.in_flight.insert(txid) {
            return BroadcastRegistration::InFlight;
        }

        // Preserve protection for recent transactions by evicting only the oldest
        // entries when the bounded cache is full.
        while inner.accepted.len() >= MAX_ACCEPTED_BROADCASTS {
            let oldest = inner
                .accepted
                .iter()
                .min_by_key(|(_, accepted_at)| *accepted_at)
                .map(|(txid, _)| *txid);
            match oldest {
                Some(txid) => {
                    inner.accepted.remove(&txid);
                }
                None => break,
            }
        }

        BroadcastRegistration::Start
    }

    fn succeeded(&self, txid: Txid) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.in_flight.remove(&txid);
        inner.accepted.insert(txid, Instant::now());
    }

    fn failed(&self, txid: Txid) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.in_flight.remove(&txid);
    }

    fn uncertain(&self, txid: Txid) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.in_flight.remove(&txid);
        inner.uncertain.insert(txid, Instant::now());
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::healthy())
}

async fn wallet_snapshot(
    State(state): State<AppState>,
    Json(request): Json<WalletSnapshotRequest>,
) -> Result<Json<WalletSnapshot>, ApiError> {
    let addresses = validate_addresses(&request.addresses)?;
    let snapshot = state
        .chain
        .wallet_snapshot(&addresses)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(snapshot))
}

async fn broadcast_transaction(
    State(state): State<AppState>,
    Json(request): Json<BroadcastTransactionRequest>,
) -> Result<Json<BroadcastTransactionResponse>, ApiError> {
    let (raw_transaction, expected_txid) = validate_raw_transaction(&request.raw_transaction_hex)?;

    match state.broadcasts.begin(expected_txid) {
        BroadcastRegistration::AlreadyAccepted => {
            return Ok(Json(BroadcastTransactionResponse {
                txid: expected_txid.to_string(),
            }));
        }
        BroadcastRegistration::InFlight => {
            return Err(ApiError::InvalidRequest(
                "This transaction is already being broadcast.".to_owned(),
            ));
        }
        BroadcastRegistration::Start => {}
        BroadcastRegistration::Uncertain => {
            return Err(ApiError::Chain(ChainError::BroadcastOutcomeUncertain));
        }
    }

    let result = state.chain.broadcast(&raw_transaction).await;
    let provider_txid = match result {
        Ok(provider_txid) => provider_txid,
        Err(ChainError::BroadcastRejected(message)) => {
            state.broadcasts.failed(expected_txid);
            return Err(ApiError::Chain(ChainError::BroadcastRejected(message)));
        }
        Err(_) => {
            // A network or provider failure is not proof that the transaction was
            // rejected; the provider may have accepted it before the response was
            // lost. Mark it uncertain and prevent a blind retry.
            state.broadcasts.uncertain(expected_txid);
            return Err(ApiError::Chain(ChainError::BroadcastOutcomeUncertain));
        }
    };
    let provider_txid = match Txid::from_str(provider_txid.trim()) {
        Ok(provider_txid) if provider_txid == expected_txid => provider_txid,
        _ => {
            state.broadcasts.failed(expected_txid);
            return Err(ApiError::Chain(ChainError::InvalidResponse));
        }
    };

    state.broadcasts.succeeded(expected_txid);
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
    if transaction
        .input
        .iter()
        .any(|input| input.witness.is_empty() && input.script_sig.is_empty())
    {
        return Err(ApiError::InvalidRequest(
            "The transaction has unsigned inputs.".to_owned(),
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
            Self::Chain(ChainError::BroadcastOutcomeUncertain) => (
                StatusCode::CONFLICT,
                "The broadcast may have been accepted. Check the transaction status before retrying."
                    .to_owned(),
            ),
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
                witness: Witness::from_slice(&[vec![0_u8; 72], vec![2_u8; 33]]),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(1_000),
                script_pubkey: ScriptBuf::new(),
            }],
        }
    }

    #[test]
    fn broadcast_tracker_rejects_in_flight_and_repeated_submissions() {
        let tracker = BroadcastTracker::default();
        let txid =
            Txid::from_str("1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::InFlight);

        tracker.succeeded(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::AlreadyAccepted);

        tracker.failed(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::AlreadyAccepted);
    }

    #[test]
    fn failed_broadcasts_can_be_retried() {
        let tracker = BroadcastTracker::default();
        let txid =
            Txid::from_str("2222222222222222222222222222222222222222222222222222222222222222")
                .unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        tracker.failed(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
    }

    #[test]
    fn accepted_broadcast_capacity_evicts_oldest_entry() {
        let tracker = BroadcastTracker::default();

        for index in 0..MAX_ACCEPTED_BROADCASTS {
            let txid = Txid::from_str(&format!("{:064x}", index + 1)).unwrap();
            assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
            tracker.succeeded(txid);
        }

        let oldest = Txid::from_str(&format!("{:064x}", 1)).unwrap();
        let next = Txid::from_str(&format!("{:064x}", MAX_ACCEPTED_BROADCASTS + 1)).unwrap();
        assert_eq!(tracker.begin(next), BroadcastRegistration::Start);
        tracker.succeeded(next);

        // The oldest accepted entry was evicted instead of clearing the cache.
        assert_eq!(tracker.begin(oldest), BroadcastRegistration::Start);
    }

    #[test]
    fn expired_accepted_entries_are_removed_before_capacity_checks() {
        let tracker = BroadcastTracker::default();
        let txid = Txid::from_str(&format!("{:064x}", 99)).unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        tracker.succeeded(txid);

        {
            let mut inner = tracker
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            inner
                .accepted
                .insert(txid, Instant::now() - BROADCAST_MEMORY);
        }

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
    }

    #[test]
    fn duplicate_accepted_broadcasts_remain_protected() {
        let tracker = BroadcastTracker::default();
        let txid = Txid::from_str(&format!("{:064x}", 7)).unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        tracker.succeeded(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::AlreadyAccepted);
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

        let mut unsigned = unsigned_transaction();
        unsigned.input[0].witness = Witness::new();
        let raw_hex = serialize(&unsigned)
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
        let (provider_url, _broadcast_calls) = spawn_mock_provider(txid.clone()).await;

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
    async fn duplicate_broadcasts_are_idempotent_and_do_not_reach_the_provider_twice() {
        let transaction = unsigned_transaction();
        let txid = transaction.compute_txid().to_string();
        let (provider_url, calls) = spawn_mock_provider(txid.clone()).await;

        let mut config = Config::for_test();
        config.testnet_api_url = provider_url;
        let app = crate::app(&config);

        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let body = serde_json::json!({ "raw_transaction_hex": raw_hex }).to_string();

        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/testnet/broadcast")
                        .header("content-type", "application/json")
                        .body(Body::from(body.clone()))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let broadcast: BroadcastTransactionResponse = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(broadcast.txid, txid);
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn provider_rejections_are_reported_without_false_success() {
        let transaction = unsigned_transaction();
        let txid = transaction.compute_txid().to_string();
        let (provider_url, calls) = spawn_mock_provider_with(
            txid,
            Some((
                StatusCode::BAD_REQUEST,
                "bad-txns-inputs-missingorspent".to_owned(),
            )),
        )
        .await;

        let mut config = Config::for_test();
        config.testnet_api_url = provider_url;
        let app = crate::app(&config);

        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let body = serde_json::json!({ "raw_transaction_hex": raw_hex }).to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/broadcast")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error: ApiErrorResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(error.error.contains("missingorspent"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn mismatched_provider_transaction_ids_are_not_accepted() {
        let transaction = unsigned_transaction();
        let txid = transaction.compute_txid().to_string();
        let different_txid =
            "3333333333333333333333333333333333333333333333333333333333333333".to_owned();
        let (provider_url, _calls) = spawn_mock_provider(different_txid.clone()).await;

        let mut config = Config::for_test();
        config.testnet_api_url = provider_url;
        let app = crate::app(&config);

        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let body = serde_json::json!({ "raw_transaction_hex": raw_hex }).to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/broadcast")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert_ne!(txid, different_txid);
    }

    #[tokio::test]
    async fn uncertain_provider_outcomes_are_not_reported_as_rejections() {
        let transaction = unsigned_transaction();
        let txid = transaction.compute_txid().to_string();
        let (provider_url, _calls) = spawn_mock_provider_with(
            txid,
            Some((
                StatusCode::INTERNAL_SERVER_ERROR,
                "provider unavailable".to_owned(),
            )),
        )
        .await;

        let mut config = Config::for_test();
        config.testnet_api_url = provider_url;
        let app = crate::app(&config);

        let raw_hex = serialize(&transaction)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let body = serde_json::json!({ "raw_transaction_hex": raw_hex }).to_string();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/testnet/broadcast")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let error: ApiErrorResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(error.error.contains("may have been accepted"));
    }

    #[test]
    fn uncertain_broadcasts_are_not_retried_blindly() {
        let tracker = BroadcastTracker::default();
        let txid =
            Txid::from_str("4444444444444444444444444444444444444444444444444444444444444444")
                .unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        tracker.uncertain(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::Uncertain);
    }

    #[test]
    fn rejected_broadcasts_release_in_flight_registration() {
        let tracker = BroadcastTracker::default();
        let txid =
            Txid::from_str("5555555555555555555555555555555555555555555555555555555555555555")
                .unwrap();

        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
        tracker.failed(txid);
        assert_eq!(tracker.begin(txid), BroadcastRegistration::Start);
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

    #[derive(Clone)]
    struct MockProviderState {
        txid: String,
        calls: Arc<AtomicUsize>,
        response: Option<(StatusCode, String)>,
    }

    async fn spawn_mock_provider(txid: String) -> (String, Arc<AtomicUsize>) {
        spawn_mock_provider_with(txid, None).await
    }

    async fn spawn_mock_provider_with(
        txid: String,
        response: Option<(StatusCode, String)>,
    ) -> (String, Arc<AtomicUsize>) {
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

        async fn broadcast(State(state): State<MockProviderState>) -> Response {
            state.calls.fetch_add(1, Ordering::SeqCst);
            if let Some((status, body)) = state.response {
                return (status, body).into_response();
            }
            (StatusCode::OK, state.txid).into_response()
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route("/address/{address}", get(stats))
            .route("/address/{address}/utxo", get(utxos))
            .route("/address/{address}/txs", get(transactions))
            .route("/fee-estimates", get(fees))
            .route("/blocks/tip/height", get(tip))
            .route("/tx", post(broadcast))
            .with_state(MockProviderState {
                txid,
                calls: Arc::clone(&calls),
                response,
            });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (format!("http://{address}"), calls)
    }
}
