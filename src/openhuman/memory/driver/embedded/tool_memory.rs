//! [`MemoryToolMemory`] for the embedded driver — per-tool learned rules.
//!
//! Backed by the engine's own [`ToolMemoryStore`], built over this driver's
//! `Arc<dyn Memory>` through the host wrapper
//! [`tool_memory_store`](crate::openhuman::memory::tool_memory::tool_memory_store).
//! The store is a single `Arc` behind a `#[derive(Clone)]` struct, so
//! constructing one per call costs an `Arc` clone and keeps the driver's lazy
//! client rule intact — no eager handle, no second `OnceCell`.
//!
//! ## Not through `memory::ops::tool_memory`
//!
//! Those are the RPC handlers, and their `open_store()` resolves the
//! **process-global** active memory client. This driver holds a
//! workspace-scoped client on purpose: routing through the global slot would
//! let a driver bound to workspace B write into workspace A, which is exactly
//! the property the workspace-keyed binding map buys.
//!
//! ## Taint
//!
//! `ToolMemoryStore::put_rule` writes through `Memory::store`, and that is
//! correct here. The taint trap this milestone warns about is the engine's
//! *default* `store` impl, which drops the taint argument;
//! `UnifiedMemory::store` is an explicit impl forwarding to
//! `store_with_taint(..., MemoryTaint::Internal)`. Tool rules are host-authored,
//! so `Internal` is the right provenance — and no method in this family takes a
//! taint argument to lose in the first place.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::MemoryToolMemory;
use tinycortex_api::tool_memory::ToolMemoryRule;

use super::{host_error, EmbeddedMemoryProvider};
use crate::openhuman::memory::tool_memory::{tool_memory_store, ToolMemoryStore};

/// The two rejections `ToolMemoryStore::put_rule` performs before touching
/// storage. Matched by value so a genuine backend failure is never mislabelled
/// as caller error.
const PUT_RULE_REJECTIONS: [&str; 2] = ["tool_name is required", "rule body is required"];

/// Classifies a `put_rule` failure: a validated rejection is
/// [`MemoryError::Invalid`], everything else is a backend failure.
fn classify_put_rule(error: String) -> MemoryError {
    if PUT_RULE_REJECTIONS.contains(&error.as_str()) {
        return MemoryError::Invalid(error);
    }
    host_error("put_tool_rule", error)
}

impl EmbeddedMemoryProvider {
    async fn tool_memory(&self) -> Result<ToolMemoryStore, MemoryError> {
        Ok(tool_memory_store(self.memory().await?))
    }
}

#[async_trait]
impl MemoryToolMemory for EmbeddedMemoryProvider {
    async fn tool_rules(&self, tool_name: &str) -> Result<Vec<ToolMemoryRule>, MemoryError> {
        log::debug!("[memory:driver:embedded] tool_rules tool={tool_name}");
        self.tool_memory()
            .await?
            .list_rules(tool_name)
            .await
            .map_err(|error| host_error("tool_rules", error))
    }

    async fn put_tool_rule(&self, rule: ToolMemoryRule) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] put_tool_rule tool={} priority={:?}",
            rule.tool_name,
            rule.priority
        );
        // The stored copy (with `created_at` preserved and `updated_at`
        // refreshed) is discarded: the contract returns unit, and re-reading it
        // is `tool_rules`' job.
        self.tool_memory()
            .await?
            .put_rule(rule)
            .await
            .map(|_| ())
            .map_err(classify_put_rule)
    }

    async fn delete_tool_rule(&self, tool_name: &str, rule_id: &str) -> Result<bool, MemoryError> {
        log::debug!("[memory:driver:embedded] delete_tool_rule tool={tool_name} rule={rule_id}");
        self.tool_memory()
            .await?
            .delete_rule(tool_name, rule_id)
            .await
            .map_err(|error| host_error("delete_tool_rule", error))
    }
}

#[cfg(test)]
#[path = "tool_memory_tests.rs"]
mod tests;
