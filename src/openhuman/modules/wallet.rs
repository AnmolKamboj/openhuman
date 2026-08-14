//! Calling the `tinywallet` module.
//!
//! The host half of `ai.tinyhumans.tinywallet.Wallet`. Two ways to get a signed
//! transaction, and which one a chain uses is a property of that chain.
//!
//! # The confidential flow — preferred, and what BTC, EVM and Tron use
//!
//! [`sign_transaction_in_module`] sends the recovery phrase itself, once, and
//! the module derives, encodes, signs and assembles. No private key is ever
//! reassembled in this process.
//!
//! ```text
//!   host  ==SignTransaction{phrase, path, fields}==>  module   (confidential)
//!   host  <=={raw transaction, txid}================  module
//! ```
//!
//! The phrase only goes to a recipient tinybus has attested, and
//! [`attested_proxy`] additionally insists the attested digest is one
//! `registry.rs` pinned — see there for why both checks are worth having.
//!
//! # The split flow — still here, still correct for some hosts
//!
//! [`sign_transaction`] keeps the key in this process: the module returns
//! digests, this process signs them, the module reassembles.
//!
//! ```text
//!   host  --BuildUnsigned{fields, public key}-->  module
//!   host  <--[digests to sign]-------------------  module
//!   host    (signs locally, with a key the module never sees)
//!   host  --AttachSignature{fields, signatures}->  module
//!   host  <--{raw transaction, txid}-------------  module
//! ```
//!
//! It is not deprecated. It is the only option for a backend reached across a
//! transport, where the bus cannot say what is on the other end, and it is what
//! Solana still uses here — Solana hand-builds SPL messages that
//! `TransactionSpec::Solana` does not model, so there is nothing to send.
//!
//! # What the confidential flow does and does not buy
//!
//! A loaded module shares this address space and could read the phrase out of
//! process memory whichever flow is used, so neither is a hard isolation
//! boundary and neither is claimed as one.
//!
//! What changes is that this binary no longer performs derivation or signing at
//! all — the key exists only inside the module, for the duration of one call,
//! rather than being assembled here and held across two round trips. The bus
//! also refuses to carry the phrase to anything that is not an allowlisted,
//! hash-verified module, which the split flow could not express.
//!
//! # The fields are sent twice in the split flow, deliberately
//!
//! `AttachSignature` re-sends everything `BuildUnsigned` was given rather than a
//! handle to something the module remembered. That is what lets the module hold
//! no state between the calls — no store, no bound on it, no expiry for a host
//! that never comes back. Building is deterministic, so the module rebuilds the
//! transaction the digests were computed over. The confidential flow needs none
//! of this: it is one call.
//!
//! # Two signing schemes, and the difference matters
//!
//! A `Secp256k1Prehash` payload is **already hashed** and must be signed with a
//! prehash entry point; hashing it again produces a valid signature over the
//! wrong thing. An `Ed25519` payload is the whole message and must **not** be
//! pre-hashed, because ed25519 hashes internally. The module tags every payload
//! with which it is, and [`sign_payload`] dispatches on the tag rather than on
//! the chain — so a chain that changes scheme cannot silently sign wrongly.

use tinywallet::wire::{
    AttachRequest, DerivedAccount, ExportedKey, PublicKey, Scheme, SecretMaterial,
    SignMessageRequest, SignRequest, Signature, SignedTransaction, SigningPayload, SigningRequest,
    TransactionSpec, UnsignedTransaction,
};

use super::{host, ops, registry};
use crate::openhuman::config::Config;

/// Registry id of the module these calls go to.
const MODULE_ID: &str = "tinywallet";

/// Why a wallet call did not produce a signed transaction.
///
/// Three variants because the wallet tools map them onto three different
/// user-facing outcomes: something the caller can correct, something it cannot,
/// and a capability that is not present on this host at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletCallError {
    /// The module is not loaded and cannot be: unsupported host, downloads off,
    /// disabled in config, or a load that already failed in this process.
    Unavailable(String),
    /// The request was rejected. The caller can act on this.
    InvalidInput(String),
    /// Building, signing, or assembly failed.
    Failed(String),
}

impl std::fmt::Display for WalletCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::InvalidInput(message) | Self::Failed(message) => {
                f.write_str(message)
            }
        }
    }
}

