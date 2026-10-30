//! Testnet wallet derivation, transaction planning, and signing.
//!
//! Wallet secrets stay in browser memory for the current tab only. The seed is
//! zeroized when the wallet session is dropped; browser memory cannot be
//! guaranteed to be fully erased.

use std::{fmt, str::FromStr};

use bip39::{Error as Bip39Error, Mnemonic};
use bitcoin::{
    absolute::LockTime,
    bip32::{DerivationPath, Xpriv},
    consensus::encode::serialize_hex,
    ecdsa::Signature as EcdsaSignature,
    hashes::Hash,
    secp256k1::{Message, Secp256k1},
    sighash::{EcdsaSighashType, SighashCache},
    transaction::Version,
    Address, Amount, CompressedPublicKey, Network, NetworkKind, OutPoint, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Txid, Witness,
};
use bitsabi_shared::WalletUtxo;
use zeroize::Zeroizing;

#[cfg(test)]
const CREATED_WORD_COUNT: usize = 12;
const CREATED_ENTROPY_BYTES: usize = 16;
const RECEIVE_GAP_LIMIT: u32 = 20;
const CHANGE_GAP_LIMIT: u32 = 20;
const MAX_RECEIVE_ADDRESSES: u32 = 100;
const MAX_FEE_RATE_SAT_VB: u64 = 1_000;
const P2WPKH_SCRIPT_LEN: u64 = 22;
const P2WPKH_INPUT_BASE_BYTES: u64 = 41;
const P2WPKH_WITNESS_BYTES: u64 = 108;
const WITNESS_MARKER_BYTES: u64 = 2;

/// Errors that can occur while validating, deriving, planning, or signing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletError {
    /// The UI accepts only 12- or 24-word recovery phrases.
    UnsupportedWordCount { found: usize },
    /// BIP39 reported an invalid word count.
    InvalidWordCount { found: usize },
    /// A word was not found in the enabled BIP39 word list.
    UnknownWord { index: usize },
    /// The phrase has a valid word list but an invalid checksum.
    InvalidChecksum,
    /// Another BIP39 validation failure.
    InvalidRecoveryPhrase,
    /// The browser did not provide a secure random-number source.
    SecureRandomUnavailable,
    /// A new mnemonic could not be built from the generated entropy.
    MnemonicGenerationFailed,
    /// A testnet address or key could not be derived.
    KeyDerivationFailed,
    /// The amount is not a valid BTC amount.
    InvalidAmount,
    /// The amount is below the recipient output dust limit.
    AmountBelowDust { minimum_sat: u64 },
    /// The fee rate is outside the supported range.
    InvalidFeeRate,
    /// The recipient is not a valid Bitcoin testnet address.
    InvalidRecipient,
    /// No confirmed outputs are available to spend.
    NoConfirmedFunds,
    /// Confirmed outputs do not cover the amount and fee.
    InsufficientFunds {
        available_sat: u64,
        required_sat: u64,
    },
    /// The wallet cannot create more receiving addresses in this session.
    ReceiveAddressLimit,
    /// The transaction could not be constructed or signed.
    TransactionFailed,
}

impl fmt::Display for WalletError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedWordCount { found } => write!(
                formatter,
                "Recovery phrases must contain 12 or 24 words. This one has {found}."
            ),
            Self::InvalidWordCount { found } => write!(
                formatter,
                "The recovery phrase has an unsupported number of words ({found})."
            ),
            Self::UnknownWord { index } => write!(
                formatter,
                "Word {} is not in the BIP39 English word list.",
                index + 1
            ),
            Self::InvalidChecksum => formatter.write_str(
                "The recovery phrase checksum is invalid. Check the spelling and order of every word.",
            ),
            Self::InvalidRecoveryPhrase => {
                formatter.write_str("This recovery phrase is not valid BIP39.")
            }
            Self::SecureRandomUnavailable => formatter.write_str(
                "Secure random-number generation is unavailable in this browser. No wallet was created.",
            ),
            Self::MnemonicGenerationFailed => formatter.write_str(
                "A new recovery phrase could not be generated. Please try again.",
            ),
            Self::KeyDerivationFailed => {
                formatter.write_str("The testnet wallet could not be derived from this phrase.")
            }
            Self::InvalidAmount => formatter.write_str(
                "Enter a BTC amount with up to 8 decimal places.",
            ),
            Self::AmountBelowDust { minimum_sat } => write!(
                formatter,
                "The amount is too small for a standard output. Send at least {}.",
                format_sats(*minimum_sat)
            ),
            Self::InvalidFeeRate => {
                formatter.write_str("Choose a fee rate between 1 and 1000 sat/vB.")
            }
            Self::InvalidRecipient => formatter.write_str(
                "Enter a valid Bitcoin testnet address. Mainnet addresses are not supported.",
            ),
            Self::NoConfirmedFunds => formatter.write_str(
                "No confirmed testnet coins are available to spend yet.",
            ),
            Self::InsufficientFunds {
                available_sat,
                required_sat,
            } => write!(
                formatter,
                "Confirmed balance is {}, but at least {} plus the network fee is required.",
                format_sats(*available_sat),
                format_sats(*required_sat)
            ),
            Self::ReceiveAddressLimit => formatter.write_str(
                "This session cannot create more receiving addresses. Start over to restore the wallet.",
            ),
            Self::TransactionFailed => {
                formatter.write_str("The transaction could not be built or signed.")
            }
        }
    }
}

