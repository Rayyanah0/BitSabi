use serde::{Deserialize, Serialize};

/// Maximum number of wallet addresses accepted in one snapshot request.
pub const MAX_SNAPSHOT_ADDRESSES: usize = 50;
/// Maximum serialized transaction size accepted for broadcast.
pub const MAX_RAW_TRANSACTION_BYTES: usize = 100_000;

/// Request body for a testnet wallet snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletSnapshotRequest {
    /// Derived testnet addresses owned by the wallet.
    pub addresses: Vec<String>,
}

/// Confirmed or mempool totals reported for one address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressStats {
    /// Total value received by the address.
    pub funded_txo_sum: u64,
    /// Total value spent by the address.
    pub spent_txo_sum: u64,
    /// Number of transactions touching the address.
    pub tx_count: u64,
}

/// Confirmed and mempool totals for one address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressStatsSet {
    /// Totals from confirmed blocks.
    pub chain_stats: AddressStats,
    /// Totals from the mempool.
    pub mempool_stats: AddressStats,
}

/// Confirmation state for a UTXO or transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionStatus {
    /// Whether the UTXO or transaction is confirmed.
    pub confirmed: bool,
    /// Block height when confirmed.
    pub block_height: Option<u64>,
    /// Block timestamp in Unix seconds when confirmed.
    pub block_time: Option<u64>,
}

/// An unspent transaction output owned by a wallet address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletUtxo {
    /// Transaction ID containing the output.
    pub txid: String,
    /// Output index.
    pub vout: u32,
    /// Output value in satoshis.
    pub value: u64,
    /// Confirmation state.
    pub status: TransactionStatus,
}

/// A transaction net effect for one wallet address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletTransaction {
    /// Transaction ID.
    pub txid: String,
    /// Net value for this address in satoshis. Positive is received.
    pub net_value: i64,
    /// Transaction fee in satoshis when reported by the provider.
    pub fee: Option<u64>,
    /// Confirmation state.
    pub status: TransactionStatus,
}

/// Chain data for one derived wallet address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressSnapshot {
    /// Testnet address.
    pub address: String,
    /// Confirmed and mempool balance totals.
    pub stats: AddressStatsSet,
    /// Spendable outputs currently reported for the address.
    pub utxos: Vec<WalletUtxo>,
    /// Recent transactions touching the address.
    pub transactions: Vec<WalletTransaction>,
    /// Whether older transactions exist beyond the returned recent set.
    pub history_truncated: bool,
}

/// Recommended fee rates from the chain provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeEstimates {
    /// Target for the next block, in satoshis per virtual byte.
    pub fastest_sat_vb: u64,
    /// Target for roughly 30 minutes, in satoshis per virtual byte.
    pub half_hour_sat_vb: u64,
    /// Target for roughly one hour, in satoshis per virtual byte.
    pub hour_sat_vb: u64,
    /// Economy target, in satoshis per virtual byte.
    pub economy_sat_vb: u64,
    /// Provider minimum, in satoshis per virtual byte.
    pub minimum_sat_vb: u64,
}

/// A complete testnet wallet snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletSnapshot {
    /// Public provider label.
    pub provider: String,
    /// Current chain tip height.
    pub tip_height: u64,
    /// Snapshot creation time in Unix seconds.
    pub fetched_at: u64,
    /// Fee recommendations when the provider returned usable data.
    pub fees: Option<FeeEstimates>,
    /// Per-address chain data.
    pub addresses: Vec<AddressSnapshot>,
}

/// Request body for broadcasting a signed testnet transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastTransactionRequest {
    /// Consensus-serialized transaction encoded as lowercase or uppercase hex.
    pub raw_transaction_hex: String,
}

/// Successful broadcast response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastTransactionResponse {
    /// Transaction ID reported by the chain provider.
    pub txid: String,
}

/// Public API error body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// Safe error message for the user.
    pub error: String,
}
