//! PII redaction for log output.
//!
//! Per `CLAUDE.md`: "Never log secrets or full PII." Source ids, entity ids and
//! content paths can embed full email addresses, so any log line printing one
//! has to redact it first.
//!
//! # Why this is OpenHuman's and not the memory crate's
//!
//! An identical helper lives at `tinymemory_core::util::redact`, and this host
//! used to call it. That is an engine dependency taken on for the sake of six
//! lines of SHA-256 — and the whole point of the memory-module port is that the
//! host reaches memory over the bus and does not link the engine. A log
//! formatter is not contract vocabulary either, so widening `tinymemory-api` to
//! carry it would have meant adding a dependency to a crate whose manifest
//! documents that it stays dependency-light.
//!
//! The engine keeps its copy for its own log lines. The two are independent by
//! design: neither reads the other's output, and the hash is a grep key for a
//! human debugging with the raw value to hand, not a wire value that has to
//! agree across a boundary.

use sha2::{Digest, Sha256};

/// Redact a string by hashing it to 8 hex chars.
///
/// Stable across runs for the same input, so it can be grepped for in logs when
/// debugging with the raw value available externally.
///
/// Use for source ids, entity ids, content paths and similar PII-bearing
/// strings in log output.
#[must_use]
pub fn redact(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let digest = hasher.finalize();
    format!(
        "{:08x}",
        u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
    )
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redaction_is_stable_for_the_same_input() {
        assert_eq!(redact("alice@example.com"), redact("alice@example.com"));
    }

    #[test]
    fn redaction_distinguishes_different_inputs() {
        assert_ne!(redact("alice@example.com"), redact("bob@example.com"));
    }

    #[test]
    fn redaction_never_echoes_the_input() {
        let out = redact("alice@example.com");
        assert!(!out.contains("alice"), "{out}");
        assert_eq!(out.len(), 8, "expected 8 hex chars, got {out:?}");
        assert!(out.chars().all(|c| c.is_ascii_hexdigit()), "{out}");
    }

    /// Pins byte-parity with the `tinymemory_core::util::redact::redact` this
    /// replaced at five call sites, so old and new log lines for the same id
    /// still match when someone greps across the change.
    ///
    /// The expected value is a **literal**, not a call into the engine helper.
    /// Asserting against the engine would reintroduce exactly the dependency
    /// this module exists to remove — and `memory::direct_engine_refs_tests`
    /// counts inline `#[cfg(test)]` references, so it would also have to be
    /// allowlisted. A pinned constant tests the same property without either.
    #[test]
    fn matches_the_engine_helper_it_replaced() {
        assert_eq!(redact("gmail:alice@example.com"), "c3e9777d");
    }
}