/// Build, sign, and assemble a transaction for `chain`.
///
/// `secret` is the raw signing key and never leaves this process. `public_key`
/// is its compressed SEC1 form for secp256k1 chains, or the 32-byte public key
/// for Solana — the module needs it to check that the key controls the sender
/// before it will build anything.
///
/// # Errors
///
/// [`WalletCallError`] describing whether the request, the module, or the
/// signing was at fault.
pub async fn sign_transaction(
    config: &Config,
    transaction: &TransactionSpec,
    secret: &[u8],
    public_key: &[u8],
) -> Result<SignedTransaction, WalletCallError> {
    let (runtime, record) = ready(config).await?;
    let proxy = proxy(runtime, record)?;
    let public_key = PublicKey {
        key_hex: hex(public_key),
    };

    let chain = transaction.chain();
    log::debug!("[modules:wallet] build_unsigned chain={chain:?} module={MODULE_ID}");
    let unsigned: UnsignedTransaction = proxy
        .call(
            "BuildUnsigned",
            (SigningRequest {
                transaction: transaction.clone(),
                public_key: public_key.clone(),
            },),
        )
        .await
        .map_err(|error| classify(&error))?;

    // Signed here. Bitcoin returns one payload per selected input and the
    // signatures must come back in the same order, so this maps rather than
    // handling a single value.
    let signatures = unsigned
        .payloads
        .iter()
        .map(|payload| sign_payload(payload, secret))
        .collect::<Result<Vec<_>, _>>()?;

    log::debug!(
        "[modules:wallet] attach_signature chain={chain:?} signatures={}",
        signatures.len()
    );
    proxy
        .call(
            "AttachSignature",
            (AttachRequest {
                transaction: transaction.clone(),
                public_key,
                signatures,
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Derive, build, sign and assemble entirely inside the module.
///
/// The counterpart to [`sign_transaction`], and the one to prefer: the phrase
/// crosses the bus once and no key material is ever reassembled here.
///
/// # Errors
///
/// [`WalletCallError`]. `Unavailable` if the module is not an attested
/// recipient — see [`attested_proxy`], which refuses to send the phrase at all
/// in that case.
pub async fn sign_transaction_in_module(
    config: &Config,
    transaction: &TransactionSpec,
    secret: &SecretMaterial,
) -> Result<SignedTransaction, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] sign_transaction chain={:?} module={MODULE_ID} (confidential)",
        transaction.chain()
    );
    proxy
        .call_confidential(
            "SignTransaction",
            (SignRequest {
                secret: secret.clone(),
                transaction: transaction.clone(),
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Ask the module for an address and public key.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn derive_account(
    config: &Config,
    secret: &SecretMaterial,
) -> Result<DerivedAccount, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] derive_account chain={:?} module={MODULE_ID} (confidential)",
        secret.chain
    );
    proxy
        .call_confidential("DeriveAccount", (secret.clone(),))
        .await
        .map_err(|error| classify(&error))
}

/// Sign opaque bytes with the key derived from `secret`.
///
/// Blind: the module cannot check what the bytes mean, so
/// [`sign_transaction_in_module`] is the right call wherever the request can be
/// expressed as a `TransactionSpec`. This exists for the two encodings the wire
/// contract does not model — Solana SPL transfers and x402 payments — where the
/// alternative is not a verified signature but deriving the key in this process
/// and signing here, which is what these callers used to do.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn sign_message(
    config: &Config,
    secret: &SecretMaterial,
    message: &[u8],
    scheme: Scheme,
) -> Result<Signature, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] sign_message chain={:?} bytes={} module={MODULE_ID} (confidential)",
        secret.chain,
        message.len()
    );
    proxy
        .call_confidential(
            "SignMessage",
            (SignMessageRequest {
                secret: secret.clone(),
                message_hex: hex(message),
                scheme,
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Ask the module for the raw derived key.
///
/// The one call that brings key material back into this process. It exists for
/// signers this host must drive itself — tinyplace's `LocalSigner`, which takes
/// a seed and cannot be handed a transaction instead. Anything that can use
/// [`sign_transaction_in_module`] must.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn export_key(
    config: &Config,
    secret: &SecretMaterial,
) -> Result<ExportedKey, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] export_key chain={:?} module={MODULE_ID} (confidential)",
        secret.chain
    );
    proxy
        .call_confidential("ExportKey", (secret.clone(),))
        .await
        .map_err(|error| classify(&error))
}

