//! Error names for a driver reached over a wire, and the mapping both ends use.
//!
//! # Why this is here and not in the transport
//!
//! A driver can be in-process, in a loadable module, or behind a socket. The
//! last two need [`MemoryError`] to survive a round trip through a
//! `(name, message)` pair, because that is all a bus or an HTTP status gives
//! you.
//!
//! The mapping could have lived in whichever adapter needed it first. It lives
//! here instead because there will be more than one adapter, and two copies of
//! a name table drift: the module side starts answering
//! `…Error.PathEscape` while the host side still only recognises
//! `…Error.Invalid`, and the symptom is a security-relevant error
//! silently reclassified as a caller mistake. One table, used by both ends, with
//! [`round_trips_every_variant`](self) pinning it.
//!
//! # One name per variant, not one per outcome class
//!
//! An earlier sketch collapsed these onto three names — "the caller can fix it",
//! "the capability is absent", "something broke" — on the grounds that a host has
//! only those three responses. That is wrong for two reasons.
//!
//! A host does not merely *react* to a driver error; it **is** a
//! [`MemoryProvider`](crate::openhuman::memory::api::provider::MemoryProvider) to everything above it,
//! so it has to hand its own callers a `MemoryError`. Collapsing on the way out
//! and guessing on the way back in would turn a `NotFound` into an `Invalid`,
//! and `get`'s contract says a missing entry is `Ok(None)` while an `Invalid` is
//! a real failure — so the guess is observable.
//!
//! And `PathEscape` is not interchangeable with `Invalid`. It reports a symlink
//! or traversal attempt that left the workspace sandbox, which a host may want
//! to log, report or refuse to retry differently from a malformed argument.
//!
//! # Unrecognised names are backend failures
//!
//! [`from_wire`] maps anything it does not know to [`MemoryError::Other`], never
//! to [`MemoryError::Invalid`]. A driver newer than this build may name an error
//! this table has no variant for, and telling a caller its input was wrong when
//! it was not sends it into a rewrite loop over something already correct.
//!
//! # Messages, and what must not be in them
//!
//! The name is the contract; the message is for a human. Neither may carry a
//! namespace key, an entry's content, a recall query, a credential or an
//! absolute path — memory content is user data, and an error string is not a
//! place for it. `Io` and `Serde` are deliberately flattened into a message
//! here, because reconstructing a live `std::io::Error` or
//! `serde_json::Error` on the far side is not possible and not useful.

use crate::openhuman::memory::api::error::MemoryError;

/// A requested record, source or node was not found.
pub const NOT_FOUND: &str = "ai.tinyhumans.tinymemory.Error.NotFound";
/// Caller-supplied input failed validation.
pub const INVALID: &str = "ai.tinyhumans.tinymemory.Error.Invalid";
/// A configured budget was exceeded.
pub const BUDGET_EXCEEDED: &str = "ai.tinyhumans.tinymemory.Error.BudgetExceeded";
/// A path escaped the workspace sandbox.
pub const PATH_ESCAPE: &str = "ai.tinyhumans.tinymemory.Error.PathEscape";
/// An underlying IO failure.
pub const IO: &str = "ai.tinyhumans.tinymemory.Error.Io";
/// A serialization or deserialization failure.
pub const SERDE: &str = "ai.tinyhumans.tinymemory.Error.Serde";
/// The driver does not implement the named capability family.
pub const UNSUPPORTED: &str = "ai.tinyhumans.tinymemory.Error.Unsupported";
/// An opaque lower-level failure.
pub const OTHER: &str = "ai.tinyhumans.tinymemory.Error.Other";

/// The wire name for `error`.
///
/// Total by construction: the `match` is exhaustive, so a variant added to
/// [`MemoryError`] is a compile error here rather than a silent fallthrough onto
/// [`OTHER`].
#[must_use]
pub fn wire_name(error: &MemoryError) -> &'static str {
    match error {
        MemoryError::NotFound(_) => NOT_FOUND,
        MemoryError::Invalid(_) => INVALID,
        MemoryError::BudgetExceeded(_) => BUDGET_EXCEEDED,
        MemoryError::PathEscape(_) => PATH_ESCAPE,
        MemoryError::Io(_) => IO,
        MemoryError::Serde(_) => SERDE,
        MemoryError::Unsupported { .. } => UNSUPPORTED,
        MemoryError::Other(_) => OTHER,
    }
}

/// The message to send alongside [`wire_name`].
///
/// For most variants this is the inner string rather than the `Display` output,
/// so the receiving side can rebuild the variant without the prefix
/// (`"invalid input: "`, …) being baked into the payload twice.
#[must_use]
pub fn wire_message(error: &MemoryError) -> String {
    match error {
        MemoryError::NotFound(message)
        | MemoryError::Invalid(message)
        | MemoryError::BudgetExceeded(message)
        | MemoryError::PathEscape(message) => message.clone(),
        MemoryError::Unsupported { capability } => capability.clone(),
        // No inner string to lift: these carry a foreign error type, so the
        // rendered form is all there is.
        MemoryError::Io(inner) => inner.to_string(),
        MemoryError::Serde(inner) => inner.to_string(),
        MemoryError::Other(inner) => inner.to_string(),
    }
}

/// Rebuild a [`MemoryError`] from a `(name, message)` pair.
///
/// An unrecognised `name` becomes [`MemoryError::Other`] — see the module docs
/// on why it must not become [`MemoryError::Invalid`].
#[must_use]
pub fn from_wire(name: &str, message: &str) -> MemoryError {
    match name {
        NOT_FOUND => MemoryError::NotFound(message.to_string()),
        INVALID => MemoryError::Invalid(message.to_string()),
        BUDGET_EXCEEDED => MemoryError::BudgetExceeded(message.to_string()),
        PATH_ESCAPE => MemoryError::PathEscape(message.to_string()),
        // `std::io::Error` cannot be reconstructed with its original kind from a
        // string, and inventing one would be worse than being honest that this
        // crossed a wire. The message is preserved.
        IO => MemoryError::Other(anyhow::anyhow!("io error: {message}")),
        SERDE => MemoryError::Other(anyhow::anyhow!("serde error: {message}")),
        UNSUPPORTED => MemoryError::unsupported_raw(message),
        _ => MemoryError::Other(anyhow::anyhow!("{message}")),
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
