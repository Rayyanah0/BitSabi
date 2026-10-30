//! Esplora-compatible testnet chain data client.
//!
//! The client is the only component that talks to a public chain provider. It
//! never receives wallet secrets, only public testnet addresses and signed
//! transactions.

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bitsabi_shared::{
    AddressSnapshot, AddressStats, AddressStatsSet, FeeEstimates, TransactionStatus,
    WalletSnapshot, WalletTransaction, WalletUtxo,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use thiserror::Error;

const PROVIDER_LABEL: &str = "Esplora testnet API";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_PROVIDER_MESSAGE_CHARS: usize = 240;
const MIN_FEE_RATE: u64 = 1;
const MAX_FEE_RATE: u64 = 1_000;

/// Esplora-compatible chain data client with an optional fallback provider.
#[derive(Clone)]
pub struct ChainClient {
    http: reqwest::Client,
    base_urls: Arc<[String]>,
}

impl ChainClient {
    /// Build a client from validated provider base URLs.
    #[must_use]
    pub fn new(primary: &str, fallback: Option<&str>) -> Self {
        let mut base_urls = vec![primary.trim_end_matches('/').to_owned()];
        if let Some(fallback) = fallback {
            let fallback = fallback.trim_end_matches('/');
            if !fallback.is_empty() && fallback != primary.trim_end_matches('/') {
                base_urls.push(fallback.to_owned());
            }
        }

        Self {
            http: reqwest::Client::new(),
            base_urls: Arc::from(base_urls),
        }
    }

    /// Fetch a complete snapshot for the supplied public testnet addresses.
    pub async fn wallet_snapshot(
        &self,
        addresses: &[String],
    ) -> Result<WalletSnapshot, ChainError> {
        let (tip_height, fees) = tokio::join!(self.tip_height(), self.fee_estimates());
        let tip_height = tip_height?;
        let fees = fees?;

        let stats = self.address_stats_many(addresses).await?;
        let mut address_results: Vec<Option<AddressSnapshot>> =
            (0..addresses.len()).map(|_| None).collect();

        let mut tasks = tokio::task::JoinSet::new();
        for (index, (address, stats)) in addresses.iter().zip(stats).enumerate() {
            let total_tx_count = stats.chain_stats.tx_count + stats.mempool_stats.tx_count;
            if total_tx_count == 0 {
                address_results[index] = Some(AddressSnapshot {
                    address: address.clone(),
                    stats,
                    utxos: Vec::new(),
                    transactions: Vec::new(),
                    history_truncated: false,
                });
                continue;
            }

            let client = self.clone();
            let address = address.clone();
            tasks.spawn(async move {
                let (utxos, transactions) = tokio::join!(
                    client.address_utxos(&address),
                    client.address_transactions(&address)
                );
                (index, address, stats, utxos, transactions, total_tx_count)
            });
        }

        while let Some(result) = tasks.join_next().await {
            let (index, address, stats, utxos, transactions, total_tx_count) =
                result.map_err(|_| ChainError::Unavailable)?;
            let utxos = utxos?;
            let transactions = transactions?;
            let history_truncated = total_tx_count > transactions.len() as u64;
            address_results[index] = Some(AddressSnapshot {
                address,
                stats,
                utxos,
                transactions,
                history_truncated,
            });
        }

        let addresses = address_results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(ChainError::InvalidResponse)?;

        Ok(WalletSnapshot {
            provider: PROVIDER_LABEL.to_owned(),
            tip_height,
            fetched_at: unix_timestamp(),
            fees,
            addresses,
        })
    }

    /// Broadcast a consensus-serialized transaction and return its transaction ID.
    pub async fn broadcast(&self, raw_transaction_hex: &str) -> Result<String, ChainError> {
        let txid = self.post_text("/tx", raw_transaction_hex).await?;
        let txid = txid.trim();
        if txid.is_empty() || txid.len() > 128 {
            return Err(ChainError::InvalidResponse);
        }
        Ok(txid.to_owned())
    }

    async fn address_stats_many(
        &self,
        addresses: &[String],
    ) -> Result<Vec<AddressStatsSet>, ChainError> {
        let mut results: Vec<Option<AddressStatsSet>> =
            (0..addresses.len()).map(|_| None).collect();
        let mut tasks = tokio::task::JoinSet::new();

        for (index, address) in addresses.iter().enumerate() {
            let client = self.clone();
            let address = address.clone();
            tasks.spawn(async move {
                let stats = client.address_stats(&address).await;
                (index, stats)
            });
        }

        while let Some(result) = tasks.join_next().await {
            let (index, stats) = result.map_err(|_| ChainError::Unavailable)?;
            results[index] = Some(stats?);
        }

        results
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(ChainError::InvalidResponse)
    }

    async fn address_stats(&self, address: &str) -> Result<AddressStatsSet, ChainError> {
        let stats: EsploraAddressStats = self
            .get_json(&format!("/address/{address}"))
            .await?;

        Ok(AddressStatsSet {
            chain_stats: AddressStats {
                funded_txo_sum: stats.chain_stats.funded_txo_sum,
                spent_txo_sum: stats.chain_stats.spent_txo_sum,
                tx_count: stats.chain_stats.tx_count,
            },
            mempool_stats: AddressStats {
                funded_txo_sum: stats.mempool_stats.funded_txo_sum,
                spent_txo_sum: stats.mempool_stats.spent_txo_sum,
                tx_count: stats.mempool_stats.tx_count,
            },
        })
    }

    async fn address_utxos(&self, address: &str) -> Result<Vec<WalletUtxo>, ChainError> {
        let utxos: Vec<EsploraUtxo> = self
            .get_json(&format!("/address/{address}/utxo"))
            .await?;

        Ok(utxos
            .into_iter()
            .map(|utxo| WalletUtxo {
                txid: utxo.txid,
                vout: utxo.vout,
                value: utxo.value,
                status: TransactionStatus {
                    confirmed: utxo.status.confirmed,
                    block_height: utxo.status.block_height,
                    block_time: utxo.status.block_time,
                },
            })
            .collect())
    }

    async fn address_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, ChainError> {
        let transactions: Vec<EsploraTransaction> = self
            .get_json(&format!("/address/{address}/txs"))
            .await?;

        Ok(transactions
            .iter()
            .filter_map(|transaction| transaction_for_address(transaction, address))
            .collect())
    }

    async fn tip_height(&self) -> Result<u64, ChainError> {
        let height = self.get_text("/blocks/tip/height").await?;
        height.trim().parse::<u64>().map_err(|_| ChainError::InvalidResponse)
    }

    async fn fee_estimates(&self) -> Result<FeeEstimates, ChainError> {
        let estimates: BTreeMap<String, f64> = self.get_json("/fee-estimates").await?;
        Ok(normalize_fee_estimates(&estimates))
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ChainError> {
        let mut last_error = ChainError::Unavailable;

        for base in self.base_urls.iter() {
            let response = self
                .http
                .get(format!("{base}{path}"))
                .timeout(REQUEST_TIMEOUT)
                .header(reqwest::header::USER_AGENT, user_agent())
                .send()
                .await;

            let response = match response {
                Ok(response) => response,
                Err(_) => continue,
            };

            if !response.status().is_success() {
                last_error = ChainError::ProviderStatus(response.status().as_u16());
                continue;
            }

            let bytes = match response.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => {
                    last_error = ChainError::InvalidResponse;
                    continue;
                }
            };

            match serde_json::from_slice(&bytes) {
                Ok(value) => return Ok(value),
                Err(_) => last_error = ChainError::InvalidResponse,
            }
        }

        Err(last_error)
    }

    async fn get_text(&self, path: &str) -> Result<String, ChainError> {
        let mut last_error = ChainError::Unavailable;

        for base in self.base_urls.iter() {
            let response = self
                .http
                .get(format!("{base}{path}"))
                .timeout(REQUEST_TIMEOUT)
                .header(reqwest::header::USER_AGENT, user_agent())
                .send()
                .await;

            let response = match response {
                Ok(response) => response,
                Err(_) => continue,
            };

            if !response.status().is_success() {
                last_error = ChainError::ProviderStatus(response.status().as_u16());
                continue;
            }

            match response.text().await {
                Ok(text) => return Ok(text),
                Err(_) => last_error = ChainError::InvalidResponse,
            }
        }

        Err(last_error)
    }

    async fn post_text(&self, path: &str, body: &str) -> Result<String, ChainError> {
        let mut last_error = ChainError::Unavailable;

        for base in self.base_urls.iter() {
            let response = self
                .http
                .post(format!("{base}{path}"))
                .timeout(REQUEST_TIMEOUT)
                .header(reqwest::header::USER_AGENT, user_agent())
                .header(reqwest::header::CONTENT_TYPE, "text/plain")
                .body(body.to_owned())
                .send()
                .await;

            let response = match response {
                Ok(response) => response,
                Err(_) => continue,
            };

            if response.status().is_success() {
                return response.text().await.map_err(|_| ChainError::InvalidResponse);
            }

            let status = response.status();
            let message = provider_message(response).await;
            if status.is_client_error() && status.as_u16() != 429 {
                return Err(ChainError::BroadcastRejected(message));
            }
            last_error = ChainError::ProviderStatus(status.as_u16());
        }

        Err(last_error)
    }
}

/// Chain provider failure safe for API error mapping.
#[derive(Debug, Error)]
pub enum ChainError {
    /// Provider unreachable, rate limited, or currently failing.
    #[error("testnet chain data is temporarily unavailable")]
    Unavailable,
    /// Provider returned data this API could not use.
    #[error("the testnet chain provider returned an invalid response")]
    InvalidResponse,
    /// Provider returned an unexpected HTTP status.
    #[error("the testnet chain provider rejected the request")]
    ProviderStatus(u16),
    /// Provider rejected a signed transaction.
    #[error("{0}")]
    BroadcastRejected(String),
}

#[derive(Debug, Deserialize)]
struct EsploraAddressStats {
    chain_stats: EsploraStats,
    mempool_stats: EsploraStats,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct EsploraStats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct EsploraUtxo {
    txid: String,
    vout: u32,
    value: u64,
    status: EsploraStatus,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
struct EsploraStatus {
    confirmed: bool,
    block_height: Option<u64>,
    block_time: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct EsploraTransaction {
    txid: String,
    fee: u64,
    status: EsploraStatus,
    vin: Vec<EsploraVin>,
    vout: Vec<EsploraVout>,
}

#[derive(Debug, Deserialize)]
struct EsploraVin {
    prevout: Option<EsploraVout>,
}

#[derive(Debug, Deserialize)]
struct EsploraVout {
    scriptpubkey_address: Option<String>,
    value: u64,
}

fn transaction_for_address(
    transaction: &EsploraTransaction,
    address: &str,
) -> Option<WalletTransaction> {
    let mut net_value = 0_i64;

    for output in &transaction.vout {
        if output.scriptpubkey_address.as_deref() == Some(address) {
            net_value = net_value.checked_add(i64::try_from(output.value).ok()?)?;
        }
    }

    for input in &transaction.vin {
        if let Some(previous_output) = &input.prevout {
            if previous_output.scriptpubkey_address.as_deref() == Some(address) {
                net_value = net_value.checked_sub(i64::try_from(previous_output.value).ok()?)?;
            }
        }
    }

    Some(WalletTransaction {
        txid: transaction.txid.clone(),
        net_value,
        fee: Some(transaction.fee),
        status: TransactionStatus {
            confirmed: transaction.status.confirmed,
            block_height: transaction.status.block_height,
            block_time: transaction.status.block_time,
        },
    })
}

fn normalize_fee_estimates(estimates: &BTreeMap<String, f64>) -> FeeEstimates {
    FeeEstimates {
        fastest_sat_vb: fee_for_target(estimates, 1),
        half_hour_sat_vb: fee_for_target(estimates, 3),
        hour_sat_vb: fee_for_target(estimates, 6),
        economy_sat_vb: fee_for_target(estimates, 144),
        minimum_sat_vb: fee_for_target(estimates, 1_008),
    }
}

fn fee_for_target(estimates: &BTreeMap<String, f64>, target: u64) -> u64 {
    let exact = target.to_string();
    let selected = estimates
        .get(&exact)
        .copied()
        .or_else(|| nearest_estimate(estimates, target))
        .unwrap_or(1.0);

    if !selected.is_finite() || selected <= 0.0 {
        return MIN_FEE_RATE;
    }

    (selected.ceil() as u64).clamp(MIN_FEE_RATE, MAX_FEE_RATE)
}

fn nearest_estimate(estimates: &BTreeMap<String, f64>, target: u64) -> Option<f64> {
    estimates
        .iter()
        .filter_map(|(key, value)| {
            key.parse::<u64>()
                .ok()
                .map(|entry_target| (entry_target.abs_diff(target), *value))
        })
        .min_by_key(|(difference, _)| *difference)
        .map(|(_, value)| value)
}

async fn provider_message(response: reqwest::Response) -> String {
    let message = response
        .text()
        .await
        .unwrap_or_else(|_| "Transaction rejected by the testnet provider.".to_owned());
    let message: String = message
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_PROVIDER_MESSAGE_CHARS)
        .collect();
    let message = message.trim();
    if message.is_empty() {
        "Transaction rejected by the testnet provider.".to_owned()
    } else {
        message.to_owned()
    }
}

fn user_agent() -> String {
    format!("BitSabi/{}", env!("CARGO_PKG_VERSION"))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimates(entries: &[(&str, f64)]) -> BTreeMap<String, f64> {
        entries
            .iter()
            .map(|(target, rate)| ((*target).to_owned(), *rate))
            .collect()
    }

    #[test]
    fn fee_estimates_select_exact_targets() {
        let fees = normalize_fee_estimates(&estimates(&[
            ("1", 2.1),
            ("3", 3.0),
            ("6", 4.0),
            ("144", 1.2),
            ("1008", 1.0),
        ]));

        assert_eq!(fees.fastest_sat_vb, 3);
        assert_eq!(fees.half_hour_sat_vb, 3);
        assert_eq!(fees.hour_sat_vb, 4);
        assert_eq!(fees.economy_sat_vb, 2);
        assert_eq!(fees.minimum_sat_vb, 1);
    }

    #[test]
    fn fee_estimates_fall_back_to_the_nearest_target() {
        let estimates = estimates(&[("2", 1.01), ("10", 2.4)]);

        assert_eq!(fee_for_target(&estimates, 1), 2);
        assert_eq!(fee_for_target(&estimates, 144), 3);
    }

    #[test]
    fn fee_estimates_reject_invalid_values() {
        let estimates = estimates(&[("1", f64::NAN), ("2", 0.0), ("3", 5_000.0)]);

        assert_eq!(fee_for_target(&estimates, 1), MIN_FEE_RATE);
        assert_eq!(fee_for_target(&estimates, 2), MIN_FEE_RATE);
        assert_eq!(fee_for_target(&estimates, 3), MAX_FEE_RATE);
    }

    #[test]
    fn transaction_net_value_handles_wallet_inputs_and_outputs() {
        let address = "tb1qexample";
        let transaction = EsploraTransaction {
            txid: "abc".to_owned(),
            fee: 150,
            status: EsploraStatus::default(),
            vin: vec![
                EsploraVin {
                    prevout: Some(EsploraVout {
                        scriptpubkey_address: Some(address.to_owned()),
                        value: 10_000,
                    }),
                },
                EsploraVin {
                    prevout: Some(EsploraVout {
                        scriptpubkey_address: Some("tb1qexternal".to_owned()),
                        value: 5_000,
                    }),
                },
            ],
            vout: vec![
                EsploraVout {
                    scriptpubkey_address: Some(address.to_owned()),
                    value: 4_000,
                },
                EsploraVout {
                    scriptpubkey_address: Some("tb1qrecipient".to_owned()),
                    value: 10_850,
                },
            ],
        };

        let transaction = transaction_for_address(&transaction, address).unwrap();

        assert_eq!(transaction.txid, "abc");
        assert_eq!(transaction.net_value, -6_000);
        assert_eq!(transaction.fee, Some(150));
        assert!(!transaction.status.confirmed);
    }

    #[test]
    fn provider_messages_are_trimmed_and_bounded() {
        let message: String = "bad transaction\n".repeat(40);
        let sanitized: String = message
            .chars()
            .filter(|character| !character.is_control())
            .take(MAX_PROVIDER_MESSAGE_CHARS)
            .collect();
        assert!(!sanitized.contains('\n'));
        assert!(sanitized.len() <= MAX_PROVIDER_MESSAGE_CHARS);
    }
}
