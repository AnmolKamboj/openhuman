//! The ten optional-family decorators — the load-bearing half of the guard.
//!
//! ## Why these exist at all
//!
//! [`MemoryProvider::as_tree`] and its nine siblings return a **borrow** of a
//! family trait object. If [`MemoryGuard`]'s override simply forwarded
//! `self.inner.as_tree()`, every caller that reached memory through a family
//! accessor would hold a raw, unguarded driver handle — and the guard's whole
//! reason to exist ("the only handle product code receives") would be
//! bypassable by one method call. Nine of the thirteen families are *only*
//! reachable that way.
//!
//! So each family gets its own decorator, and the accessor hands back a borrow
//! of that. Because the accessor returns a reference, the decorators cannot be
//! constructed on demand inside it — a reference to a temporary does not
//! outlive the call — so they are **fields on the guard, built once at
//! construction**. That is also what makes their presence mirror the inner
//! driver's exactly: a field exists iff `inner.provides(...)` said so, which is
//! what keeps `audit_provider` happy.
//!
//! ## Why each decorator holds the provider, not the family
//!
//! A `GuardedTree { inner: &dyn MemoryTree }` borrowed out of an
//! `Arc<dyn MemoryProvider>` the same struct owns is self-referential, and Rust
//! has no way to express that without unsafe pinning. Holding
//! `Arc<dyn MemoryProvider>` and re-deriving the family per call sidesteps it
//! entirely, at the cost of one `Option` unwrap that is structurally
//! unreachable — see [`family`](GuardedTree::family).
//!
//! [`MemoryGuard`]: super::MemoryGuard

use std::sync::Arc;

use crate::openhuman::memory::api::capabilities::Capability;
use crate::openhuman::memory::api::chunks::Chunk;
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::goals::GoalsDoc;
use crate::openhuman::memory::api::provider::types::{
    DiffReport, EntityHit, IngestItem, IngestOutcome, MaintenanceReport, SnapshotRef, SourceItem,
    SourceScope,
};
use crate::openhuman::memory::api::provider::{
    MemoryDiff, MemoryDocuments, MemoryEntities, MemoryGoals, MemoryGraph, MemoryIngest,
    MemoryMaintenance, MemoryProvider, MemorySourceSink, MemoryToolMemory, MemoryTree,
};
use crate::openhuman::memory::api::tool_memory::ToolMemoryRule;
use crate::openhuman::memory::api::tree::{IngestRequest, QueryResult, TreeStatus};
use crate::openhuman::memory::api::types::{
    GraphRelationRecord, MemoryKvRecord, MemoryTaint, NamespaceDocumentInput,
    NamespaceRetrievalContext, StoredMemoryDocument,
};
use async_trait::async_trait;

use super::audit::{trace_allowed, NO_NAMESPACE};
use super::policy::GuardPolicy;

