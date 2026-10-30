//! Encrypted browser storage for a persistent testnet wallet.
//!
//! The recovery phrase is encrypted with a password-derived key before it is
//! written to browser storage. Plaintext secrets are handled here as carefully
//! as practical, but browser memory cannot guarantee complete erasure.

use std::fmt;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::wallet::validate_recovery_phrase;

/// Storage key used for the encrypted wallet envelope.
pub const STORAGE_KEY: &str = "bitsabi:wallet:encrypted:v1";

/// Minimum accepted password length for a persisted wallet.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Maximum accepted serialized envelope size in bytes.
pub const MAX_ENCRYPTED_WALLET_JSON_BYTES: usize = 8 * 1024;
/// Maximum accepted base64 ciphertext length in characters.
pub const MAX_ENCRYPTED_CIPHERTEXT_CHARS: usize = 2048;
/// Maximum accepted decoded secret size in bytes.
pub const MAX_ENCRYPTED_SECRET_BYTES: usize = 1024;

/// Argon2id memory cost in KiB. 16 MiB is a browser-friendly balance between
/// resistance to offline guessing and interactive unlock latency.
pub const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
/// Argon2id iteration count.
pub const ARGON2_ITERATIONS: u32 = 2;
/// Argon2id parallelism.
pub const ARGON2_PARALLELISM: u32 = 1;

/// Encrypted wallet envelope stored in the browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncryptedWallet {
    /// Envelope format version.
    pub version: u8,
    /// Argon2 salt.
    pub salt: [u8; 16],
    /// AES-GCM nonce.
    pub nonce: [u8; 12],
    /// Ciphertext including the authentication tag, base64-encoded.
    pub ciphertext: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletStorageError {
    InvalidPhrase,
    PasswordTooShort,
    EncryptionFailed,
    DecryptionFailed,
    CorruptedEnvelope,
    StorageUnavailable,
    SerializationFailed,
}

impl fmt::Display for WalletStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPhrase => formatter.write_str("The recovery phrase is not valid."),
            Self::PasswordTooShort => {
                write!(
                    formatter,
                    "Use a password with at least {MIN_PASSWORD_LEN} characters."
                )
            }
            Self::EncryptionFailed => formatter.write_str("The wallet could not be encrypted."),
            Self::DecryptionFailed => {
                formatter.write_str("The password is incorrect or the wallet is corrupted.")
            }
            Self::CorruptedEnvelope => formatter.write_str("The saved wallet is corrupted."),
            Self::StorageUnavailable => formatter.write_str("Browser storage is unavailable."),
            Self::SerializationFailed => formatter.write_str("The wallet could not be serialized."),
        }
    }
}

impl std::error::Error for WalletStorageError {}

/// Validate an encrypted envelope before attempting decryption.
pub fn validate_envelope(envelope: &EncryptedWallet) -> Result<(), WalletStorageError> {
    if envelope.version != 1 {
        return Err(WalletStorageError::CorruptedEnvelope);
    }
    if envelope.ciphertext.is_empty() || envelope.ciphertext.len() > MAX_ENCRYPTED_CIPHERTEXT_CHARS
    {
        return Err(WalletStorageError::CorruptedEnvelope);
    }

    let ciphertext = BASE64
        .decode(&envelope.ciphertext)
        .map_err(|_| WalletStorageError::CorruptedEnvelope)?;
    // AES-GCM authentication tag is 16 bytes; anything shorter is malformed.
    if ciphertext.len() < 16 || ciphertext.len() > MAX_ENCRYPTED_SECRET_BYTES {
        return Err(WalletStorageError::CorruptedEnvelope);
    }

    Ok(())
}

/// Encrypt arbitrary secret bytes with a password.
pub fn encrypt_bytes(data: &[u8], password: &str) -> Result<EncryptedWallet, WalletStorageError> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(WalletStorageError::PasswordTooShort);
    }

    let mut salt = [0_u8; 16];
    getrandom::getrandom(&mut salt).map_err(|_| WalletStorageError::EncryptionFailed)?;
    let mut nonce_bytes = [0_u8; 12];
    getrandom::getrandom(&mut nonce_bytes).map_err(|_| WalletStorageError::EncryptionFailed)?;

    let mut key = Zeroizing::new([0_u8; 32]);
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|_| WalletStorageError::EncryptionFailed)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &salt, key.as_mut())
        .map_err(|_| WalletStorageError::EncryptionFailed)?;

    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|_| WalletStorageError::EncryptionFailed)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|_| WalletStorageError::EncryptionFailed)?;

    Ok(EncryptedWallet {
        version: 1,
        salt,
        nonce: nonce_bytes,
        ciphertext: BASE64.encode(ciphertext),
    })
}

