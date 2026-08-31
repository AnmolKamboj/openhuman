use std::sync::Arc;

use crate::openhuman::memory::api::capabilities::Capability;
use crate::openhuman::memory::api::chunks::Chunk;
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::goals::GoalsDoc;
use crate::openhuman::memory::api::provider::chunks::{
    ChunkDetail, ChunkEmbedding, ChunkListRow, ChunkQuery, MemoryChunks, SourceTotal,
};
use crate::openhuman::memory::api::provider::episodic::{
    ConversationSegment, EpisodicTurn, MemoryEpisodic,
};
use crate::openhuman::memory::api::provider::people::{
    AddressBookSeedOutcome, MemoryPeople, PersonHandle, PersonInteraction, PersonRecord,
    PersonScore, RankedPerson, ResolvedPerson,
};
use crate::openhuman::memory::api::provider::profile::{
    FacetType, MemoryProfile, ProfileFacet, UserState,
};
use crate::openhuman::memory::api::provider::retrieval::{
    CoverWindowQuery, EntityMatch, FastRetrieveQuery, MemoryRetrieval, RetrievalHit,
    RetrievalResponse, SourceRetrievalQuery,
};
use crate::openhuman::memory::api::provider::scoring::MemoryScoring;
use crate::openhuman::memory::api::provider::sessions::{
    CodingSessionIngestReport, CodingSessionIngestRequest, CodingSessionSource,
};
use crate::openhuman::memory::api::provider::sync::{
    RawArchiveCoverage, RawRebuildOutcome, SourceSyncState, SourceSyncStatus, SyncAuditEntry,
    SyncRunOutcome,
};
use crate::openhuman::memory::api::provider::types::{
    ChunkEntityOccurrence, DiffReport, EntityHit, EntityOccurrence, ForgetOutcome, ForgetSelector,
    IngestItem, IngestOutcome, MaintenanceReport, PurgeOutcome, SnapshotRef, SourceItem,
    SourceScope,
};
use crate::openhuman::memory::api::provider::{
    EpisodicEvent, MemoryCodingSessions, MemoryDiff, MemoryDocuments, MemoryEntities, MemoryGoals,
    MemoryGraph, MemoryIngest, MemoryMaintenance, MemoryProvider, MemorySourceSink,
    MemorySourceSync, MemoryToolMemory, MemoryTree,
};
use crate::openhuman::memory::api::tool_memory::ToolMemoryRule;
use crate::openhuman::memory::api::tree::{
    IngestRequest, QueryResult, SummaryForest, TreeLeaf, TreeStatus,
};
use crate::openhuman::memory::api::types::NamespaceMemoryHit;
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
decorator!(
    /// Guarded [`MemoryPeople`].
    GuardedPeople,
    dyn MemoryPeople,
    as_people,
    People
);
decorator!(
    /// Guarded [`MemoryChunks`].
    GuardedChunks,
    dyn MemoryChunks,
    as_chunks,
    Chunks
);
decorator!(
    /// Guarded [`MemoryRetrieval`].
    GuardedRetrieval,
    dyn MemoryRetrieval,
    as_retrieval,
    Retrieval
);
decorator!(
    /// Guarded [`MemoryEpisodic`].
    GuardedEpisodic,
    dyn MemoryEpisodic,
    as_episodic,
    Episodic
);
decorator!(
    /// Guarded [`MemorySourceSync`].
    GuardedSourceSync,
    dyn MemorySourceSync,
    as_source_sync,
    SourceSync
);
decorator!(
    /// Guarded [`MemoryCodingSessions`].
    GuardedCodingSessions,
    dyn MemoryCodingSessions,
    as_coding_sessions,
    CodingSessions
);
decorator!(
    /// Guarded [`MemoryProfile`].
    GuardedProfile,
    dyn MemoryProfile,
    as_profile,
    Profile
);
decorator!(
    /// Guarded [`MemoryScoring`].
    GuardedScoring,
    dyn MemoryScoring,
    as_scoring,
    Scoring
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

    async fn ingest_email(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        // Admitted exactly like chat: a thread is one conversation with no
        // namespace of its own, and every message is taint-stamped and
        // redacted before it reaches the driver.
        self.policy.admit_write(
            Capability::Ingest,
            "ingest.ingest_email",
            NO_NAMESPACE,
            true,
        )?;
        let messages: Vec<IngestItem> = messages.into_iter().map(|m| self.admit(m)).collect();
        trace_allowed(
            &self.policy,
            "ingest.ingest_email",
            NO_NAMESPACE,
            messages.iter().map(|m| m.content.chars().count()).sum(),
        );
        self.family()?.ingest_email(messages).await
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

    /// Enumerates the sealed forest, so it narrows by the ambient scope for the
    /// same reason [`Self::query_source`] does: a summary is derived from the
    /// chunks beneath it, and handing back a node built from sources the caller
    /// may not read discloses their contents in condensed form.
    async fn summary_forest(
        &self,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<SummaryForest, MemoryError> {
        self.policy
            .admit_read(Capability::Tree, "tree.summary_forest", NO_NAMESPACE, false)?;
        let effective = self.policy.narrow_scope(scope);
        self.family()?
            .summary_forest(limit, effective.as_ref())
            .await
    }

    /// Sealing one source's tree is a write, and it names the scope it acts on
    /// — so unlike the reads above it is admitted against that scope rather
    /// than `NO_NAMESPACE`. `carries_content: false`: the caller supplies a
    /// scope label, never prose, and the seals it fires write content the
    /// driver already holds.
    async fn flush_source_tree(&self, source_scope: &str) -> Result<u64, MemoryError> {
        self.policy.admit_write(
            Capability::Tree,
            "tree.flush_source_tree",
            source_scope,
            false,
        )?;
        self.family()?.flush_source_tree(source_scope).await
    }

    /// Leaves are chunks, so this is the same disclosure as
    /// [`Self::query_source`] with a different ordering, and takes the same
    /// intersection.
    async fn recent_leaves(
        &self,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<TreeLeaf>, MemoryError> {
        self.policy
            .admit_read(Capability::Tree, "tree.recent_leaves", NO_NAMESPACE, false)?;
        let effective = self.policy.narrow_scope(scope);
        self.family()?
            .recent_leaves(limit, effective.as_ref())
            .await
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

    /// The occurrence index has no namespace and the contract gives this member
    /// no scope argument, so there is nothing to intersect — the tier check is
    /// the whole gate. Worth stating rather than leaving as an apparent
    /// omission beside the scoped members above.
    async fn top_entities(
        &self,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityOccurrence>, MemoryError> {
        self.policy.admit_read(
            Capability::Entities,
            "entities.top_entities",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.top_entities(kind, limit).await
    }

    /// Scoped by the chunk ids the caller already holds: it can only name
    /// chunks a previous, scoped read handed it, so this adds no reach beyond
    /// the read that produced them.
    async fn chunk_entities(
        &self,
        chunk_ids: &[String],
        kinds: Option<&[String]>,
    ) -> Result<Vec<ChunkEntityOccurrence>, MemoryError> {
        self.policy.admit_read(
            Capability::Entities,
            "entities.chunk_entities",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.chunk_entities(chunk_ids, kinds).await
    }

    /// Returns ids only, never content. A caller still has to read those chunks
    /// through [`MemoryChunks`] to see anything, and that path applies the
    /// scope intersection.
    async fn entity_chunk_ids(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<String>, MemoryError> {
        self.policy.admit_read(
            Capability::Entities,
            "entities.entity_chunk_ids",
            NO_NAMESPACE,
            false,
        )?;
        self.family()?.entity_chunk_ids(entity_id, limit).await
    }
}

// ── Graph ────────────────────────────────────────────────────────────────────

/// Namespace label for the graph family's `Option<&str>` namespace — `None`
/// addresses the global, namespace-less slice.
fn graph_ns(namespace: Option<&str>) -> &str {
    namespace.unwrap_or(NO_NAMESPACE)
}