/// Declares one decorator: the two shared fields, a constructor, and the
/// `family()` re-derivation.
macro_rules! decorator {
    ($(#[$meta:meta])* $name:ident, $fam:ty, $accessor:ident, $cap:ident) => {
        $(#[$meta])*
        pub struct $name {
            inner: Arc<dyn MemoryProvider>,
            policy: Arc<GuardPolicy>,
        }

        impl $name {
            pub(super) fn new(inner: Arc<dyn MemoryProvider>, policy: Arc<GuardPolicy>) -> Self {
                Self { inner, policy }
            }

            /// The underlying family handle.
            ///
            /// The `Err` arm is **structurally unreachable**: `MemoryGuard::new`
            /// only builds this decorator when the inner provider answered
            /// `provides(Capability::$cap)`, and the contract documents the
            /// capability set as fixed at bind time. It is written as a real
            /// error rather than `.expect(...)` because a panic inside a memory
            /// call is a strictly worse failure than an `Unsupported` a caller
            /// can already handle.
            fn family(&self) -> Result<&$fam, MemoryError> {
                self.inner
                    .$accessor()
                    .ok_or_else(|| MemoryError::unsupported(Capability::$cap))
            }
        }
    };
}

decorator!(
    /// Guarded [`MemoryIngest`].
    GuardedIngest,
    dyn MemoryIngest,
    as_ingest,
    Ingest
);
decorator!(
    /// Guarded [`MemoryDocuments`].
    GuardedDocuments,
    dyn MemoryDocuments,
    as_documents,
    Documents
);
decorator!(
    /// Guarded [`MemoryTree`] — the one family that carries step 2.
    GuardedTree,
    dyn MemoryTree,
    as_tree,
    Tree
);
decorator!(
    /// Guarded [`MemoryEntities`].
    GuardedEntities,
    dyn MemoryEntities,
    as_entities,
    Entities
);
decorator!(
    /// Guarded [`MemoryGraph`].
    GuardedGraph,
    dyn MemoryGraph,
    as_graph,
    Graph
);
decorator!(
    /// Guarded [`MemoryDiff`].
    GuardedDiff,
    dyn MemoryDiff,
    as_diff,
    Diff
);
decorator!(
    /// Guarded [`MemoryGoals`].
    GuardedGoals,
    dyn MemoryGoals,
    as_goals,
    Goals
);
decorator!(
    /// Guarded [`MemoryToolMemory`].
    GuardedToolMemory,
    dyn MemoryToolMemory,
    as_tool_memory,
    ToolMemory
);
decorator!(
    /// Guarded [`MemorySourceSink`].
    GuardedSources,
    dyn MemorySourceSink,
    as_sources,
    Sources
);
decorator!(
    /// Guarded [`MemoryMaintenance`].
    GuardedMaintenance,
    dyn MemoryMaintenance,
    as_maintenance,
    Maintenance
);

// ── Ingest ───────────────────────────────────────────────────────────────────

impl GuardedIngest {
    /// Steps 3 + 4 over one ingest item: stamp provenance, redact on egress.
    fn admit(&self, mut item: IngestItem) -> IngestItem {
        item.taint = self.policy.stamp_taint(item.taint);
        item.content = self.policy.redact_outbound(&item.content).into_owned();
        item
    }
}

#[async_trait]
impl MemoryIngest for GuardedIngest {
    async fn ingest_document(&self, item: IngestItem) -> Result<IngestOutcome, MemoryError> {
        let namespace = item.namespace.clone().unwrap_or_else(|| "-".to_string());
        self.policy.admit_write(
            Capability::Ingest,
            "ingest.ingest_document",
            &namespace,
            true,
        )?;
        let item = self.admit(item);
        trace_allowed(
            &self.policy,
            "ingest.ingest_document",
            &namespace,
            item.content.chars().count(),
        );
        self.family()?.ingest_document(item).await
    }

    async fn ingest_chat(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        self.policy
            .admit_write(Capability::Ingest, "ingest.ingest_chat", NO_NAMESPACE, true)?;
        let messages: Vec<IngestItem> = messages.into_iter().map(|m| self.admit(m)).collect();
        trace_allowed(
            &self.policy,
            "ingest.ingest_chat",
            NO_NAMESPACE,
            messages.iter().map(|m| m.content.chars().count()).sum(),
        );
        self.family()?.ingest_chat(messages).await
    }
}

// ── Documents ────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryDocuments for GuardedDocuments {
    async fn put_document(&self, mut input: NamespaceDocumentInput) -> Result<String, MemoryError> {
        self.policy.admit_write(
            Capability::Documents,
            "documents.put_document",
            &input.namespace,
            true,
        )?;
        input.taint = self.policy.stamp_taint(input.taint);
        input.title = self.policy.redact_outbound(&input.title).into_owned();
        input.content = self.policy.redact_outbound(&input.content).into_owned();
        input.metadata = self.policy.redact_outbound_json(input.metadata);
        trace_allowed(
            &self.policy,
            "documents.put_document",
            &input.namespace,
            input.content.chars().count(),
        );
        self.family()?.put_document(input).await
    }

    async fn get_document(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StoredMemoryDocument>, MemoryError> {
        self.policy.admit_read(
            Capability::Documents,
            "documents.get_document",
            namespace,
            false,
        )?;
        self.family()?.get_document(namespace, key).await
    }

    async fn list_documents(
        &self,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, MemoryError> {
        self.policy.admit_read(
            Capability::Documents,
            "documents.list_documents",
            namespace.unwrap_or(NO_NAMESPACE),
            false,
        )?;
        self.family()?.list_documents(namespace).await
    }

    async fn list_namespaces(&self) -> Result<Vec<String>, MemoryError> {
        self.policy.admit_read(
            Capability::Documents,
            "documents.list_namespaces",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.list_namespaces().await
    }

    async fn delete_document(
        &self,
        namespace: &str,
        document_id: &str,
    ) -> Result<serde_json::Value, MemoryError> {
        self.policy.admit_write(
            Capability::Documents,
            "documents.delete_document",
            namespace,
            false,
        )?;
        self.family()?.delete_document(namespace, document_id).await
    }

    async fn clear_namespace(&self, namespace: &str) -> Result<(), MemoryError> {
        self.policy.admit_write(
            Capability::Documents,
            "documents.clear_namespace",
            namespace,
            false,
        )?;
        self.family()?.clear_namespace(namespace).await
    }

    async fn query_documents(
        &self,
        namespace: &str,
        query: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        // The query text itself crosses the boundary on an external driver.
        self.policy.admit_read(
            Capability::Documents,
            "documents.query_documents",
            namespace,
            true,
        )?;
        let query = self.policy.redact_outbound(query).into_owned();
        self.family()?
            .query_documents(namespace, &query, limit)
            .await
    }

    async fn recall_documents(
        &self,
        namespace: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        self.policy.admit_read(
            Capability::Documents,
            "documents.recall_documents",
            namespace,
            false,
        )?;
        self.family()?.recall_documents(namespace, limit).await
    }
}

// ── Tree ─────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryTree for GuardedTree {
    async fn append(&self, mut request: IngestRequest) -> Result<(), MemoryError> {
        self.policy
            .admit_write(Capability::Tree, "tree.append", &request.namespace, true)?;
        request.content = self.policy.redact_outbound(&request.content).into_owned();
        trace_allowed(
            &self.policy,
            "tree.append",
            &request.namespace,
            request.content.chars().count(),
        );
        self.family()?.append(request).await
    }

    /// **Step 2 lives here.** This is the only contract method in the tree
    /// today that both takes a [`SourceScope`] and applies it as a real query
    /// predicate: the embedded driver pushes `scope.allow` into
    /// `ListChunksQuery.source_scope`, which reaches SQL *before* `LIMIT`.
    ///
    /// The ambient allowlist
    /// ([`source_scope::current_source_scope`](crate::openhuman::memory::source_scope::current_source_scope))
    /// is therefore read at this boundary and passed down, rather than being
    /// applied to the returned rows. An explicit `scope` argument may only
    /// *narrow* it: the two are intersected by
    /// [`GuardPolicy::narrow_scope`](crate::openhuman::memory::guard::GuardPolicy::narrow_scope),
    /// so a caller that computed a tighter scope than the task-local still wins,
    /// while one that names a collection outside the ambient allowlist cannot
    /// widen the turn back out.
    ///
    /// There is **no double application**: the embedded `query_source` does not
    /// itself read the task-local (only the deeper `tree::retrieval` and
    /// `list_chunks` paths do, and the guard does not sit in front of those),
    /// so this fills a predicate that would otherwise be `None`.
    async fn query_source(
        &self,
        namespace: &str,
        source_id: &str,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<Chunk>, MemoryError> {
        self.policy
            .admit_read(Capability::Tree, "tree.query_source", namespace, false)?;
        let ambient = self.policy.ambient_scope();
        let effective = self.policy.narrow_scope(scope);
        log::debug!(
            "[memory:guard] tree.query_source namespace={namespace} limit={limit} \
             scoped={} scope_from={}",
            effective.is_some(),
            match (scope.is_some(), ambient.is_some()) {
                (true, true) => "argument∩ambient",
                (true, false) => "argument",
                (false, true) => "ambient",
                (false, false) => "none",
            }
        );
        self.family()?
            .query_source(namespace, source_id, limit, effective.as_ref())
            .await
    }

    async fn drill_down(&self, namespace: &str, node_id: &str) -> Result<QueryResult, MemoryError> {
        self.policy
            .admit_read(Capability::Tree, "tree.drill_down", namespace, false)?;
        self.family()?.drill_down(namespace, node_id).await
    }

    async fn seal(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        self.policy
            .admit_write(Capability::Tree, "tree.seal", namespace, false)?;
        self.family()?.seal(namespace).await
    }

    async fn cascade(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        self.policy
            .admit_write(Capability::Tree, "tree.cascade", namespace, false)?;
        self.family()?.cascade(namespace).await
    }
}

// ── Entities ─────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryEntities for GuardedEntities {
    async fn entities(
        &self,
        namespace: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityHit>, MemoryError> {
        self.policy.admit_read(
            Capability::Entities,
            "entities.entities",
            namespace,
            query.is_some(),
        )?;
        let redacted = query.map(|q| self.policy.redact_outbound(q).into_owned());
        self.family()?
            .entities(namespace, redacted.as_deref(), limit)
            .await
    }

    async fn entity_edges(
        &self,
        namespace: &str,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        self.policy.admit_read(
            Capability::Entities,
            "entities.entity_edges",
            namespace,
            false,
        )?;
        self.family()?
            .entity_edges(namespace, entity_id, limit)
            .await
    }

    async fn touch_entities(
        &self,
        namespace: &str,
        entity_ids: &[String],
    ) -> Result<(), MemoryError> {
        self.policy.admit_write(
            Capability::Entities,
            "entities.touch_entities",
            namespace,
            false,
        )?;
        self.family()?.touch_entities(namespace, entity_ids).await
    }
}

// ── Graph ────────────────────────────────────────────────────────────────────

/// Namespace label for the graph family's `Option<&str>` namespace — `None`
/// addresses the global, namespace-less slice.
fn graph_ns(namespace: Option<&str>) -> &str {
    namespace.unwrap_or(NO_NAMESPACE)
}

#[async_trait]
impl MemoryGraph for GuardedGraph {
    async fn kv_get(
        &self,
        namespace: Option<&str>,
        key: &str,
    ) -> Result<Option<MemoryKvRecord>, MemoryError> {
        self.policy.admit_read(
            Capability::Graph,
            "graph.kv_get",
            graph_ns(namespace),
            false,
        )?;
        self.family()?.kv_get(namespace, key).await
    }

    async fn kv_put(
        &self,
        namespace: Option<&str>,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), MemoryError> {
        self.policy
            .admit_write(Capability::Graph, "graph.kv_put", graph_ns(namespace), true)?;
        let value = self.policy.redact_outbound_json(value);
        self.family()?.kv_put(namespace, key, value).await
    }

    async fn kv_delete(&self, namespace: Option<&str>, key: &str) -> Result<bool, MemoryError> {
        self.policy.admit_write(
            Capability::Graph,
            "graph.kv_delete",
            graph_ns(namespace),
            false,
        )?;
        self.family()?.kv_delete(namespace, key).await
    }

    async fn kv_list(
        &self,
        namespace: Option<&str>,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryKvRecord>, MemoryError> {
        self.policy.admit_read(
            Capability::Graph,
            "graph.kv_list",
            graph_ns(namespace),
            false,
        )?;
        self.family()?.kv_list(namespace, prefix, limit).await
    }

    async fn relations(
        &self,
        namespace: Option<&str>,
        subject: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        self.policy.admit_read(
            Capability::Graph,
            "graph.relations",
            graph_ns(namespace),
            false,
        )?;
        self.family()?
            .relations(namespace, subject, predicate, limit)
            .await
    }

    async fn put_relation(&self, relation: GraphRelationRecord) -> Result<(), MemoryError> {
        self.policy.admit_write(
            Capability::Graph,
            "graph.put_relation",
            graph_ns(relation.namespace.as_deref()),
            true,
        )?;
        self.family()?.put_relation(relation).await
    }
}

// ── Diff ─────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryDiff for GuardedDiff {
    async fn capture_snapshot(&self, source_id: &str) -> Result<SnapshotRef, MemoryError> {
        self.policy.admit_write(
            Capability::Diff,
            "diff.capture_snapshot",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.capture_snapshot(source_id).await
    }

    async fn snapshots(
        &self,
        source_id: &str,
        limit: usize,
    ) -> Result<Vec<SnapshotRef>, MemoryError> {
        self.policy
            .admit_read(Capability::Diff, "diff.snapshots", NO_NAMESPACE, false)?;
        self.family()?.snapshots(source_id, limit).await
    }

    async fn diff(
        &self,
        source_id: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<DiffReport, MemoryError> {
        self.policy
            .admit_read(Capability::Diff, "diff.diff", NO_NAMESPACE, false)?;
        self.family()?.diff(source_id, from, to).await
    }
}

// ── Goals ────────────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryGoals for GuardedGoals {
    async fn goals(&self) -> Result<GoalsDoc, MemoryError> {
        self.policy
            .admit_read(Capability::Goals, "goals.goals", NO_NAMESPACE, false)?;
        self.family()?.goals().await
    }

    async fn set_goals(&self, goals: GoalsDoc) -> Result<(), MemoryError> {
        self.policy
            .admit_write(Capability::Goals, "goals.set_goals", NO_NAMESPACE, true)?;
        // The goals document's own validating mutation surface (the PII and
        // secret predicates) is host policy that already runs in
        // `memory::goals` before a document reaches the contract, so the guard
        // does not re-scrub item text here. If an external driver ever binds,
        // M6 must decide whether that upstream scrub is sufficient for egress
        // or whether item bodies need the same `redact_outbound` treatment the
        // document and ingest paths get.
        self.family()?.set_goals(goals).await
    }
}

// ── Tool memory ──────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryToolMemory for GuardedToolMemory {
    async fn tool_rules(&self, tool_name: &str) -> Result<Vec<ToolMemoryRule>, MemoryError> {
        self.policy.admit_read(
            Capability::ToolMemory,
            "tool_memory.tool_rules",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.tool_rules(tool_name).await
    }

    async fn put_tool_rule(&self, rule: ToolMemoryRule) -> Result<(), MemoryError> {
        self.policy.admit_write(
            Capability::ToolMemory,
            "tool_memory.put_tool_rule",
            NO_NAMESPACE,
            true,
        )?;
        self.family()?.put_tool_rule(rule).await
    }

    async fn delete_tool_rule(&self, tool_name: &str, rule_id: &str) -> Result<bool, MemoryError> {
        self.policy.admit_write(
            Capability::ToolMemory,
            "tool_memory.delete_tool_rule",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.delete_tool_rule(tool_name, rule_id).await
    }
}

// ── Sources ──────────────────────────────────────────────────────────────────

#[async_trait]
impl MemorySourceSink for GuardedSources {
    async fn accept_source_items(
        &self,
        source_id: &str,
        source_kind: &str,
        items: Vec<SourceItem>,
        taint: MemoryTaint,
    ) -> Result<IngestOutcome, MemoryError> {
        self.policy.admit_write(
            Capability::Sources,
            "sources.accept_source_items",
            NO_NAMESPACE,
            true,
        )?;
        // Step 3: the batch taint is the guard's to decide. `stamp_taint` never
        // downgrades, so a sync path that already asked for `ExternalSync` keeps
        // it whether or not a source scope is active.
        let taint = self.policy.stamp_taint(taint);
        let items: Vec<SourceItem> = items
            .into_iter()
            .map(|mut item| {
                item.title = self.policy.redact_outbound(&item.title).into_owned();
                item.content = self.policy.redact_outbound(&item.content).into_owned();
                item
            })
            .collect();
        trace_allowed(
            &self.policy,
            "sources.accept_source_items",
            NO_NAMESPACE,
            items.iter().map(|i| i.content.chars().count()).sum(),
        );
        self.family()?
            .accept_source_items(source_id, source_kind, items, taint)
            .await
    }

    async fn forget_source(&self, source_id: &str) -> Result<u64, MemoryError> {
        self.policy.admit_write(
            Capability::Sources,
            "sources.forget_source",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.forget_source(source_id).await
    }
}

// ── Maintenance ──────────────────────────────────────────────────────────────

#[async_trait]
impl MemoryMaintenance for GuardedMaintenance {
    async fn reembed(&self) -> Result<MaintenanceReport, MemoryError> {
        self.policy.admit_write(
            Capability::Maintenance,
            "maintenance.reembed",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.reembed().await
    }

    async fn compact(&self) -> Result<MaintenanceReport, MemoryError> {
        self.policy.admit_write(
            Capability::Maintenance,
            "maintenance.compact",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.compact().await
    }

    async fn consolidate(&self) -> Result<MaintenanceReport, MemoryError> {
        self.policy.admit_write(
            Capability::Maintenance,
            "maintenance.consolidate",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.consolidate().await
    }

    /// Read-only by contract, so this takes the **read** tier check: a
    /// `readonly` operator must still be able to run `doctor`, which is exactly
    /// the tier where diagnosing without mutating matters most.
    async fn doctor(&self) -> Result<MaintenanceReport, MemoryError> {
        self.policy.admit_read(
            Capability::Maintenance,
            "maintenance.doctor",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.doctor().await
    }
}

#[cfg(test)]
#[path = "families_tests.rs"]
mod tests;
