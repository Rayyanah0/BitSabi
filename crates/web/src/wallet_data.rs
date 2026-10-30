//! Aggregation of per-address testnet chain data for the wallet UI.

use std::collections::{HashMap, HashSet};

use bitsabi_shared::{TransactionStatus, WalletSnapshot, WalletTransaction};

use crate::wallet::AddressUtxo;

/// Aggregated wallet balance, outputs, and transaction history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WalletSummary {
    /// Confirmed balance in satoshis.
    pub confirmed_sat: i64,
    /// Pending balance change in satoshis.
    pub pending_sat: i64,
    /// Confirmed plus pending balance in satoshis.
    pub total_sat: i64,
    /// All confirmed and pending outputs.
    pub utxos: Vec<AddressUtxo>,
    /// De-duplicated transaction history.
    pub transactions: Vec<HistoryTransaction>,
    /// Whether any address returned only its recent history.
    pub history_truncated: bool,
    /// Chain tip at snapshot time.
    pub tip_height: u64,
    /// Public provider label.
    pub provider: String,
}

/// Direction of a wallet transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionDirection {
    Received,
    Sent,
    SelfTransfer,
}

/// Aggregated transaction history entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryTransaction {
    /// Transaction ID.
    pub txid: String,
    /// Net wallet value in satoshis.
    pub net_value: i64,
    /// Transaction fee when known.
    pub fee: Option<u64>,
    /// Confirmation state.
    pub status: TransactionStatus,
    /// Confirmations at snapshot time. Zero while pending.
    pub confirmations: u64,
}

impl HistoryTransaction {
    /// Whether this transaction added or removed wallet value.
    #[must_use]
    pub fn direction(&self) -> TransactionDirection {
        if self.net_value > 0 {
            TransactionDirection::Received
        } else if self.net_value < 0 {
            TransactionDirection::Sent
        } else {
            TransactionDirection::SelfTransfer
        }
    }
}