impl std::error::Error for WalletError {}

/// A newly generated wallet held only in browser memory.
pub struct CreatedWallet {
    phrase: Zeroizing<String>,
    address: String,
}

impl CreatedWallet {
    /// Recovery phrase for the user to back up.
    #[must_use]
    pub fn phrase(&self) -> &str {
        self.phrase.as_str()
    }

    /// Derived testnet receiving address.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddressBranch {
    Receive,
    Change,
}

impl AddressBranch {
    fn index(self) -> u32 {
        match self {
            Self::Receive => 0,
            Self::Change => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DerivedAddress {
    branch: AddressBranch,
    index: u32,
    address: String,
    script_pubkey: ScriptBuf,
}

impl DerivedAddress {
    /// Address index within its receive or change branch.
    #[must_use]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Encoded testnet address.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// A wallet session with the signing seed retained in memory for this tab.
pub struct WalletSession {
    seed: Zeroizing<Vec<u8>>,
    receive_addresses: Vec<DerivedAddress>,
    change_addresses: Vec<DerivedAddress>,
    visible_receive_count: usize,
    next_change_index: usize,
}

impl WalletSession {
    /// Derive a testnet wallet session from a validated recovery phrase.
    pub fn from_phrase(phrase: &str) -> Result<Self, WalletError> {
        let normalized = normalize_phrase(phrase);
        let mnemonic = parse_import_mnemonic(&normalized)?;
        let seed = Zeroizing::new(mnemonic.to_seed("").to_vec());
        Self::from_seed(seed)
    }

    fn from_seed(seed: Zeroizing<Vec<u8>>) -> Result<Self, WalletError> {
        let secp = Secp256k1::new();
        let master = Xpriv::new_master(NetworkKind::Test, &seed[..])
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        let mut receive_addresses = Vec::with_capacity(RECEIVE_GAP_LIMIT as usize);
        let mut change_addresses = Vec::with_capacity(CHANGE_GAP_LIMIT as usize);

        for index in 0..RECEIVE_GAP_LIMIT {
            receive_addresses.push(derive_address(
                &master,
                &secp,
                AddressBranch::Receive,
                index,
            )?);
        }
        for index in 0..CHANGE_GAP_LIMIT {
            change_addresses.push(derive_address(
                &master,
                &secp,
                AddressBranch::Change,
                index,
            )?);
        }

        Ok(Self {
            seed,
            receive_addresses,
            change_addresses,
            visible_receive_count: 1,
            next_change_index: 0,
        })
    }

    /// Number of receiving addresses generated for the user in this session.
    #[must_use]
    pub fn receive_address_count(&self) -> usize {
        self.visible_receive_count
    }

    /// Receiving addresses generated for the user so far.
    #[must_use]
    pub fn visible_receive_addresses(&self) -> &[DerivedAddress] {
        &self.receive_addresses[..self.visible_receive_count]
    }

    /// The active receiving address.
    #[must_use]
    pub fn current_receive_address(&self) -> &DerivedAddress {
        &self.receive_addresses[self.visible_receive_count - 1]
    }

    /// Generate the next receiving address while retaining earlier addresses.
    pub fn generate_receive_address(&mut self) -> Result<&DerivedAddress, WalletError> {
        if self.visible_receive_count as u32 >= MAX_RECEIVE_ADDRESSES {
            return Err(WalletError::ReceiveAddressLimit);
        }

        self.visible_receive_count += 1;
        let required = (self.visible_receive_count as u32 + RECEIVE_GAP_LIMIT)
            .min(MAX_RECEIVE_ADDRESSES);
        while self.receive_addresses.len() < required as usize {
            let index = self.receive_addresses.len() as u32;
            self.extend_receive_address(index)?;
        }

        Ok(self.current_receive_address())
    }

    fn extend_receive_address(&mut self, index: u32) -> Result<(), WalletError> {
        let secp = Secp256k1::new();
        let master = Xpriv::new_master(NetworkKind::Test, &self.seed[..])
            .map_err(|_| WalletError::KeyDerivationFailed)?;
        self.receive_addresses.push(derive_address(
            &master,
            &secp,
            AddressBranch::Receive,
            index,
        )?);
        Ok(())
    }

    /// All public addresses that should be scanned for balance and history.
    #[must_use]
    pub fn scan_addresses(&self) -> Vec<String> {
        self.receive_addresses
            .iter()
            .chain(self.change_addresses.iter())
            .map(|address| address.address.clone())
            .collect()
    }

    fn next_change_address(&self) -> &DerivedAddress {
        &self.change_addresses[self.next_change_index]
    }

    /// Mark the current change address as used after a successful broadcast.
    pub fn record_change_used(&mut self) -> Result<(), WalletError> {
        self.next_change_index += 1;
        if self.next_change_index >= self.change_addresses.len() {
            let secp = Secp256k1::new();
            let master = Xpriv::new_master(NetworkKind::Test, &self.seed[..])
                .map_err(|_| WalletError::KeyDerivationFailed)?;
            let index = self.change_addresses.len() as u32;
            self.change_addresses.push(derive_address(
                &master,
                &secp,
                AddressBranch::Change,
                index,
            )?);
        }
        Ok(())
    }

    fn owned_address(&self, address: &str) -> Option<&DerivedAddress> {
        self.receive_addresses
            .iter()
            .chain(self.change_addresses.iter())
            .find(|candidate| candidate.address == address)
    }
}

/// A chain UTXO paired with the wallet address that owns it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressUtxo {
    /// Wallet address.
    pub address: String,
    /// Chain UTXO details.
    pub utxo: WalletUtxo,
}

/// A reviewed send plan. No signature exists until the user confirms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendPlan {
    /// Recipient testnet address.
    pub recipient: String,
    /// Amount sent to the recipient in satoshis.
    pub amount_sat: u64,
    /// Network fee in satoshis.
    pub fee_sat: u64,
    /// Selected fee rate in satoshis per virtual byte.
    pub fee_rate_sat_vb: u64,
    /// Change returned to the wallet in satoshis.
    pub change_sat: u64,
    /// Change address when a change output is included.
    pub change_address: Option<String>,
    /// Estimated virtual size.
    pub vsize: u64,
    /// Total value of selected inputs.
    pub total_input_sat: u64,
    /// Number of selected inputs.
    pub input_count: usize,
    inputs: Vec<SendInput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SendInput {
    utxo: WalletUtxo,
    branch: AddressBranch,
    index: u32,
    script_pubkey: ScriptBuf,
}

/// A locally signed transaction ready for broadcast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedTransaction {
    /// Consensus-serialized transaction hex.
    pub raw_hex: String,
    /// Transaction ID.
    pub txid: String,
    /// Network fee in satoshis.
    pub fee_sat: u64,
    /// Actual virtual size.
    pub vsize: u64,
    /// Number of inputs.
    pub input_count: usize,
    /// Change value in satoshis.
    pub change_sat: u64,
}

/// Generates a new 12-word wallet using the browser's secure randomness.
pub fn create_testnet_wallet() -> Result<CreatedWallet, WalletError> {
    generate_testnet_wallet_with(|entropy| {
        getrandom::getrandom(entropy).map_err(|_| WalletError::SecureRandomUnavailable)
    })
}

fn generate_testnet_wallet_with(
    mut fill_entropy: impl FnMut(&mut [u8]) -> Result<(), WalletError>,
) -> Result<CreatedWallet, WalletError> {
    let mut entropy = Zeroizing::new([0_u8; CREATED_ENTROPY_BYTES]);
    fill_entropy(entropy.as_mut())?;

    let mnemonic =
        Mnemonic::from_entropy(&entropy[..]).map_err(|_| WalletError::MnemonicGenerationFailed)?;
    let phrase = Zeroizing::new(mnemonic.to_string());
    let address = derive_address_from_mnemonic(&mnemonic)?;

    Ok(CreatedWallet { phrase, address })
}

/// Validates a recovery phrase for the 12- or 24-word wallet import flow.
pub fn validate_recovery_phrase(phrase: &str) -> Result<(), WalletError> {
    let normalized = normalize_phrase(phrase);
    parse_import_mnemonic(&normalized).map(|_| ())
}

/// Imports a 12- or 24-word phrase and returns its first testnet address.
pub fn import_testnet_address(phrase: &str) -> Result<String, WalletError> {
    derive_testnet_address(phrase)
}

/// Derives the first BIP84 native SegWit Bitcoin testnet address.
pub fn derive_testnet_address(phrase: &str) -> Result<String, WalletError> {
    let normalized = normalize_phrase(phrase);
    let mnemonic = parse_import_mnemonic(&normalized)?;
    derive_address_from_mnemonic(&mnemonic)
}

/// Parses a BTC amount into satoshis without floating-point arithmetic.
pub fn parse_btc_amount(input: &str) -> Result<u64, WalletError> {
    let input = input.trim();
    if input.is_empty() || input.starts_with('-') || input.starts_with('+') {
        return Err(WalletError::InvalidAmount);
    }

    let mut parts = input.split('.');
    let whole = parts.next().ok_or(WalletError::InvalidAmount)?;
    let fraction = parts.next();
    if parts.next().is_some() || whole.is_empty() {
        return Err(WalletError::InvalidAmount);
    }
    if !whole.chars().all(|character| character.is_ascii_digit()) {
        return Err(WalletError::InvalidAmount);
    }

    let whole = whole.parse::<u64>().map_err(|_| WalletError::InvalidAmount)?;
    let mut sats = whole
        .checked_mul(100_000_000)
        .ok_or(WalletError::InvalidAmount)?;

    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || fraction.len() > 8
            || !fraction
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            return Err(WalletError::InvalidAmount);
        }
        let digits = fraction.len();
        let fraction = fraction
            .parse::<u64>()
            .map_err(|_| WalletError::InvalidAmount)?;
        let scale = 10_u64.pow((8 - digits) as u32);
        sats = sats
            .checked_add(
                fraction
                    .checked_mul(scale)
                    .ok_or(WalletError::InvalidAmount)?,
            )
            .ok_or(WalletError::InvalidAmount)?;
    }

