//! Solana key derivation: SLIP-0010 on ed25519, address is the public key.

use ed25519_dalek::SigningKey;

use super::{seed_from_mnemonic, slip10, DerivedKey, Result};
use crate::openhuman::web3::wallet::primitives::chain::Chain;

/// Derive the Solana signing key and address for `path`.
///
/// A Solana address *is* the ed25519 public key in base58 — there is no hash
/// and no version byte, unlike every other chain here.
pub(super) fn derive(mnemonic: &str, path: &str) -> Result<DerivedKey> {
    let seed = seed_from_mnemonic(mnemonic)?;
    let secret = slip10::derive(&seed, path)?;
    let signing = SigningKey::from_bytes(&secret);
    let address = crate::openhuman::web3::wallet::primitives::address::solana::encode(
        &signing.verifying_key().to_bytes(),
    );
    Ok(DerivedKey::new(Chain::Solana, address, secret.to_vec()))
}