/// A proxy that has proved it is the artifact this build pinned.
///
/// # Why check here when the broker already refuses
///
/// The broker will not route a confidential call to an unattested recipient, so
/// omitting this would still be safe against the case it covers. Two reasons to
/// check anyway.
///
/// The first is the error. A broker refusal arrives after the request has been
/// serialized, which means a frame containing the recovery phrase was built and
/// handed to the bus before anything said no. Checking first means the phrase is
/// never put into a buffer on a call that was always going to fail.
///
/// The second is that this compares the digest against **this build's own
/// table**, which the broker cannot do — it only knows the host vouched for
/// something. Here we can insist it vouched for one of the artifacts
/// `registry.rs` names. That closes the gap where a host is somehow induced to
/// attest a different artifact for this bus name.
///
/// Both are belt-and-braces over a check that already exists. That is the right
/// posture for the one code path that hands over a recovery phrase.
async fn attested_proxy(config: &Config) -> Result<tinybus::Proxy, WalletCallError> {
    let (runtime, record) = ready(config).await?;
    let proxy = proxy(runtime, record)?;

    let attestation = proxy
        .attestation()
        .await
        .map_err(|error| WalletCallError::Failed(error.to_string()))?
        .ok_or_else(|| {
            WalletCallError::Unavailable(
                "the wallet module is loaded but not an attested recipient, so it cannot be sent \
                 key material. This host is running a build whose module loader does not record \
                 an attestation for pinned releases; upgrade it rather than working around this."
                    .to_string(),
            )
        })?;

    if attestation.name.as_str() != record.bus_name {
        return Err(WalletCallError::Failed(format!(
            "the wallet module's attestation names '{}' rather than '{}'",
            attestation.name.as_str(),
            record.bus_name
        )));
    }

    if !digest_is_pinned(record, &attestation.sha256) {
        return Err(WalletCallError::Failed(
            "the loaded wallet module is not one of the artifacts this build pinned, so it will \
             not be sent key material"
                .to_string(),
        ));
    }

    Ok(proxy)
}

/// Whether `sha256` is one of the artifacts this build pinned for `record`.
///
/// Case-insensitive because the release manifest and the pinned table are
/// written by different hands and both are hex of the same bytes. Nothing
/// compared here is a secret, so a constant-time comparison would buy nothing.
///
/// Every asset is checked rather than just the one for this platform: which
/// artifact loaded is not recorded, and a host that fell through to an older
/// build after an admission failure is running a pinned artifact either way.
fn digest_is_pinned(record: &super::ModuleRecord, sha256: &str) -> bool {
    record
        .assets
        .iter()
        .any(|asset| asset.sha256.eq_ignore_ascii_case(sha256))
}

/// Sign one payload with whichever scheme it declares.
///
/// Dispatches on the payload's own tag, never on the chain: the module is the
/// authority on what it needs signed and how, and a host that decided for itself
/// would sign wrongly the moment the two disagreed.
#[allow(unreachable_patterns)]
fn sign_payload(payload: &SigningPayload, secret: &[u8]) -> Result<Signature, WalletCallError> {
    let bytes = unhex(&payload.bytes_hex)?;
    match payload.scheme {
        Scheme::Secp256k1Prehash => {
            let digest: [u8; 32] = bytes.try_into().map_err(|_| {
                WalletCallError::Failed(
                    "the module asked for a prehash signature over something that is not 32 bytes"
                        .to_string(),
                )
            })?;
            sign_secp256k1_prehash(&digest, secret)
        }
        Scheme::Ed25519 => sign_ed25519(&bytes, secret),
        // `Scheme` is `#[non_exhaustive]`. A module newer than this build may
        // name a scheme we cannot perform, and guessing would produce a
        // signature over the wrong preimage.
        _ => Err(WalletCallError::Failed(
            "the module asked for a signing scheme this build does not implement".to_string(),
        )),
    }
}