/// Decrypt arbitrary secret bytes from an encrypted envelope.
pub fn decrypt_bytes(
    envelope: &EncryptedWallet,
    password: &str,
) -> Result<Zeroizing<Vec<u8>>, WalletStorageError> {
    validate_envelope(envelope)?;
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(WalletStorageError::PasswordTooShort);
    }

    let ciphertext = BASE64
        .decode(&envelope.ciphertext)
        .map_err(|_| WalletStorageError::CorruptedEnvelope)?;

    let mut key = Zeroizing::new([0_u8; 32]);
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|_| WalletStorageError::DecryptionFailed)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &envelope.salt, key.as_mut())
        .map_err(|_| WalletStorageError::DecryptionFailed)?;

    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|_| WalletStorageError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(&envelope.nonce);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| WalletStorageError::DecryptionFailed)?;

    Ok(Zeroizing::new(plaintext))
}

/// Encrypt a recovery phrase with a password.
pub fn encrypt_wallet(phrase: &str, password: &str) -> Result<EncryptedWallet, WalletStorageError> {
    validate_recovery_phrase(phrase).map_err(|_| WalletStorageError::InvalidPhrase)?;
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(WalletStorageError::PasswordTooShort);
    }

    encrypt_bytes(phrase.as_bytes(), password)
}

/// Decrypt a recovery phrase from an encrypted envelope.
pub fn decrypt_wallet(
    envelope: &EncryptedWallet,
    password: &str,
) -> Result<Zeroizing<String>, WalletStorageError> {
    if envelope.version != 1 {
        return Err(WalletStorageError::CorruptedEnvelope);
    }
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(WalletStorageError::PasswordTooShort);
    }

    let plaintext = decrypt_bytes(envelope, password)?;
    let mut plaintext = plaintext;
    let phrase_bytes = std::mem::take(plaintext.as_mut());
    let phrase =
        String::from_utf8(phrase_bytes).map_err(|_| WalletStorageError::DecryptionFailed)?;
    validate_recovery_phrase(&phrase).map_err(|_| WalletStorageError::DecryptionFailed)?;
    Ok(Zeroizing::new(phrase))
}

/// Serialize an envelope to a JSON string.
pub fn envelope_to_json(envelope: &EncryptedWallet) -> Result<String, WalletStorageError> {
    serde_json::to_string(envelope).map_err(|_| WalletStorageError::SerializationFailed)
}

/// Parse an envelope from a JSON string.
pub fn envelope_from_json(value: &str) -> Result<EncryptedWallet, WalletStorageError> {
    if value.len() > MAX_ENCRYPTED_WALLET_JSON_BYTES {
        return Err(WalletStorageError::CorruptedEnvelope);
    }
    let envelope: EncryptedWallet =
        serde_json::from_str(value).map_err(|_| WalletStorageError::CorruptedEnvelope)?;
    validate_envelope(&envelope)?;
    Ok(envelope)
}

#[cfg(target_arch = "wasm32")]
pub fn save_wallet(envelope: &EncryptedWallet) -> Result<(), WalletStorageError> {
    use gloo_storage::{LocalStorage, Storage};
    LocalStorage::set(STORAGE_KEY, envelope).map_err(|_| WalletStorageError::StorageUnavailable)
}

