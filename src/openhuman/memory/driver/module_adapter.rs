//! Bridges a TinyMemory-contract driver into the TinyCortex-contract slot.
//!
//! `memory::binding::build` produces an `Arc<dyn tinycortex_api::provider::
//! MemoryProvider>`, while `modules::memory::ModuleMemoryProvider` implements
//! `tinymemory_api::provider::MemoryProvider`. Those are two distinct traits in
//! two distinct crates. This adapter is what lets the module-backed driver bind
//! before the host's own memory binding finishes migrating onto the TinyMemory
//! contract.
//!
//! # It exists to be deleted
//!
//! `tinymemory-api` was moved *out of* `tinycortex-api`, so the two describe the
//! same values with the same trait shape — `MemoryProvider: MemoryCore +
//! MemoryRecall + MemoryPortability` on both sides, with matching method
//! signatures. When the binding migrates, this file goes away and
//! `ModuleMemoryProvider` is bound directly.
//!
//! # Why serde round-trips and not field-by-field conversion
//!
//! This is the one decision here worth arguing with, so the argument is written
//! down.
//!
//! `tinymemory-tinycortex::convert` does it properly: exhaustive destructuring,
//! total matches, no `..`, so a field added to one contract is a compile error
//! rather than a silently dropped value. That discipline is right for a permanent
//! seam, and it is why the two conversions this adapter needed most
//! (`entry_to_tinycortex`, `namespace_summary_to_tinycortex`) were added there.
//!
//! The remaining types — `ExportPage`, `ExportRecord`, `ImportOutcome`,
//! `SourceScope`, `OwnedRecallOpts` — cross here by serde instead, and that is a
//! deliberate trade with a real downside: a round trip **tolerates** drift where
//! destructuring would catch it. A field added to one side and not the other is
//! dropped at runtime, silently.
//!
//! Three things make that acceptable *for this file specifically*:
//!
//! 1. The contracts are byte-identical today by construction, not by coincidence.
//! 2. [`round_trip_preserves_every_field`] fails if a value stops surviving the
//!    crossing, which is the failure mode being traded away — so it is detected,
//!    just at test time rather than compile time.
//! 3. This code has a scheduled death. Writing ~250 lines of exhaustive
//!    conversion for a seam that is removed once the binding migrates would be
//!    work done twice.
//!
//! Do **not** copy this approach into `convert.rs`, and do not extend this file
//! to carry a permanent conversion. If it stops being temporary, replace the
//! round trips with destructuring first.

use std::sync::Arc;

use async_trait::async_trait;
use tinycortex_api::capabilities::Capabilities as TcCapabilities;
use tinycortex_api::error::MemoryError as TcError;
use tinycortex_api::health::MemoryHealth as TcHealth;
use tinycortex_api::provider::types::{
    ExportPage as TcExportPage, ExportRecord as TcExportRecord, ImportOutcome as TcImportOutcome,
    SourceScope as TcSourceScope,
};
use tinycortex_api::provider::{
    MemoryCore as TcCore, MemoryPortability as TcPortability, MemoryProvider as TcProvider,
    MemoryRecall as TcRecall,
};
use tinycortex_api::recall::OwnedRecallOpts as TcRecallOpts;
use tinycortex_api::types::{
    MemoryCategory as TcCategory, MemoryEntry as TcEntry, MemoryTaint as TcTaint,
    NamespaceSummary as TcSummary,
};
use tinymemory_api::provider::MemoryProvider as TmProvider;
use tinymemory_tinycortex::convert;

/// A TinyMemory driver, presented as a TinyCortex one.
pub struct TinyMemoryContractAdapter {
    inner: Arc<dyn TmProvider>,
}

impl std::fmt::Debug for TinyMemoryContractAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TinyMemoryContractAdapter")
            .field("driver_id", &self.inner.driver_id())
            .finish_non_exhaustive()
    }
}

impl TinyMemoryContractAdapter {
    /// Wrap `inner` so it can bind in a TinyCortex-contract slot.
    #[must_use]
    pub fn new(inner: Arc<dyn TmProvider>) -> Self {
        Self { inner }
    }
}

/// Cross a value between the two contracts through its serde form.
///
/// # Errors
///
/// A message naming the type, when the two contracts have drifted far enough
/// that a value no longer decodes. Never carries the value itself — these are
/// user memory records.
fn cross<A: serde::Serialize, B: serde::de::DeserializeOwned>(
    value: &A,
    what: &'static str,
) -> Result<B, TcError> {
    let json = serde_json::to_value(value)
        .map_err(|error| TcError::Other(anyhow::anyhow!("encode {what}: {error}")))?;
    serde_json::from_value(json)
        .map_err(|error| TcError::Other(anyhow::anyhow!("decode {what}: {error}")))
}