/// secp256k1 ECDSA over an already-computed digest, with the recovery id.
///
/// `k256` rather than the `bitcoin` crate's `secp256k1`: this is the entire
/// reason the host can drop that dependency and its native C build, and `k256`
/// is already in the tree beneath `coins-bip32`, which derives the key being
/// used here. It produces low-`s` signatures by default, which Bitcoin requires
/// as relay policy (BIP-146) and Ethereum as consensus (EIP-2).
fn sign_secp256k1_prehash(digest: &[u8; 32], secret: &[u8]) -> Result<Signature, WalletCallError> {
    use k256::ecdsa::SigningKey;

    let key = SigningKey::from_slice(secret)
        .map_err(|_| WalletCallError::Failed("not a valid secp256k1 secret key".to_string()))?;
    let (signature, recovery_id) = key
        .sign_prehash_recoverable(digest)
        .map_err(|_| WalletCallError::Failed("secp256k1 signing failed".to_string()))?;

    Ok(Signature::Secp256k1 {
        rs_hex: hex(&signature.to_bytes()),
        recovery_id: recovery_id.to_byte(),
    })
}

/// ed25519 over the whole message.
fn sign_ed25519(message: &[u8], secret: &[u8]) -> Result<Signature, WalletCallError> {
    use ed25519_dalek::{Signer as _, SigningKey};

    let key: [u8; 32] = secret.try_into().map_err(|_| {
        WalletCallError::Failed("an ed25519 secret key must be 32 bytes".to_string())
    })?;
    let signature = SigningKey::from_bytes(&key).sign(message);
    Ok(Signature::Ed25519 {
        signature_hex: hex(&signature.to_bytes()),
    })
}

/// Load the wallet module if it is not already serving.
///
/// Callers do not have to invoke this — [`sign_transaction`] does — but a caller
/// that wraps its work in a deadline should, *outside* that deadline. A first
/// use may download and verify an artifact, and charging that against a
/// transaction timeout means the first transfer a user ever makes is the one
/// that fails.
///
/// # Errors
///
/// [`WalletCallError::Unavailable`].
pub async fn ensure_ready(config: &Config) -> Result<(), WalletCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(WalletCallError::Unavailable)
}

/// Ensure the module is serving and hand back what a call needs.
async fn ready(
    config: &Config,
) -> Result<(&'static host::ModuleRuntime, &'static super::ModuleRecord), WalletCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(WalletCallError::Unavailable)?;
    let record = registry::find(MODULE_ID)
        .ok_or_else(|| WalletCallError::Unavailable(format!("unknown module '{MODULE_ID}'")))?;
    let runtime = host::runtime()
        .await
        .map_err(|_| WalletCallError::Unavailable("the module bus is not running".to_string()))?;
    Ok((runtime, record))
}

/// A proxy for the module's object.
fn proxy(
    runtime: &'static host::ModuleRuntime,
    record: &super::ModuleRecord,
) -> Result<tinybus::Proxy, WalletCallError> {
    runtime
        .proxy(record.bus_name, record.object_path)
        .map_err(|error| WalletCallError::Failed(error.to_string()))
}

/// Map a bus failure onto the shape a caller can act on.
///
/// The wire name is the contract; the message is for a human. An unrecognised
/// name is `Failed` rather than `InvalidInput`, because telling a caller its
/// input was wrong when it was not points at the wrong fix.
fn classify(error: &tinybus::Error) -> WalletCallError {
    let message = error.to_string();
    match error.wire_name() {
        "ai.tinyhumans.tinywallet.Error.InvalidInput" => WalletCallError::InvalidInput(message),
        "ai.tinyhumans.tinywallet.Error.UnsupportedChain" => WalletCallError::Unavailable(message),
        // The module is loaded but not answering: refused, faulted, or gone.
        name if name.contains("ModuleUnavailable") => WalletCallError::Unavailable(message),
        _ => WalletCallError::Failed(message),
    }
}

/// Lowercase hex, unprefixed — the form every field in the wire contract uses.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Decode lowercase hex from the module.
fn unhex(value: &str) -> Result<Vec<u8>, WalletCallError> {
    if !value.len().is_multiple_of(2) {
        return Err(WalletCallError::Failed(
            "the module returned a payload with an odd number of hex characters".to_string(),
        ));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| {
                    WalletCallError::Failed("the module returned a non-hex payload".to_string())
                })
        })
        .collect()
}

#[cfg(test)]
#[path = "wallet_tests.rs"]
mod tests;
