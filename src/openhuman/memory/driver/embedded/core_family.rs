//! [`MemoryCore`] for the embedded driver — store / get / forget / list /
//! namespaces.
//!
//! Four of the five are a straight delegation to the engine's [`Memory`] trait.
//! Two things are *not* straight, and both are load-bearing:
//!
//! 1. **`store` maps onto [`Memory::store_with_taint`], never [`Memory::store`].**
//!    The contract has a single `store` that always carries a
//!    [`MemoryTaint`](tinycortex_api::types::MemoryTaint), because provenance is
//!    stamped by the host policy guard *before* the call. `Memory::store` hard-codes
//!    `MemoryTaint::Internal`, so routing through it would launder
//!    externally-sourced content into internal-trust content — the single
//!    failure mode the guard exists to prevent. (`Memory::store_with_taint`'s
//!    *trait default* also silently drops the taint; `UnifiedMemory` overrides
//!    it, which is why this delegation is correct and the other is not.)
//!
//! 2. **`list(None, ..)` spans every namespace.** The contract says all-`None`
//!    lists everything the driver holds; the engine's `list` normalises a
//!    `None` namespace to `GLOBAL_NAMESPACE`, so a naive delegation would
//!    silently return one namespace and call it "everything". The driver
//!    composes `namespace_summaries()` with a per-namespace `list` instead —
//!    two existing calls, no new query logic.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::MemoryCore;
use tinycortex_api::types::{MemoryCategory, MemoryEntry, MemoryTaint, NamespaceSummary};

use super::{engine_error, EmbeddedMemoryProvider};

#[async_trait]
impl MemoryCore for EmbeddedMemoryProvider {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        taint: MemoryTaint,
    ) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] store namespace={namespace} key_len={} content_len={} \
             category={category} session={} taint={}",
            key.len(),
            content.len(),
            session_id.unwrap_or("-"),
            taint.as_db_str()
        );
        // `store_with_taint`, never `store` — see the module docs.
        self.memory()
            .await?
            .store_with_taint(namespace, key, content, category, session_id, taint)
            .await
            .map_err(engine_error)
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        self.memory()
            .await?
            .get(namespace, key)
            .await
            .map_err(engine_error)
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] forget namespace={namespace} key_len={}",
            key.len()
        );
        self.memory()
            .await?
            .forget(namespace, key)
            .await
            .map_err(engine_error)
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let memory = self.memory().await?;

        if let Some(namespace) = namespace {
            return memory
                .list(Some(namespace), category, session_id)
                .await
                .map_err(engine_error);
        }

        // All-`None` must mean "everything this driver holds" (contract), which
        // the engine's namespace normalisation would otherwise narrow to the
        // global namespace alone.
        let summaries = memory.namespace_summaries().await.map_err(engine_error)?;
        log::debug!(
            "[memory:driver:embedded] list spanning {} namespace(s)",
            summaries.len()
        );
        let mut entries = Vec::new();
        for summary in summaries {
            let mut page = memory
                .list(Some(&summary.namespace), category, session_id)
                .await
                .map_err(engine_error)?;
            entries.append(&mut page);
        }
        Ok(entries)
    }

    async fn namespaces(&self) -> Result<Vec<NamespaceSummary>, MemoryError> {
        // The contract's `namespaces` is the engine's `namespace_summaries`;
        // the return type is identical, only the name differs.
        self.memory()
            .await?
            .namespace_summaries()
            .await
            .map_err(engine_error)
    }
}

#[cfg(test)]
#[path = "core_family_tests.rs"]
mod tests;
