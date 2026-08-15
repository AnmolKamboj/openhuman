//! BIP-32 derivation on secp256k1, shared by Bitcoin, EVM and Tron.
//!
//! All three chains use the same scheme and differ only in what they do with
//! the resulting key, so the walk lives here once rather than three times.
//!
//! # This is delegated on purpose
//!
//! Rolling BIP-32 by hand is possible — it is HMAC-SHA512 plus a scalar
//! addition — but an off-by-one in the hardened-index encoding produces a
//! *valid key for the wrong account*, which is silent, unrecoverable, and
//! exactly the kind of bug not worth risking to avoid a dependency.
//!
//! That reasoning is unchanged from when this used `bitcoin`'s `Xpriv`. What
//! changed is which vetted implementation it delegates to: `coins-bip32`, whose
//! secp256k1 backend is the pure-Rust `k256` rather than the `secp256k1` C
//! library. The derived key is identical either way — BIP-32 is a specification,
//! not an implementation detail, and [`super::test`] pins the addresses against
//! the same fixed mnemonic as before the swap. `coins-bip32` is also the code
//! path `coins-bip39` already uses beneath [`super::seed_from_mnemonic`], so
//! this removes a native C build and a second elliptic-curve stack without
//! adding anything to the graph.
//!
//! Contrast [`crate::openhuman::web3::wallet::primitives::address::btc`], which *is* hand-rolled. The difference is
//! the failure mode, not the difficulty: a wrong parser is caught by the first
//! test vector, a wrong derivation is caught by nobody.

use std::str::FromStr;

use coins_bip32::path::DerivationPath;
use coins_bip32::prelude::SigningKey;
use coins_bip32::xkeys::XPriv;

use super::{Error, Result};

/// A secp256k1 key derived at a BIP-32 path.
pub(super) struct Secp256k1Key {
    pub(super) secret: SigningKey,
}

impl Secp256k1Key {
    /// The 65-byte uncompressed SEC1 encoding, `0x04` prefix included.
    ///
    /// EVM and Tron both hash this — minus the prefix byte — with Keccak-256 to
    /// form an address.
    pub(super) fn uncompressed_public(&self) -> [u8; 65] {
        let encoded = self.secret.verifying_key().to_encoded_point(false);
        let mut out = [0u8; 65];
        // Uncompressed SEC1 is 65 bytes by definition, so this cannot be short.
        out.copy_from_slice(encoded.as_bytes());
        out
    }

    /// The 33-byte compressed SEC1 encoding.
    ///
    /// Bitcoin hashes this — not the uncompressed form — to form a P2WPKH
    /// address. Using the wrong one yields a well-formed address for an account
    /// nobody holds the key to, which is why the two encodings are separate
    /// named methods rather than one with a boolean.
    pub(super) fn compressed_public(&self) -> [u8; 33] {
        let encoded = self.secret.verifying_key().to_encoded_point(true);
        let mut out = [0u8; 33];
        out.copy_from_slice(encoded.as_bytes());
        out
    }

    /// The 32-byte secret scalar.
    pub(super) fn secret_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes().into()
    }
}

/// Walk `path` from the master key for `seed`.
///
/// The derived secret does not depend on a network: BIP-32 version bytes only
/// matter when an extended key is serialized, which never happens here. The
/// same walk is therefore correct for Bitcoin, EVM and Tron alike.
pub(super) fn derive(seed: &[u8], path: &str) -> Result<Secp256k1Key> {
    let master = XPriv::root_from_seed(seed, None).map_err(|_| Error::Derivation {
        step: "BIP-32 master key",
    })?;
    let parsed = DerivationPath::from_str(path).map_err(|e| Error::InvalidPath {
        path: path.to_string(),
        reason: e.to_string(),
    })?;

    // Depth is checked here rather than left to the backend, because
    // `coins-bip32` does not check it: `derive_child` increments a `u8` depth
    // unguarded, which panics in a debug build and **wraps silently in a
    // release build** — deriving at a wrapped depth instead of refusing. The
    // `bitcoin` implementation this replaced returned `MaximumDepthExceeded`,
    // so without this the swap would have traded a clean error for a wrong key.
    //
    // The master node is depth 0, leaving 255 usable levels. No real path comes
    // close; the bound exists so a hostile or generated one cannot get through.
    if parsed.len() > usize::from(u8::MAX) {
        return Err(Error::InvalidPath {
            path: path.to_string(),
            reason: format!(
                "BIP-32 depth is limited to {} levels, got {}",
                u8::MAX,
                parsed.len()
            ),
        });
    }

    let child = master.derive_path(parsed).map_err(|_| Error::Derivation {
        step: "BIP-32 child key",
    })?;

    let secret: &SigningKey = child.as_ref();
    Ok(Secp256k1Key {
        secret: secret.clone(),
    })
}