    if sats == 0 || sats > Amount::MAX_MONEY.to_sat() {
        return Err(WalletError::InvalidAmount);
    }

    Ok(sats)
}

/// Formats satoshis as a BTC string with trailing zeroes removed.
#[must_use]
pub fn format_sats(sats: u64) -> String {
    let whole = sats / 100_000_000;
    let fraction = sats % 100_000_000;
    if fraction == 0 {
        return format!("{whole} BTC");
    }
    let fraction = format!("{fraction:08}");
    format!("{whole}.{} BTC", fraction.trim_end_matches('0'))
}

/// Formats a signed satoshi amount as a BTC string.
#[must_use]
pub fn format_signed_sats(sats: i64) -> String {
    if sats < 0 {
        format!("-{}", format_sats(sats.unsigned_abs()))
    } else {
        format_sats(sats as u64)
    }
}

/// Validates a recipient and returns its checked testnet address.
pub fn validate_recipient(recipient: &str) -> Result<Address, WalletError> {
    let address = Address::from_str(recipient.trim()).map_err(|_| WalletError::InvalidRecipient)?;
    address
        .require_network(Network::Testnet)
        .map_err(|_| WalletError::InvalidRecipient)
}

/// Builds a deterministic send plan from confirmed wallet UTXOs.
pub fn plan_send(
    session: &WalletSession,
    utxos: &[AddressUtxo],
    recipient: &str,
    amount_sat: u64,
    fee_rate_sat_vb: u64,
) -> Result<SendPlan, WalletError> {
    if !(1..=MAX_FEE_RATE_SAT_VB).contains(&fee_rate_sat_vb) {
        return Err(WalletError::InvalidFeeRate);
    }

    let recipient = validate_recipient(recipient)?;
    let recipient_script = recipient.script_pubkey();
    let recipient_dust = recipient_script.minimal_non_dust().to_sat();
    if amount_sat < recipient_dust {
        return Err(WalletError::AmountBelowDust {
            minimum_sat: recipient_dust,
        });
    }
    if amount_sat > Amount::MAX_MONEY.to_sat() {
        return Err(WalletError::InvalidAmount);
    }

    let mut candidates = utxos
        .iter()
        .filter(|entry| entry.utxo.status.confirmed)
        .filter_map(|entry| {
            session.owned_address(&entry.address).map(|owned| SendInput {
                utxo: entry.utxo.clone(),
                branch: owned.branch,
                index: owned.index,
                script_pubkey: owned.script_pubkey.clone(),
            })
        })
        .collect::<Vec<_>>();

    let available_sat = candidates
        .iter()
        .try_fold(0_u64, |total, input| total.checked_add(input.utxo.value))
        .ok_or(WalletError::TransactionFailed)?;

    if candidates.is_empty() {
        return Err(WalletError::NoConfirmedFunds);
    }

    candidates.sort_by(|left, right| {
        right
            .utxo
            .value
            .cmp(&left.utxo.value)
            .then_with(|| right.utxo.txid.cmp(&left.utxo.txid))
            .then_with(|| right.utxo.vout.cmp(&left.utxo.vout))
    });

    let change_address = session.next_change_address();
    let change_dust = change_address.script_pubkey.minimal_non_dust().to_sat();
    let mut selected = Vec::new();
    let mut total_input_sat = 0_u64;

    for candidate in candidates {
        total_input_sat = total_input_sat
            .checked_add(candidate.utxo.value)
            .ok_or(WalletError::TransactionFailed)?;
        selected.push(candidate);

        let fee_with_change = fee_for_size(
            estimated_vsize(
                selected.len(),
                recipient_script.len(),
                true,
            ),
            fee_rate_sat_vb,
        )?;
        let with_change_total = amount_sat
            .checked_add(fee_with_change)
            .ok_or(WalletError::TransactionFailed)?;

        if total_input_sat >= with_change_total {
            let change_sat = total_input_sat - with_change_total;
            if change_sat >= change_dust {
                let vsize = estimated_vsize(
                    selected.len(),
                    recipient_script.len(),
                    true,
                );
                return Ok(SendPlan {
                    recipient: recipient.to_string(),
                    amount_sat,
                    fee_sat: fee_with_change,
                    fee_rate_sat_vb,
                    change_sat,
                    change_address: Some(change_address.address.clone()),
                    vsize,
                    total_input_sat,
                    input_count: selected.len(),
                    inputs: selected,
                });
            }

            let fee_without_change = fee_for_size(
                estimated_vsize(
                    selected.len(),
                    recipient_script.len(),
                    false,
                ),
                fee_rate_sat_vb,
            )?;
            let without_change_total = amount_sat
                .checked_add(fee_without_change)
                .ok_or(WalletError::TransactionFailed)?;
            if total_input_sat >= without_change_total {
                return Ok(SendPlan {
                    recipient: recipient.to_string(),
                    amount_sat,
                    fee_sat: total_input_sat - amount_sat,
                    fee_rate_sat_vb,
                    change_sat: 0,
                    change_address: None,
                    vsize: estimated_vsize(
                        selected.len(),
                        recipient_script.len(),
                        false,
                    ),
                    total_input_sat,
                    input_count: selected.len(),
                    inputs: selected,
                });
            }
        }
    }

    Err(WalletError::InsufficientFunds {
        available_sat,
        required_sat: amount_sat,
    })
}