#[cfg(target_arch = "wasm32")]
pub fn load_wallet() -> Option<EncryptedWallet> {
    use gloo_storage::{LocalStorage, Storage};
    LocalStorage::get(STORAGE_KEY).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn clear_wallet() {
    use gloo_storage::{LocalStorage, Storage};
    LocalStorage::delete(STORAGE_KEY);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_wallet(_envelope: &EncryptedWallet) -> Result<(), WalletStorageError> {
    Err(WalletStorageError::StorageUnavailable)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_wallet() -> Option<EncryptedWallet> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_wallet() {}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn encrypt_and_decrypt_round_trip() {
        let envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        let phrase = decrypt_wallet(&envelope, "correct horse battery staple").unwrap();
        assert_eq!(phrase.as_str(), TEST_PHRASE);
    }

    #[test]
    fn wrong_password_fails() {
        let envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        assert!(matches!(
            decrypt_wallet(&envelope, "wrong password 123"),
            Err(WalletStorageError::DecryptionFailed)
        ));
    }

    #[test]
    fn short_password_is_rejected() {
        assert!(matches!(
            encrypt_wallet(TEST_PHRASE, "short"),
            Err(WalletStorageError::PasswordTooShort)
        ));
    }

    #[test]
    fn invalid_phrase_is_rejected() {
        assert!(matches!(
            encrypt_wallet("not a wallet phrase", "correct horse battery staple"),
            Err(WalletStorageError::InvalidPhrase)
        ));
    }

    #[test]
    fn native_storage_is_unavailable() {
        let envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        assert!(matches!(
            save_wallet(&envelope),
            Err(WalletStorageError::StorageUnavailable)
        ));
        assert!(load_wallet().is_none());
    }

    #[test]
    fn oversized_envelope_is_rejected() {
        let oversized = "a".repeat(MAX_ENCRYPTED_WALLET_JSON_BYTES + 1);
        assert!(matches!(
            envelope_from_json(&oversized),
            Err(WalletStorageError::CorruptedEnvelope)
        ));
    }

    #[test]
    fn password_validation_uses_characters() {
        let multibyte = "é".repeat(MIN_PASSWORD_LEN - 1);
        assert!(matches!(
            encrypt_wallet(TEST_PHRASE, &multibyte),
            Err(WalletStorageError::PasswordTooShort)
        ));
    }

    #[test]
    fn corrupted_json_envelope_is_rejected() {
        assert!(matches!(
            envelope_from_json("{not json}"),
            Err(WalletStorageError::CorruptedEnvelope)
        ));
    }

    #[test]
    fn repeated_encryption_uses_fresh_salt_and_nonce() {
        let first = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        let second = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();

        assert_ne!(first.salt, second.salt);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn validate_envelope_rejects_malformed_values() {
        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        envelope.version = 2;
        assert!(matches!(
            validate_envelope(&envelope),
            Err(WalletStorageError::CorruptedEnvelope)
        ));

        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        envelope.ciphertext = String::new();
        assert!(matches!(
            validate_envelope(&envelope),
            Err(WalletStorageError::CorruptedEnvelope)
        ));

        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        envelope.ciphertext = "!!!!".to_owned();
        assert!(matches!(
            validate_envelope(&envelope),
            Err(WalletStorageError::CorruptedEnvelope)
        ));
    }

    #[test]
    fn envelope_json_round_trips() {
        let envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        let json = envelope_to_json(&envelope).unwrap();
        let parsed = envelope_from_json(&json).unwrap();
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn unsupported_envelope_version_is_rejected() {
        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        envelope.version = 2;
        assert!(matches!(
            decrypt_wallet(&envelope, "correct horse battery staple"),
            Err(WalletStorageError::CorruptedEnvelope)
        ));
    }

    #[test]
    fn decrypt_rejects_short_password() {
        let envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        assert!(matches!(
            decrypt_wallet(&envelope, "short"),
            Err(WalletStorageError::PasswordTooShort)
        ));
    }

    #[test]
    fn decrypt_rejects_invalid_recovery_phrase() {
        // Encrypt raw bytes that are not a valid phrase, then tamper version? Instead test decrypt_wallet on envelope whose plaintext is invalid by using encrypt_bytes directly.
        let envelope =
            encrypt_bytes(b"not a recovery phrase", "correct horse battery staple").unwrap();
        assert!(matches!(
            decrypt_wallet(&envelope, "correct horse battery staple"),
            Err(WalletStorageError::DecryptionFailed)
        ));
    }

    #[test]
    fn modified_ciphertext_is_rejected() {
        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        // Flip a character in the base64 payload.
        let mut chars: Vec<char> = envelope.ciphertext.chars().collect();
        if let Some(last) = chars.last_mut() {
            *last = if *last == 'A' { 'B' } else { 'A' };
        }
        envelope.ciphertext = chars.into_iter().collect();
        assert!(matches!(
            decrypt_wallet(&envelope, "correct horse battery staple"),
            Err(WalletStorageError::DecryptionFailed | WalletStorageError::CorruptedEnvelope)
        ));
    }

    #[test]
    fn corrupted_ciphertext_fails() {
        let mut envelope = encrypt_wallet(TEST_PHRASE, "correct horse battery staple").unwrap();
        envelope.ciphertext = "AAAA".to_owned();
        assert!(matches!(
            decrypt_wallet(&envelope, "correct horse battery staple"),
            Err(WalletStorageError::DecryptionFailed | WalletStorageError::CorruptedEnvelope)
        ));
    }
}
