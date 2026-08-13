//! Per-chain address validation.
//!
//! Each submodule owns one chain's address format and exposes the same core
//! shape: a `validate` returning the trimmed address, plus whatever
//! chain-specific conversions are genuinely useful (`solana::decode`,
//! `tron::to_hex`, `btc::validate_sender`).
//!
//! [`validate`] dispatches across all of them for chain-generic callers.
//!
//! ## What validation does and does not prove
//!
//! Every function here answers one question: *is this string a well-formed
//! address on this chain*. None of them touch the network, so none can tell
//! you an account exists, is funded, or is controlled by anyone in particular.
//!
//! How much a successful validation is worth also varies sharply by chain, and
//! it is worth being explicit about because it is easy to assume otherwise:
//!
//! | Chain | Checksum | A single typo is… |
//! | --- | --- | --- |
//! | Bitcoin | yes (base58check / bech32) | caught |
//! | Tron | yes (base58check) | caught |
//! | EVM | optional (EIP-55, only if mixed-case) | usually *not* caught |
//! | Solana | none | *not reliably* caught |
//!
//! For EVM, `evm::is_checksum_valid` recovers the typo protection when the
//! caller has a mixed-case address. For Solana there is nothing to recover:
//! confirm the address out of band.

use crate::openhuman::web3::wallet::primitives::chain::Chain;
use crate::openhuman::web3::wallet::primitives::Result;

#[cfg(feature = "web3")]
pub mod btc;
#[cfg(feature = "web3")]
pub mod evm;
#[cfg(feature = "web3")]
pub mod solana;
#[cfg(feature = "web3")]
pub mod tron;

/// Validate `address` for `chain`, returning it trimmed.
///
/// Dispatches to the chain's own module. For Bitcoin this is the
/// **recipient** rule — any well-formed mainnet address; call
/// `btc::validate_sender` directly when validating a sender, since the
/// distinction has no equivalent on the other chains and cannot be expressed
/// through this entry point.
///
/// # Errors
///
/// Whatever the chain's own `validate` returns, plus
/// [`crate::openhuman::web3::wallet::primitives::Error::ChainNotCompiled`] if the shared `web3` feature gate is
/// disabled in this build. That case is a build fact rather than a property of
/// the address: the validation code was not compiled, so there is no answer to
/// give, and silently accepting or rejecting would be a wrong answer dressed
/// up as a real one.
///
/// # Examples
///
/// ```ignore
/// # #[cfg(feature = "web3")] {
/// use crate::openhuman::web3::wallet::primitives::{address, chain::Chain};
///
/// let addr = address::validate(Chain::Solana, "11111111111111111111111111111111")?;
/// assert_eq!(addr, "11111111111111111111111111111111");
/// # }
/// # Ok::<(), crate::openhuman::web3::wallet::primitives::Error>(())
/// ```
// With every chain gate off, only the `ChainNotCompiled` arm survives and
// `address` goes unread. That build is legal (a host may depend on this crate
// purely for `Chain`), so the unused binding is expected rather than a bug.
#[cfg_attr(not(feature = "web3"), allow(unused_variables))]
pub fn validate(chain: Chain, address: &str) -> Result<String> {
    match chain {
        #[cfg(feature = "web3")]
        Chain::Btc => btc::validate(address),
        #[cfg(feature = "web3")]
        Chain::Evm => evm::validate(address),
        #[cfg(feature = "web3")]
        Chain::Solana => solana::validate(address),
        #[cfg(feature = "web3")]
        Chain::Tron => tron::validate(address),
        #[cfg(not(all(feature = "web3", feature = "web3", feature = "web3", feature = "web3")))]
        other => Err(
            crate::openhuman::web3::wallet::primitives::Error::ChainNotCompiled { chain: other },
        ),
    }
}

#[cfg(test)]
mod test;