/// Formats a Unix timestamp as a compact UTC date and time.
#[must_use]
pub fn format_block_time(timestamp: u64) -> String {
    const SECONDS_PER_DAY: u64 = 86_400;

    let days = timestamp / SECONDS_PER_DAY;
    let seconds = timestamp % SECONDS_PER_DAY;
    let (year, month, day) = civil_date(i64::try_from(days).unwrap_or(i64::MAX));
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;

    format!("{year}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

fn civil_date(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = (days - era * 146_097) as u32;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = i64::from(year_of_era) + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    if month <= 2 {
        year += 1;
    }

    (year, month, day)
}

impl WalletSummary {
    /// Aggregate per-address data into a wallet-level view.
    #[must_use]
    pub fn from_snapshot(snapshot: &WalletSnapshot) -> Self {
        let mut confirmed_sat = 0_i64;
        let mut pending_sat = 0_i64;
        let mut utxos = Vec::new();
        let mut seen_utxos = HashSet::new();
        let mut history_truncated = false;

        for address in &snapshot.addresses {
            confirmed_sat = confirmed_sat.saturating_add(net_sat(
                address.stats.chain_stats.funded_txo_sum,
                address.stats.chain_stats.spent_txo_sum,
            ));
            pending_sat = pending_sat.saturating_add(net_sat(
                address.stats.mempool_stats.funded_txo_sum,
                address.stats.mempool_stats.spent_txo_sum,
            ));
            history_truncated |= address.history_truncated;

            for utxo in &address.utxos {
                if seen_utxos.insert((utxo.txid.as_str(), utxo.vout)) {
                    utxos.push(AddressUtxo {
                        address: address.address.clone(),
                        utxo: utxo.clone(),
                    });
                }
            }
        }

        let total_sat = confirmed_sat.saturating_add(pending_sat);
        let mut transactions = aggregate_transactions(snapshot);
        transactions.sort_by(|left, right| {
            left.status
                .confirmed
                .cmp(&right.status.confirmed)
                .then_with(|| right.status.block_height.cmp(&left.status.block_height))
                .then_with(|| left.txid.cmp(&right.txid))
        });

        for transaction in &mut transactions {
            transaction.confirmations = transaction
                .status
                .block_height
                .filter(|_| transaction.status.confirmed)
                .map_or(0, |height| {
                    snapshot.tip_height.saturating_sub(height).saturating_add(1)
                });
        }

        Self {
            confirmed_sat,
            pending_sat,
            total_sat,
            utxos,
            transactions,
            history_truncated,
            tip_height: snapshot.tip_height,
            provider: snapshot.provider.clone(),
        }
    }

    /// Confirmed outputs available for coin selection.
    #[must_use]
    pub fn spendable_utxos(&self) -> Vec<AddressUtxo> {
        self.utxos
            .iter()
            .filter(|entry| entry.utxo.status.confirmed)
            .cloned()
            .collect()
    }
}

fn net_sat(funded: u64, spent: u64) -> i64 {
    let funded = i64::try_from(funded).unwrap_or(i64::MAX);
    let spent = i64::try_from(spent).unwrap_or(i64::MAX);
    funded.saturating_sub(spent)
}

fn aggregate_transactions(snapshot: &WalletSnapshot) -> Vec<HistoryTransaction> {
    let mut aggregated: HashMap<String, HistoryTransaction> = HashMap::new();

    for address in &snapshot.addresses {
        for transaction in &address.transactions {
            aggregate_transaction(&mut aggregated, transaction);
        }
    }

    aggregated.into_values().collect()
}

fn aggregate_transaction(
    aggregated: &mut HashMap<String, HistoryTransaction>,
    transaction: &WalletTransaction,
) {
    aggregated
        .entry(transaction.txid.clone())
        .and_modify(|entry| {
            entry.net_value = entry.net_value.saturating_add(transaction.net_value);
            if transaction.fee.is_some() {
                entry.fee = transaction.fee;
            }
            if transaction.status.confirmed || !entry.status.confirmed {
                entry.status = transaction.status;
            }
        })
        .or_insert_with(|| HistoryTransaction {
            txid: transaction.txid.clone(),
            net_value: transaction.net_value,
            fee: transaction.fee,
            status: transaction.status,
            confirmations: 0,
        });
}

#[cfg(test)]
mod tests {
    use bitsabi_shared::{
        AddressSnapshot, AddressStats, AddressStatsSet, FeeEstimates, WalletUtxo,
    };

    use super::*;

    fn address_snapshot(
        address: &str,
        funded: u64,
        spent: u64,
        mempool_funded: u64,
        mempool_spent: u64,
        transactions: Vec<WalletTransaction>,
        utxos: Vec<WalletUtxo>,
    ) -> AddressSnapshot {
        AddressSnapshot {
            address: address.to_owned(),
            stats: AddressStatsSet {
                chain_stats: AddressStats {
                    funded_txo_sum: funded,
                    spent_txo_sum: spent,
                    tx_count: transactions.len() as u64,
                },
                mempool_stats: AddressStats {
                    funded_txo_sum: mempool_funded,
                    spent_txo_sum: mempool_spent,
                    tx_count: u64::from(mempool_funded > 0 || mempool_spent > 0),
                },
            },
            utxos,
            transactions,
            history_truncated: false,
        }
    }

    fn snapshot(addresses: Vec<AddressSnapshot>) -> WalletSnapshot {
        WalletSnapshot {
            provider: "test".to_owned(),
            tip_height: 100,
            fetched_at: 1,
            fees: Some(FeeEstimates {
                fastest_sat_vb: 2,
                half_hour_sat_vb: 2,
                hour_sat_vb: 2,
                economy_sat_vb: 1,
                minimum_sat_vb: 1,
            }),
            addresses,
        }
    }

    fn transaction(
        txid: &str,
        net_value: i64,
        confirmed: bool,
        height: Option<u64>,
    ) -> WalletTransaction {
        WalletTransaction {
            txid: txid.to_owned(),
            net_value,
            fee: Some(100),
            status: TransactionStatus {
                confirmed,
                block_height: height,
                block_time: height.map(|_| 1),
            },
        }
    }

    #[test]
    fn balances_and_utxos_are_aggregated() {
        let summary = WalletSummary::from_snapshot(&snapshot(vec![
            address_snapshot("a", 100_000, 20_000, 5_000, 0, vec![], vec![]),
            address_snapshot("b", 50_000, 10_000, 0, 2_000, vec![], vec![]),
        ]));

        assert_eq!(summary.confirmed_sat, 120_000);
        assert_eq!(summary.pending_sat, 3_000);
        assert_eq!(summary.total_sat, 123_000);
    }

    #[test]
    fn duplicate_transactions_are_combined_across_addresses() {
        let summary = WalletSummary::from_snapshot(&snapshot(vec![
            address_snapshot(
                "a",
                100_000,
                0,
                0,
                0,
                vec![transaction("tx", -10_000, false, None)],
                vec![],
            ),
            address_snapshot(
                "b",
                100_000,
                0,
                0,
                0,
                vec![transaction("tx", 4_000, false, None)],
                vec![],
            ),
        ]));

        assert_eq!(summary.transactions.len(), 1);
        assert_eq!(summary.transactions[0].net_value, -6_000);
        assert_eq!(
            summary.transactions[0].direction(),
            TransactionDirection::Sent
        );
    }

    #[test]
    fn history_is_sorted_with_pending_first_and_confirmation_counts() {
        let summary = WalletSummary::from_snapshot(&snapshot(vec![address_snapshot(
            "a",
            100_000,
            0,
            0,
            0,
            vec![
                transaction("old", 1_000, true, Some(90)),
                transaction("pending", -2_000, false, None),
                transaction("new", 3_000, true, Some(99)),
            ],
            vec![],
        )]));

        assert_eq!(summary.transactions[0].txid, "pending");
        assert_eq!(summary.transactions[1].txid, "new");
        assert_eq!(summary.transactions[1].confirmations, 2);
        assert_eq!(summary.transactions[2].txid, "old");
        assert_eq!(summary.transactions[2].confirmations, 11);
    }

    #[test]
    fn transaction_directions_cover_received_sent_and_self_transfers() {
        let summary = WalletSummary::from_snapshot(&snapshot(vec![address_snapshot(
            "a",
            100_000,
            0,
            0,
            0,
            vec![
                transaction("received", 5_000, true, Some(90)),
                transaction("sent", -5_000, true, Some(91)),
                transaction("self", 0, true, Some(92)),
            ],
            vec![],
        )]));

        let directions = summary
            .transactions
            .iter()
            .map(HistoryTransaction::direction)
            .collect::<Vec<_>>();

        assert!(directions.contains(&TransactionDirection::Received));
        assert!(directions.contains(&TransactionDirection::Sent));
        assert!(directions.contains(&TransactionDirection::SelfTransfer));
    }

    #[test]
    fn block_times_are_formatted_in_utc() {
        assert_eq!(format_block_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_block_time(1_700_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(format_block_time(1_704_067_199), "2023-12-31 23:59 UTC");
    }

    #[test]
    fn duplicate_utxos_are_only_included_once() {
        let utxo = WalletUtxo {
            txid: "1111111111111111111111111111111111111111111111111111111111111111".to_owned(),
            vout: 0,
            value: 100,
            status: TransactionStatus {
                confirmed: true,
                block_height: Some(1),
                block_time: Some(1),
            },
        };
        let summary = WalletSummary::from_snapshot(&snapshot(vec![
            address_snapshot("a", 100, 0, 0, 0, vec![], vec![utxo.clone()]),
            address_snapshot("b", 100, 0, 0, 0, vec![], vec![utxo]),
        ]));

        assert_eq!(summary.utxos.len(), 1);
    }

    #[test]
    fn only_confirmed_utxos_are_spendable() {
        let confirmed = WalletUtxo {
            txid: "a".to_owned(),
            vout: 0,
            value: 100,
            status: TransactionStatus {
                confirmed: true,
                block_height: Some(1),
                block_time: Some(1),
            },
        };
        let pending = WalletUtxo {
            txid: "b".to_owned(),
            vout: 0,
            value: 200,
            status: TransactionStatus {
                confirmed: false,
                block_height: None,
                block_time: None,
            },
        };
        let summary = WalletSummary::from_snapshot(&snapshot(vec![address_snapshot(
            "a",
            100,
            0,
            200,
            0,
            vec![],
            vec![confirmed, pending],
        )]));

        assert_eq!(summary.utxos.len(), 2);
        assert_eq!(summary.spendable_utxos().len(), 1);
    }
}
