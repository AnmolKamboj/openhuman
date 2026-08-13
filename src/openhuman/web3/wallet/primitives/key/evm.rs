//! EVM key derivation: BIP-32 on secp256k1, address via Keccak-256.

use sha3::{Digest, Keccak256};

use super::{bip32, seed_from_mnemonic, DerivedKey, Result};
use crate::openhuman::web3::wallet::primitives::chain::Chain;

/// Derive the EVM signing key and address for `path`.
pub(super) fn derive(mnemonic: &str, path: &str) -> Result<DerivedKey> {
    let seed = seed_from_mnemonic(mnemonic)?;
    let key = bip32::derive(&seed, path)?;
    let address = address_from_public(&key.uncompressed_public());
    Ok(DerivedKey::new(
        Chain::Evm,
        address,
        key.secret_bytes().to_vec(),
    ))
}

/// An EVM address is the last 20 bytes of the Keccak-256 hash of the
/// uncompressed public key with its `0x04` prefix byte removed.
///
/// Returned EIP-55 checksummed, which is the canonical display form and what
/// every explorer and wallet shows.
fn address_from_public(uncompressed: &[u8; 65]) -> String {
    let hash = Keccak256::digest(&uncompressed[1..]);
    let body = hex_lower(&hash[12..]);
    // The address was just built from a hash, so it is well-formed by
    // construction and checksumming cannot fail.
    crate::openhuman::web3::wallet::primitives::address::evm::to_checksummed(&body)
        .unwrap_or_else(|_| format!("0x{body}"))
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}