/// Signs a reviewed send plan locally.
pub fn sign_send_plan(
    session: &WalletSession,
    plan: &SendPlan,
) -> Result<SignedTransaction, WalletError> {
    if plan.inputs.is_empty()
        || plan.amount_sat == 0
        || plan.input_count != plan.inputs.len()
    {
        return Err(WalletError::TransactionFailed);
    }

    let recipient = validate_recipient(&plan.recipient)?;
    let mut outputs = vec![TxOut {
        value: Amount::from_sat(plan.amount_sat),
        script_pubkey: recipient.script_pubkey(),
    }];
    if plan.change_sat > 0 {
        let change_address = plan
            .change_address
            .as_deref()
            .ok_or(WalletError::TransactionFailed)?;
        let change_address = validate_recipient(change_address)?;
        outputs.push(TxOut {
            value: Amount::from_sat(plan.change_sat),
            script_pubkey: change_address.script_pubkey(),
        });
    }

    let total_output = outputs
        .iter()
        .try_fold(0_u64, |total, output| total.checked_add(output.value.to_sat()))
        .ok_or(WalletError::TransactionFailed)?;
    let expected_fee = plan
        .total_input_sat
        .checked_sub(total_output)
        .ok_or(WalletError::TransactionFailed)?;
    if expected_fee != plan.fee_sat {
        return Err(WalletError::TransactionFailed);
    }

    let mut inputs = Vec::with_capacity(plan.inputs.len());
    for input in &plan.inputs {
        inputs.push(TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str(&input.utxo.txid)
                    .map_err(|_| WalletError::TransactionFailed)?,
                vout: input.utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        });
    }

    let mut transaction = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs,
        output: outputs,
    };

    let secp = Secp256k1::new();
    let master = Xpriv::new_master(NetworkKind::Test, &session.seed[..])
        .map_err(|_| WalletError::KeyDerivationFailed)?;

    let witnesses = {
        let mut cache = SighashCache::new(&transaction);
        let mut witnesses = Vec::with_capacity(plan.inputs.len());

        for (index, input) in plan.inputs.iter().enumerate() {
            let path = derivation_path(input.branch, input.index)?;
            let child = master
                .derive_priv(&secp, &path)
                .map_err(|_| WalletError::KeyDerivationFailed)?;
            let public_key = child.private_key.public_key(&secp);
            let expected_script =
                Address::p2wpkh(&CompressedPublicKey(public_key), Network::Testnet)
                    .script_pubkey();
            if expected_script != input.script_pubkey {
                return Err(WalletError::TransactionFailed);
            }

            let sighash = cache
                .p2wpkh_signature_hash(
                    index,
                    &input.script_pubkey,
                    Amount::from_sat(input.utxo.value),
                    EcdsaSighashType::All,
                )
                .map_err(|_| WalletError::TransactionFailed)?;
            let message = Message::from_digest(sighash.to_byte_array());
            let signature = secp.sign_ecdsa(&message, &child.private_key);
            let signature = EcdsaSignature {
                signature,
                sighash_type: EcdsaSighashType::All,
            };
            witnesses.push(Witness::from_slice(&[
                signature.to_vec(),
                public_key.serialize().to_vec(),
            ]));
        }

        witnesses
    };

    for (input, witness) in transaction.input.iter_mut().zip(witnesses) {
        input.witness = witness;
    }

    let fee_sat = plan.fee_sat;
    let vsize = transaction.vsize() as u64;

    Ok(SignedTransaction {
        raw_hex: serialize_hex(&transaction),
        txid: transaction.compute_txid().to_string(),
        fee_sat,
        vsize,
        input_count: transaction.input.len(),
        change_sat: plan.change_sat,
    })
}

