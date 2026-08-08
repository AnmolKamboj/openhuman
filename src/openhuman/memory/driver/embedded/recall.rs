//! [`MemoryRecall`] for the embedded driver.
//!
//! The delegation itself is trivial — [`OwnedRecallOpts`] borrows into the
//! engine's `RecallOpts` for free, and `UnifiedMemory::recall` already returns
//! ranked, `min_score`-filtered, category-filtered, cross-session-merged
//! results. Nothing is re-ranked here.
//!
//! ## `scope` is refused, not ignored
//!
//! The contract's `scope` is a **query predicate the driver must apply
//! internally**: applying it after the fact would let `limit` be consumed by
//! rows the caller may not see, and an empty scope denies all source-attributed
//! content rather than waving it through.
//!
//! The embedded recall path has no such predicate today. `Memory::recall` →
//! `query_namespace_ranked_excluding_session` consults nothing resembling
//! [`SourceScope`]; the host's ambient equivalent
//! ([`crate::openhuman::memory::source_scope`]) is read only by the
//! tree-retrieval and chunk-search layers, which land with the
//! [`Tree`](tinycortex_api::capabilities::Capability::Tree) family.
//!
//! So a `Some(scope)` here has exactly three possible treatments, and two are
//! wrong:
//!
//! - *silently ignore it* — a scoped query answered in full. That is a leak,
//!   and it is invisible.
//! - *post-filter the rows* — the failure mode the contract's own docs name.
//! - *refuse* — what this driver does, until the predicate exists.
//!
//! [`MemoryError::Invalid`] is the right variant, not `Unsupported`:
//! `Unsupported` names a whole capability *family*, and recall is advertised.
//! Nothing calls the driver's `recall` yet (the RPC surface still goes straight
//! to the engine), so the refusal costs no live behaviour — it just cannot be
//! mistaken for working.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::SourceScope;
use tinycortex_api::provider::MemoryRecall;
use tinycortex_api::recall::OwnedRecallOpts;
use tinycortex_api::types::{MemoryEntry, RecallOpts};

use super::{engine_error, EmbeddedMemoryProvider};

/// Refusal message for a scoped recall. A constant so the test asserts the same
/// string the caller sees.
pub(super) const SCOPE_UNAPPLIED: &str =
    "source scope is not applied by the embedded recall path yet: the scope predicate lives in \
     the tree-retrieval layer and lands with the tree capability family";

#[async_trait]
impl MemoryRecall for EmbeddedMemoryProvider {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &OwnedRecallOpts,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        if scope.is_some() {
            log::warn!("[memory:driver:embedded] recall refused: {SCOPE_UNAPPLIED}");
            return Err(MemoryError::Invalid(SCOPE_UNAPPLIED.to_string()));
        }

        log::debug!(
            "[memory:driver:embedded] recall query_len={} limit={limit} namespace={} \
             cross_session={}",
            query.len(),
            opts.namespace.as_deref().unwrap_or("-"),
            opts.cross_session
        );

        // Zero-copy for the string filters; exhaustively destructured inside
        // the contract crate so a new filter cannot be dropped silently.
        let borrowed = RecallOpts::from(opts);
        self.memory()
            .await?
            .recall(query, limit, borrowed)
            .await
            .map_err(engine_error)
    }
}

#[cfg(test)]
#[path = "recall_tests.rs"]
mod tests;