/// Map a TinyMemory error onto the TinyCortex one, preserving the variant.
///
/// Variant-preserving rather than collapsing onto `Other`, because a host
/// re-raises these to its own callers and `NotFound` versus `Invalid` is
/// observable — `get`'s contract makes a miss `Ok(None)` while an `Invalid` is a
/// real failure. `Io` and `Serde` degrade to `Other`: neither foreign error can
/// be rebuilt, and inventing an `io::ErrorKind` would be worse than being honest
/// that this crossed a boundary.
fn error_to_tinycortex(error: tinymemory_api::error::MemoryError) -> TcError {
    use tinymemory_api::error::MemoryError as Tm;
    match error {
        Tm::NotFound(message) => TcError::NotFound(message),
        Tm::Invalid(message) => TcError::Invalid(message),
        Tm::BudgetExceeded(message) => TcError::BudgetExceeded(message),
        Tm::PathEscape(message) => TcError::PathEscape(message),
        Tm::Unsupported { capability } => TcError::unsupported_raw(capability),
        Tm::Io(inner) => TcError::Other(anyhow::anyhow!("io error: {inner}")),
        Tm::Serde(inner) => TcError::Other(anyhow::anyhow!("serde error: {inner}")),
        Tm::Other(inner) => TcError::Other(inner),
    }
}

#[async_trait]
impl TcProvider for TinyMemoryContractAdapter {
    fn driver_id(&self) -> &str {
        self.inner.driver_id()
    }

    fn capabilities(&self) -> TcCapabilities {
        // Both sides serialize a capability set as a JSON array of family names,
        // so this crossing is over stable wire strings rather than bit layouts.
        // An unreadable set is reported as empty, which is the fail-closed
        // direction: the kernel filters its RPC surface from this, so an
        // overstated set would register methods that answer errors.
        cross::<_, TcCapabilities>(&self.inner.capabilities(), "capabilities")
            .unwrap_or_else(|_| TcCapabilities::empty())
    }

    async fn health(&self) -> TcHealth {
        let health = self.inner.health().await;
        cross::<_, TcHealth>(&health, "health")
            .unwrap_or_else(|error| TcHealth::down(error.to_string()))
    }

    async fn shutdown(&self) -> Result<(), TcError> {
        self.inner.shutdown().await.map_err(error_to_tinycortex)
    }
}

#[async_trait]
impl TcCore for TinyMemoryContractAdapter {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: TcCategory,
        session_id: Option<&str>,
        taint: TcTaint,
    ) -> Result<(), TcError> {
        // Category and taint use the audited destructuring conversions, not a
        // serde round trip. Taint especially: mapping it wrongly would let
        // externally-sourced content be treated as internal-trust content, which
        // is the one thing the policy guard exists to prevent.
        self.inner
            .store(
                namespace,
                key,
                content,
                convert::category_to_tinymemory(category),
                session_id,
                convert::taint_to_tinymemory(taint),
            )
            .await
            .map_err(error_to_tinycortex)
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<TcEntry>, TcError> {
        Ok(self
            .inner
            .get(namespace, key)
            .await
            .map_err(error_to_tinycortex)?
            .map(convert::entry_to_tinycortex))
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, TcError> {
        self.inner
            .forget(namespace, key)
            .await
            .map_err(error_to_tinycortex)
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&TcCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<TcEntry>, TcError> {
        let category = category.cloned().map(convert::category_to_tinymemory);
        Ok(self
            .inner
            .list(namespace, category.as_ref(), session_id)
            .await
            .map_err(error_to_tinycortex)?
            .into_iter()
            .map(convert::entry_to_tinycortex)
            .collect())
    }

    async fn namespaces(&self) -> Result<Vec<TcSummary>, TcError> {
        Ok(self
            .inner
            .namespaces()
            .await
            .map_err(error_to_tinycortex)?
            .into_iter()
            .map(convert::namespace_summary_to_tinycortex)
            .collect())
    }
}

#[async_trait]
impl TcRecall for TinyMemoryContractAdapter {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &TcRecallOpts,
        scope: Option<&TcSourceScope>,
    ) -> Result<Vec<TcEntry>, TcError> {
        let opts = cross::<_, tinymemory_api::recall::OwnedRecallOpts>(opts, "recall options")?;
        // `scope` is a query predicate the driver applies internally, so it has
        // to cross intact rather than being applied to the result here — see the
        // contract's note on why filtering afterwards is wrong.
        let scope = match scope {
            Some(scope) => Some(cross::<_, tinymemory_api::provider::types::SourceScope>(
                scope,
                "source scope",
            )?),
            None => None,
        };
        Ok(self
            .inner
            .recall(query, limit, &opts, scope.as_ref())
            .await
            .map_err(error_to_tinycortex)?
            .into_iter()
            .map(convert::entry_to_tinycortex)
            .collect())
    }
}

#[async_trait]
impl TcPortability for TinyMemoryContractAdapter {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<TcExportPage, TcError> {
        let page = self
            .inner
            .export_page(cursor, limit)
            .await
            .map_err(error_to_tinycortex)?;
        cross(&page, "export page")
    }

    async fn import_records(
        &self,
        records: Vec<TcExportRecord>,
    ) -> Result<TcImportOutcome, TcError> {
        let records: Vec<tinymemory_api::provider::types::ExportRecord> =
            cross(&records, "export records")?;
        let outcome = self
            .inner
            .import_records(records)
            .await
            .map_err(error_to_tinycortex)?;
        cross(&outcome, "import outcome")
    }
}

#[cfg(test)]
#[path = "module_adapter_tests.rs"]
mod tests;