fn derivation_path(
    branch: AddressBranch,
    index: u32,
) -> Result<DerivationPath, WalletError> {
    let path = format!("m/84'/1'/0'/{}/{}", branch.index(), index);
    DerivationPath::from_str(&path).map_err(|_| WalletError::KeyDerivationFailed)
}

fn derive_address(
    master: &Xpriv,
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    branch: AddressBranch,
    index: u32,
) -> Result<DerivedAddress, WalletError> {
    let path = derivation_path(branch, index)?;
    let child = master
        .derive_priv(secp, &path)
        .map_err(|_| WalletError::KeyDerivationFailed)?;
    let public_key = child.private_key.public_key(secp);
    let address = Address::p2wpkh(&CompressedPublicKey(public_key), Network::Testnet);

    Ok(DerivedAddress {
        branch,
        index,
        address: address.to_string(),
        script_pubkey: address.script_pubkey(),
    })
}

fn normalize_phrase(phrase: &str) -> Zeroizing<String> {
    Zeroizing::new(phrase.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn parse_import_mnemonic(phrase: &str) -> Result<Mnemonic, WalletError> {
    let word_count = phrase.split_whitespace().count();
    if !matches!(word_count, 12 | 24) {
        return Err(WalletError::UnsupportedWordCount { found: word_count });
    }

    Mnemonic::parse(phrase).map_err(map_bip39_error)
}

fn map_bip39_error(error: Bip39Error) -> WalletError {
    match error {
        Bip39Error::BadWordCount(found) => WalletError::InvalidWordCount { found },
        Bip39Error::UnknownWord(index) => WalletError::UnknownWord { index },
        Bip39Error::InvalidChecksum => WalletError::InvalidChecksum,
        _ => WalletError::InvalidRecoveryPhrase,
    }
}

fn derive_address_from_mnemonic(mnemonic: &Mnemonic) -> Result<String, WalletError> {
    let seed = Zeroizing::new(mnemonic.to_seed(""));
    let secp = Secp256k1::new();
    let path = DerivationPath::from_str("m/84'/1'/0'/0/0")
        .map_err(|_| WalletError::KeyDerivationFailed)?;
    let master = Xpriv::new_master(NetworkKind::Test, &seed[..])
        .map_err(|_| WalletError::KeyDerivationFailed)?;
    let child = master
        .derive_priv(&secp, &path)
        .map_err(|_| WalletError::KeyDerivationFailed)?;
    let public_key = child.private_key.public_key(&secp);
    let address = Address::p2wpkh(&CompressedPublicKey(public_key), Network::Testnet);
    Ok(address.to_string())
}

fn estimated_vsize(input_count: usize, recipient_script_len: usize, include_change: bool) -> u64 {
    let input_count = input_count as u64;
    let output_count = if include_change { 2 } else { 1 };
    let recipient_output =
        8 + varint_size(recipient_script_len as u64) + recipient_script_len as u64;
    let change_output = if include_change {
        8 + varint_size(P2WPKH_SCRIPT_LEN) + P2WPKH_SCRIPT_LEN
    } else {
        0
    };
    let outputs = recipient_output + change_output;

    let base_size = 4
        + varint_size(input_count)
        + input_count * P2WPKH_INPUT_BASE_BYTES
        + varint_size(output_count)
        + outputs
        + 4;
    let witness_size = WITNESS_MARKER_BYTES + input_count * P2WPKH_WITNESS_BYTES;
    let weight = base_size * 4 + witness_size;
    (weight + 3) / 4
}

fn varint_size(value: u64) -> u64 {
    match value {
        0..=0xfc => 1,
        0xfd..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

fn fee_for_size(vsize: u64, fee_rate_sat_vb: u64) -> Result<u64, WalletError> {
    if vsize == 0 {
        return Err(WalletError::TransactionFailed);
    }
    vsize
        .checked_mul(fee_rate_sat_vb)
        .ok_or(WalletError::TransactionFailed)
}

#[cfg(test)]
mod tests {
    use bitcoin::consensus::encode::deserialize_hex;

    use super::*;

    const TEST_PHRASE_12: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const TEST_PHRASE_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    const TEST_PHRASE_12_ADDRESS: &str = "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl";
    const TEST_TXID: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn test_session() -> WalletSession {
        WalletSession::from_phrase(TEST_PHRASE_12).unwrap()
    }

    fn test_utxo(address: &str, value: u64) -> AddressUtxo {
        AddressUtxo {
            address: address.to_owned(),
            utxo: WalletUtxo {
                txid: TEST_TXID.to_owned(),
                vout: 0,
                value,
                status: bitsabi_shared::TransactionStatus {
                    confirmed: true,
                    block_height: Some(1),
                    block_time: Some(1),
                },
            },
        }
    }

    #[test]
    fn valid_recovery_phrase_imports() {
        let address = import_testnet_address(TEST_PHRASE_12)
            .expect("the public 12-word test phrase should import");

        assert!(address.starts_with("tb1q"));
    }

    #[test]
    fn valid_24_word_recovery_phrase_imports() {
        let address = import_testnet_address(TEST_PHRASE_24)
            .expect("the public 24-word test phrase should import");

        assert!(address.starts_with("tb1q"));
    }

    #[test]
    fn import_rejects_unsupported_word_counts() {
        let result = validate_recovery_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon",
        );

        assert_eq!(result, Err(WalletError::UnsupportedWordCount { found: 15 }));
    }

    #[test]
    fn import_rejects_invalid_checksum() {
        let result = validate_recovery_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon",
        );

        assert_eq!(result, Err(WalletError::InvalidChecksum));
    }

    #[test]
    fn import_rejects_unknown_words() {
        let result = validate_recovery_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon notaword",
        );

        assert_eq!(result, Err(WalletError::UnknownWord { index: 11 }));
    }

    #[test]
    fn imports_are_deterministic() {
        let first = import_testnet_address(TEST_PHRASE_12).unwrap();
        let second = import_testnet_address(TEST_PHRASE_12).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn import_accepts_extra_whitespace() {
        let messy_phrase = format!("  {}\n ", TEST_PHRASE_12.replace(' ', "  \n "));
        let address = import_testnet_address(&messy_phrase).unwrap();

        assert_eq!(address, TEST_PHRASE_12_ADDRESS);
    }

    #[test]
    fn derived_address_has_testnet_native_segwit_format() {
        let address = derive_testnet_address(TEST_PHRASE_12).unwrap();

        assert_eq!(address, TEST_PHRASE_12_ADDRESS);
        assert!(
            address.starts_with("tb1q"),
            "expected a native SegWit testnet address, got: {address}"
        );
    }

    #[test]
    fn rejects_an_invalid_recovery_phrase() {
        let result = derive_testnet_address("this is not a valid wallet phrase");

        assert!(result.is_err());
    }

    #[test]
    fn generation_failure_is_returned_without_a_wallet() {
        let result = generate_testnet_wallet_with(|_| Err(WalletError::SecureRandomUnavailable));

        assert!(matches!(result, Err(WalletError::SecureRandomUnavailable)));
    }

    #[test]
    fn generation_from_known_entropy_produces_a_valid_wallet() {
        let wallet = generate_testnet_wallet_with(|entropy| {
            entropy.fill(0);
            Ok(())
        })
        .expect("generation with known entropy should succeed");

        assert_eq!(
            wallet.phrase().split_whitespace().count(),
            CREATED_WORD_COUNT
        );
        assert_eq!(wallet.phrase(), TEST_PHRASE_12);
        assert!(wallet.address().starts_with("tb1q"));
    }

    #[test]
    fn production_generation_uses_a_secure_source_and_returns_a_wallet() {
        let wallet = create_testnet_wallet().expect("the host secure RNG should be available");

        assert_eq!(
            wallet.phrase().split_whitespace().count(),
            CREATED_WORD_COUNT
        );
        assert!(wallet.address().starts_with("tb1q"));
    }

    #[test]
    fn a_session_scans_receive_and_change_gap_addresses() {
        let session = test_session();
        let addresses = session.scan_addresses();

        assert_eq!(
            addresses.len(),
            (RECEIVE_GAP_LIMIT + CHANGE_GAP_LIMIT) as usize
        );
        assert_eq!(
            session.current_receive_address().address(),
            TEST_PHRASE_12_ADDRESS
        );
        assert_eq!(session.receive_address_count(), 1);
    }

    #[test]
    fn additional_receiving_addresses_are_retained() {
        let mut session = test_session();
        let first = session.current_receive_address().address().to_owned();
        let second = session
            .generate_receive_address()
            .unwrap()
            .address()
            .to_owned();

        assert_ne!(first, second);
        assert_eq!(session.receive_address_count(), 2);
        assert_eq!(session.visible_receive_addresses()[0].address(), first);
        assert_eq!(session.current_receive_address().address(), second);
    }

    #[test]
    fn btc_amount_parsing_is_exact() {
        assert_eq!(parse_btc_amount("0.0005").unwrap(), 50_000);
        assert_eq!(parse_btc_amount("1").unwrap(), 100_000_000);
        assert_eq!(parse_btc_amount("0.00000001").unwrap(), 1);
        assert_eq!(parse_btc_amount(" 0.1 ").unwrap(), 10_000_000);
        assert!(parse_btc_amount("").is_err());
        assert!(parse_btc_amount("0").is_err());
        assert!(parse_btc_amount("-1").is_err());
        assert!(parse_btc_amount("0.123456789").is_err());
        assert!(parse_btc_amount("1.2.3").is_err());
        assert!(parse_btc_amount("abc").is_err());
    }

    #[test]
    fn satoshi_formatting_is_readable() {
        assert_eq!(format_sats(100_000_000), "1 BTC");
        assert_eq!(format_sats(50_000), "0.0005 BTC");
        assert_eq!(format_sats(1), "0.00000001 BTC");
        assert_eq!(format_signed_sats(-50_000), "-0.0005 BTC");
    }

    #[test]
    fn send_plan_selects_confirmed_utxos_and_returns_change() {
        let session = test_session();
        let utxos = vec![test_utxo(TEST_PHRASE_12_ADDRESS, 100_000)];

        let plan = plan_send(
            &session,
            &utxos,
            TEST_PHRASE_12_ADDRESS,
            50_000,
            2,
        )
        .unwrap();

        assert_eq!(plan.input_count, 1);
        assert_eq!(plan.amount_sat, 50_000);
        assert!(plan.fee_sat > 0);
        assert!(plan.change_sat > 0);
        assert!(plan.change_address.is_some());
    }

    #[test]
    fn send_plan_rejects_insufficient_and_unconfirmed_funds() {
        let session = test_session();
        let utxos = vec![test_utxo(TEST_PHRASE_12_ADDRESS, 60_000)];

        assert!(matches!(
            plan_send(&session, &utxos, TEST_PHRASE_12_ADDRESS, 100_000, 2),
            Err(WalletError::InsufficientFunds { .. })
        ));

        let mut unconfirmed = test_utxo(TEST_PHRASE_12_ADDRESS, 100_000);
        unconfirmed.utxo.status.confirmed = false;
        assert_eq!(
            plan_send(&session, &[unconfirmed], TEST_PHRASE_12_ADDRESS, 50_000, 2),
            Err(WalletError::NoConfirmedFunds)
        );
    }

    #[test]
    fn send_plan_rejects_mainnet_recipients_and_bad_fee_rates() {
        let session = test_session();
        let utxos = vec![test_utxo(TEST_PHRASE_12_ADDRESS, 100_000)];

        assert_eq!(
            plan_send(
                &session,
                &utxos,
                "bc1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl",
                50_000,
                2,
            ),
            Err(WalletError::InvalidRecipient)
        );
        assert_eq!(
            plan_send(&session, &utxos, TEST_PHRASE_12_ADDRESS, 50_000, 0),
            Err(WalletError::InvalidFeeRate)
        );
    }

    #[test]
    fn signing_produces_a_valid_native_segwit_transaction() {
        let session = test_session();
        let utxos = vec![test_utxo(TEST_PHRASE_12_ADDRESS, 100_000)];
        let plan = plan_send(&session, &utxos, TEST_PHRASE_12_ADDRESS, 50_000, 2).unwrap();

        let signed = sign_send_plan(&session, &plan).unwrap();
        let transaction: Transaction = deserialize_hex(&signed.raw_hex).unwrap();

        assert_eq!(transaction.input.len(), 1);
        assert_eq!(transaction.output.len(), 2);
        assert_eq!(transaction.input[0].witness.iter().count(), 2);
        assert_eq!(transaction.compute_txid().to_string(), signed.txid);
        assert_eq!(signed.fee_sat, plan.fee_sat);
        assert!(signed.vsize > 0);
    }

    #[test]
    fn estimated_vsize_is_close_to_a_one_input_two_output_transaction() {
        assert_eq!(estimated_vsize(1, 22, true), 141);
        assert_eq!(estimated_vsize(1, 22, false), 110);
    }
}
