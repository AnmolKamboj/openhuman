//! [`MemoryGuard`] — the kernel-owned policy decorator over a bound driver.

use std::sync::Arc;

use crate::openhuman::memory::api::capabilities::{Capabilities, Capability};
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::health::MemoryHealth;
use crate::openhuman::memory::api::provider::{
    MemoryDiff, MemoryDocuments, MemoryEntities, MemoryGoals, MemoryGraph, MemoryIngest,
    MemoryMaintenance, MemoryProvider, MemorySourceSink, MemoryToolMemory, MemoryTree,
};
use async_trait::async_trait;

use super::families::{
    GuardedDiff, GuardedDocuments, GuardedEntities, GuardedGoals, GuardedGraph, GuardedIngest,
    GuardedMaintenance, GuardedSources, GuardedToolMemory, GuardedTree,
};
use super::policy::GuardPolicy;

/// The policy decorator every product caller receives instead of the raw
/// driver (`docs/specs/plan-memory.md` §3.4, `docs/specs/kernel.md` §3.4).
///
/// It implements [`MemoryProvider`], so it is transparent to callers and cannot
/// be "skipped" by a caller that simply keeps using the contract — there is no
/// second, unguarded shape to hold. Its ten `as_*` overrides hand back
/// **guarded** family handles rather than the inner driver's, which is what
/// closes the accessor bypass; see [`super::families`] for why that forces the
/// decorators to be owned fields.
pub struct MemoryGuard {
    inner: Arc<dyn MemoryProvider>,
    policy: Arc<GuardPolicy>,

    // The ten optional families. Each is `Some` **iff** the inner driver
    // provides it, so `provides()` — which the contract's `audit_provider`
    // compares against `capabilities()` — answers identically for the guard and
    // for the driver underneath it.
    ingest: Option<GuardedIngest>,
    documents: Option<GuardedDocuments>,
    tree: Option<GuardedTree>,
    entities: Option<GuardedEntities>,
    graph: Option<GuardedGraph>,
    diff: Option<GuardedDiff>,
    goals: Option<GuardedGoals>,
    tool_memory: Option<GuardedToolMemory>,
    sources: Option<GuardedSources>,
    maintenance: Option<GuardedMaintenance>,
}

impl MemoryGuard {
    /// Wrap `inner` in `policy`.
    ///
    /// Builds all ten decorators up front. That is not an optimisation: the
    /// `as_*` accessors return borrows, so a decorator constructed inside an
    /// accessor could not outlive the call.
    pub fn new(inner: Arc<dyn MemoryProvider>, policy: Arc<GuardPolicy>) -> Self {
        macro_rules! family {
            ($cap:ident, $ty:ident) => {
                inner
                    .provides(Capability::$cap)
                    .then(|| $ty::new(Arc::clone(&inner), Arc::clone(&policy)))
            };
        }
        Self {
            ingest: family!(Ingest, GuardedIngest),
            documents: family!(Documents, GuardedDocuments),
            tree: family!(Tree, GuardedTree),
            entities: family!(Entities, GuardedEntities),
            graph: family!(Graph, GuardedGraph),
            diff: family!(Diff, GuardedDiff),
            goals: family!(Goals, GuardedGoals),
            tool_memory: family!(ToolMemory, GuardedToolMemory),
            sources: family!(Sources, GuardedSources),
            maintenance: family!(Maintenance, GuardedMaintenance),
            inner,
            policy,
        }
    }

    /// The policy this guard enforces.
    pub fn policy(&self) -> &Arc<GuardPolicy> {
        &self.policy
    }

    /// The wrapped driver.
    ///
    /// `pub(crate)` on purpose: handing this out is exactly the bypass the
    /// guard exists to prevent, and the only legitimate use is inside the
    /// memory subsystem itself (identity, health, tests). Do not widen it.
    pub(crate) fn inner(&self) -> &Arc<dyn MemoryProvider> {
        &self.inner
    }
}

#[async_trait]
impl MemoryProvider for MemoryGuard {
    /// The **wrapped driver's** id, not a synthetic `"guard"`. The guard is a
    /// policy layer, not a driver: status output, spans, and audit events all
    /// name the thing that actually stores the bytes.
    fn driver_id(&self) -> &str {
        self.inner.driver_id()
    }

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    async fn health(&self) -> MemoryHealth {
        self.inner.health().await
    }

    async fn shutdown(&self) -> Result<(), MemoryError> {
        self.inner.shutdown().await
    }

    fn as_ingest(&self) -> Option<&dyn MemoryIngest> {
        self.ingest.as_ref().map(|g| g as &dyn MemoryIngest)
    }

    fn as_documents(&self) -> Option<&dyn MemoryDocuments> {
        self.documents.as_ref().map(|g| g as &dyn MemoryDocuments)
    }

    fn as_tree(&self) -> Option<&dyn MemoryTree> {
        self.tree.as_ref().map(|g| g as &dyn MemoryTree)
    }

    fn as_entities(&self) -> Option<&dyn MemoryEntities> {
        self.entities.as_ref().map(|g| g as &dyn MemoryEntities)
    }

    fn as_graph(&self) -> Option<&dyn MemoryGraph> {
        self.graph.as_ref().map(|g| g as &dyn MemoryGraph)
    }

    fn as_diff(&self) -> Option<&dyn MemoryDiff> {
        self.diff.as_ref().map(|g| g as &dyn MemoryDiff)
    }

    fn as_goals(&self) -> Option<&dyn MemoryGoals> {
        self.goals.as_ref().map(|g| g as &dyn MemoryGoals)
    }

    fn as_tool_memory(&self) -> Option<&dyn MemoryToolMemory> {
        self.tool_memory
            .as_ref()
            .map(|g| g as &dyn MemoryToolMemory)
    }

    fn as_sources(&self) -> Option<&dyn MemorySourceSink> {
        self.sources.as_ref().map(|g| g as &dyn MemorySourceSink)
    }

    fn as_maintenance(&self) -> Option<&dyn MemoryMaintenance> {
        self.maintenance
            .as_ref()
            .map(|g| g as &dyn MemoryMaintenance)
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
