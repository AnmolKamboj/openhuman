
#[async_trait]
impl MemoryDocuments for ModuleMemoryProvider {
    async fn put_document(&self, input: NamespaceDocumentInput) -> Result<String, MemoryError> {
        module_call!(self, "put_document", "PutDocument", (input,))
    }
    async fn get_document(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StoredMemoryDocument>, MemoryError> {
        module_call!(self, "get_document", "GetDocument", (namespace, key))
    }
    async fn list_documents(
        &self,
        namespace: Option<&str>,
    ) -> Result<serde_json::Value, MemoryError> {
        module_call!(
            self,
            "list_documents",
            "ListDocuments",
            (namespace.map(str::to_string),)
        )
    }
    async fn list_namespaces(&self) -> Result<Vec<String>, MemoryError> {
        module_call!(self, "list_namespaces", "ListNamespaces", ())
    }
    async fn delete_document(
        &self,
        namespace: &str,
        document_id: &str,
    ) -> Result<serde_json::Value, MemoryError> {
        module_call!(
            self,
            "delete_document",
            "DeleteDocument",
            (namespace, document_id)
        )
    }
    async fn clear_namespace(&self, namespace: &str) -> Result<(), MemoryError> {
        module_call!(self, "clear_namespace", "ClearNamespace", (namespace,))
    }
    async fn query_documents(
        &self,
        namespace: &str,
        query: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        module_call!(
            self,
            "query_documents",
            "QueryDocuments",
            (namespace, query, limit)
        )
    }
    async fn recall_documents(
        &self,
        namespace: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        module_call!(
            self,
            "recall_documents",
            "RecallDocuments",
            (namespace, limit)
        )
    }
}

#[async_trait]
impl MemoryTree for ModuleMemoryProvider {
    async fn append(&self, request: IngestRequest) -> Result<(), MemoryError> {
        module_call!(self, "append", "Append", (request,))
    }
    async fn query_source(
        &self,
        namespace: &str,
        source_id: &str,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<Chunk>, MemoryError> {
        module_call!(
            self,
            "query_source",
            "QuerySource",
            (namespace, source_id, limit, scope.cloned())
        )
    }
    async fn drill_down(&self, namespace: &str, node_id: &str) -> Result<QueryResult, MemoryError> {
        module_call!(self, "drill_down", "DrillDown", (namespace, node_id))
    }
    async fn seal(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        module_call!(self, "seal", "Seal", (namespace,))
    }
    async fn cascade(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        module_call!(self, "cascade", "Cascade", (namespace,))
    }
    async fn summary_forest(
        &self,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<SummaryForest, MemoryError> {
        module_call!(self, "summary_forest", "SummaryForest", (limit, scope))
    }

    async fn flush_source_tree(&self, source_scope: &str) -> Result<u64, MemoryError> {
        module_call!(
            self,
            "flush_source_tree",
            "FlushSourceTree",
            (source_scope,)
        )
    }
    async fn recent_leaves(
        &self,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<TreeLeaf>, MemoryError> {
        module_call!(self, "recent_leaves", "RecentLeaves", (limit, scope))
    }
}

#[async_trait]
impl MemoryEntities for ModuleMemoryProvider {
    async fn entities(
        &self,
        namespace: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityHit>, MemoryError> {
        module_call!(
            self,
            "entities",
            "Entities",
            (namespace, query.map(str::to_string), limit)
        )
    }
    async fn entity_edges(
        &self,
        namespace: &str,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        module_call!(
            self,
            "entity_edges",
            "EntityEdges",
            (namespace, entity_id, limit)
        )
    }
    async fn touch_entities(
        &self,
        namespace: &str,
        entity_ids: &[String],
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "touch_entities",
            "TouchEntities",
            (namespace, entity_ids.to_vec())
        )
    }
    async fn top_entities(
        &self,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityOccurrence>, MemoryError> {
        module_call!(self, "top_entities", "TopEntities", (kind, limit))
    }
    async fn chunk_entities(
        &self,
        chunk_ids: &[String],
        kinds: Option<&[String]>,
    ) -> Result<Vec<ChunkEntityOccurrence>, MemoryError> {
        module_call!(self, "chunk_entities", "ChunkEntities", (chunk_ids, kinds))
    }
    async fn entity_chunk_ids(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<String>, MemoryError> {
        module_call!(
            self,
            "entity_chunk_ids",
            "EntityChunkIds",
            (entity_id, limit)
        )
    }
}

#[async_trait]
impl MemoryGraph for ModuleMemoryProvider {
    async fn kv_get(
        &self,
        namespace: Option<&str>,
        key: &str,
    ) -> Result<Option<MemoryKvRecord>, MemoryError> {
        module_call!(
            self,
            "kv_get",
            "KvGet",
            (namespace.map(str::to_string), key)
        )
    }
    async fn kv_put(
        &self,
        namespace: Option<&str>,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "kv_put",
            "KvPut",
            (namespace.map(str::to_string), key, value)
        )
    }
    async fn kv_delete(&self, namespace: Option<&str>, key: &str) -> Result<bool, MemoryError> {
        module_call!(
            self,
            "kv_delete",
            "KvDelete",
            (namespace.map(str::to_string), key)
        )
    }
    async fn kv_list(
        &self,
        namespace: Option<&str>,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryKvRecord>, MemoryError> {
        module_call!(
            self,
            "kv_list",
            "KvList",
            (
                namespace.map(str::to_string),
                prefix.map(str::to_string),
                limit
            )
        )
    }
    async fn relations(
        &self,
        namespace: Option<&str>,
        subject: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        module_call!(
            self,
            "relations",
            "Relations",
            (
                namespace.map(str::to_string),
                subject.map(str::to_string),
                predicate.map(str::to_string),
                limit
            )
        )
    }
    async fn put_relation(&self, relation: GraphRelationRecord) -> Result<(), MemoryError> {
        module_call!(self, "put_relation", "PutRelation", (relation,))
    }
}

#[async_trait]
impl MemoryDiff for ModuleMemoryProvider {
    async fn capture_snapshot(&self, source_id: &str) -> Result<SnapshotRef, MemoryError> {
        module_call!(self, "capture_snapshot", "CaptureSnapshot", (source_id,))
    }
    async fn snapshots(
        &self,
        source_id: &str,
        limit: usize,
    ) -> Result<Vec<SnapshotRef>, MemoryError> {
        module_call!(self, "snapshots", "Snapshots", (source_id, limit))
    }
    async fn diff(
        &self,
        source_id: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<DiffReport, MemoryError> {
        module_call!(
            self,
            "diff",
            "Diff",
            (source_id, from.map(str::to_string), to)
        )
    }
}

#[async_trait]
impl MemoryGoals for ModuleMemoryProvider {
    async fn goals(&self) -> Result<GoalsDoc, MemoryError> {
        module_call!(self, "goals", "Goals", ())
    }
    async fn set_goals(&self, goals: GoalsDoc) -> Result<(), MemoryError> {
        module_call!(self, "set_goals", "SetGoals", (goals,))
    }
}

#[async_trait]
impl MemoryToolMemory for ModuleMemoryProvider {
    async fn tool_rules(&self, tool_name: &str) -> Result<Vec<ToolMemoryRule>, MemoryError> {
        module_call!(self, "tool_rules", "ToolRules", (tool_name,))
    }
    async fn put_tool_rule(&self, rule: ToolMemoryRule) -> Result<(), MemoryError> {
        module_call!(self, "put_tool_rule", "PutToolRule", (rule,))
    }
    async fn delete_tool_rule(&self, tool_name: &str, rule_id: &str) -> Result<bool, MemoryError> {
        module_call!(
            self,
            "delete_tool_rule",
            "DeleteToolRule",
            (tool_name, rule_id)
        )
    }
}

#[async_trait]
impl MemorySourceSink for ModuleMemoryProvider {
    async fn accept_source_items(
        &self,
        source_id: &str,
        source_kind: &str,
        items: Vec<SourceItem>,
        taint: MemoryTaint,
    ) -> Result<IngestOutcome, MemoryError> {
        module_call!(
            self,
            "accept_source_items",
            "AcceptSourceItems",
            (source_id, source_kind, items, taint)
        )
    }
    async fn forget_source(&self, source_id: &str) -> Result<u64, MemoryError> {
        module_call!(self, "forget_source", "ForgetSource", (source_id,))
    }
    async fn forget_matching(
        &self,
        selector: &ForgetSelector,
    ) -> Result<ForgetOutcome, MemoryError> {
        module_call!(self, "forget_matching", "ForgetMatching", (selector,))
    }
}

#[async_trait]
impl MemoryMaintenance for ModuleMemoryProvider {
    async fn reembed(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "reembed", "Reembed", ())
    }
    async fn compact(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "compact", "Compact", ())
    }
    async fn consolidate(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "consolidate", "Consolidate", ())
    }
    async fn doctor(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "doctor", "Doctor", ())
    }
    async fn retry_failed(&self) -> Result<MaintenanceReport, MemoryError> {
        module_call!(self, "retry_failed", "RetryFailed", ())
    }
    async fn store_stats(&self) -> Result<StoreStats, MemoryError> {
        module_call!(self, "store_stats", "StoreStats", ())
    }
    async fn queue_stats(&self, kind: Option<&str>) -> Result<QueueStats, MemoryError> {
        module_call!(self, "queue_stats", "QueueStats", (kind,))
    }
    async fn latest_queue_failure(&self) -> Result<Option<QueueFailure>, MemoryError> {
        module_call!(self, "latest_queue_failure", "LatestQueueFailure", ())
    }
    async fn backfill_in_progress(&self) -> Result<bool, MemoryError> {
        module_call!(self, "backfill_in_progress", "BackfillInProgress", ())
    }
    async fn flush_pending(&self) -> Result<FlushOutcome, MemoryError> {
        module_call!(self, "flush_pending", "FlushPending", ())
    }
    async fn reset_derived_index(&self) -> Result<ResetOutcome, MemoryError> {
        module_call!(self, "reset_derived_index", "ResetDerivedIndex", ())
    }
    async fn purge_all(&self) -> Result<PurgeOutcome, MemoryError> {
        module_call!(self, "purge_all", "PurgeAll", ())
    }
    async fn diagnose(&self) -> Result<Diagnosis, MemoryError> {
        module_call!(self, "diagnose", "Diagnose", ())
    }
}

/// Bus deadline for the three calls that run a whole source sync inside the
/// module: `RunConnectionSync`, `RunSourceSync` and `BootstrapConnection`.
///
/// tinybus gives every call a 30 s default deadline if nobody sets one, and a
/// sync is routinely longer than that: one Gmail page is ~31 s end to end, an
/// initial bootstrap of a connection is minutes. With the default, the caller
/// was released with "call to `RunSourceSync` timed out after 30000ms" while
/// the module kept fetching and ingesting, and finished; the UI reported a
/// failure for work that succeeded (openhuman#5820). Same failure class, same
/// fix as `IngestCodingSessions` above: the deadline here is the wedged-forever
/// backstop tinybus requires, not a ceiling anyone is meant to hit.
///
/// Sized from the frontend's clamp, `PER_CALL_TIMEOUT_MAX_MS = 600 s`
/// (`app/src/services/coreRpcClient.ts`): that is the longest wait any RPC
/// caller can observe, so the bus must outlast it, plus [`INGEST_BUS_GRACE`]
/// so the client's own abort, with its clean message, is the one that fires
/// first when a run really does wedge.
const SOURCE_SYNC_BUS_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(600).saturating_add(INGEST_BUS_GRACE);

#[async_trait]
impl MemorySourceSync for ModuleMemoryProvider {
    async fn run_connection_sync(
        &self,
        toolkit: &str,
        connection_id: &str,
    ) -> Result<SyncRunOutcome, MemoryError> {
        self.proxy("run_connection_sync")
            .await?
            .with_timeout(SOURCE_SYNC_BUS_TIMEOUT)
            .call("RunConnectionSync", (toolkit, connection_id))
            .await
            .map_err(|error| from_bus(&error))
    }
    async fn run_source_sync(&self, source_id: &str) -> Result<SyncRunOutcome, MemoryError> {
        self.proxy("run_source_sync")
            .await?
            .with_timeout(SOURCE_SYNC_BUS_TIMEOUT)
            .call("RunSourceSync", (source_id,))
            .await
            .map_err(|error| from_bus(&error))
    }
    async fn bootstrap_connection(
        &self,
        toolkit: &str,
        connection_id: &str,
    ) -> Result<(), MemoryError> {
        self.proxy("bootstrap_connection")
            .await?
            .with_timeout(SOURCE_SYNC_BUS_TIMEOUT)
            .call("BootstrapConnection", (toolkit, connection_id))
            .await
            .map_err(|error| from_bus(&error))
    }
    async fn is_toolkit_syncable(&self, toolkit: &str) -> Result<bool, MemoryError> {
        module_call!(self, "is_toolkit_syncable", "IsToolkitSyncable", (toolkit,))
    }
    async fn source_sync_state(
        &self,
        toolkit: &str,
        connection_id: &str,
    ) -> Result<Option<SourceSyncState>, MemoryError> {
        module_call!(
            self,
            "source_sync_state",
            "SourceSyncState",
            (toolkit, connection_id)
        )
    }
    async fn sync_audit_log(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<SyncAuditEntry>, MemoryError> {
        module_call!(self, "sync_audit_log", "SyncAuditLog", (limit,))
    }
    async fn estimate_sync_cost_usd(
        &self,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Result<f64, MemoryError> {
        module_call!(
            self,
            "estimate_sync_cost_usd",
            "EstimateSyncCostUsd",
            (input_tokens, output_tokens)
        )
    }
    async fn sync_statuses(&self) -> Result<Vec<SourceSyncStatus>, MemoryError> {
        module_call!(self, "sync_statuses", "SyncStatuses", ())
    }
    async fn raw_archive_coverage(
        &self,
        tree_scope: &str,
        archive_source_id: &str,
    ) -> Result<RawArchiveCoverage, MemoryError> {
        module_call!(
            self,
            "raw_archive_coverage",
            "RawArchiveCoverage",
            (tree_scope, archive_source_id)
        )
    }
    async fn rebuild_from_raw_archive(
        &self,
        tree_scope: &str,
        archive_source_id: &str,
    ) -> Result<RawRebuildOutcome, MemoryError> {
        module_call!(
            self,
            "rebuild_from_raw_archive",
            "RebuildFromRawArchive",
            (tree_scope, archive_source_id)
        )
    }
}

#[async_trait]
impl MemoryCodingSessions for ModuleMemoryProvider {
    async fn coding_session_status(&self) -> Result<Vec<CodingSessionSource>, MemoryError> {
        module_call!(self, "coding_session_status", "CodingSessionStatus", ())
    }
    /// # Why this one call sets its own bus deadline
    ///
    /// Every other member here takes tinybus' `DEFAULT_TIMEOUT`
    /// (`vendor/tinybus/crates/tinybus/src/connection.rs:56`) — a flat 30 s,
    /// applied by `Proxy::new` (`proxy.rs:59`) whenever nobody says otherwise.
    /// That is the right default for a memory read. It is the wrong one here:
    /// distilling a coding session is several *sequential* model calls, and the
    /// RPC above it already computes a budget sized to the work
    /// (`memory::sources::rpc::ingest_budget`, 120 s + 90 s per session, capped
    /// at 600 s).
    ///
    /// So there were two deadlines and the tighter one was the one nobody
    /// chose. A real 35 s import tripped the 30 s default; the caller was
    /// released with an error while the module kept working and finished
    /// seconds later, having imported everything. The UI reported a failure for
    /// work that had succeeded, and invited a retry that would redo it
    /// (#5802).
    ///
    /// tinybus is explicit that this is the caller's problem to size: *"A
    /// timeout does not cancel the remote work — tinybus cannot — it stops
    /// waiting and frees the caller"* (`connection.rs:22-23`). Abandoning the
    /// call early therefore does not save anything; it only loses the report.
    ///
    /// The budget is taken from `ingest_budget` rather than restated, so the
    /// two layers cannot drift, plus [`INGEST_BUS_GRACE`]. The grace makes the
    /// ordering deterministic instead of a race between two equal deadlines:
    /// the RPC's own `tokio::time::timeout` fires first and reports its clean
    /// structured message, and this deadline survives only as the
    /// wedged-forever backstop tinybus requires. Same shape as the client's
    /// `CODING_SESSION_RPC_GRACE_MS` sitting above the server budget.
    async fn ingest_coding_sessions(
        &self,
        request: CodingSessionIngestRequest,
    ) -> Result<CodingSessionIngestReport, MemoryError> {
        let deadline = crate::openhuman::memory::sources::rpc::ingest_budget(request.max_sessions)
            + INGEST_BUS_GRACE;
        self.proxy("ingest_coding_sessions")
            .await?
            .with_timeout(deadline)
            .call("IngestCodingSessions", (request,))
            .await
            .map_err(|error| from_bus(&error))
    }
}

/// Head-room added to [`ingest_budget`](crate::openhuman::memory::sources::rpc::ingest_budget)
/// for the bus deadline on `IngestCodingSessions`.
///
/// Exists to order two deadlines, not to allow more work: the RPC's own
/// wall-clock ceiling must be the one that fires, because its message names the
/// budget rather than the wire member. Anything comfortably longer than the
/// scheduling jitter between the two `tokio::time::timeout` arms would do.
const INGEST_BUS_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

#[async_trait]
impl MemoryEpisodic for ModuleMemoryProvider {
    async fn insert_turn(&self, turn: &EpisodicTurn) -> Result<i64, MemoryError> {
        module_call!(self, "insert_turn", "InsertTurn", (turn,))
    }
    async fn session_turns(&self, session_id: &str) -> Result<Vec<EpisodicTurn>, MemoryError> {
        module_call!(self, "session_turns", "SessionTurns", (session_id,))
    }
    async fn open_segment(
        &self,
        session_id: &str,
    ) -> Result<Option<ConversationSegment>, MemoryError> {
        module_call!(self, "open_segment", "OpenSegment", (session_id,))
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "trait signature; see the contract's rationale"
    )]
    async fn create_segment(
        &self,
        segment_id: &str,
        session_id: &str,
        namespace: &str,
        start_episodic_id: i64,
        start_seq: Option<u32>,
        start_timestamp: f64,
        now: f64,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "create_segment",
            "CreateSegment",
            (
                segment_id,
                session_id,
                namespace,
                start_episodic_id,
                start_seq,
                start_timestamp,
                now
            )
        )
    }
    async fn append_turn(
        &self,
        segment_id: &str,
        episodic_id: i64,
        seq: Option<u32>,
        timestamp: f64,
        now: f64,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "append_turn",
            "AppendTurn",
            (segment_id, episodic_id, seq, timestamp, now)
        )
    }
    async fn insert_event(&self, event: &EpisodicEvent) -> Result<(), MemoryError> {
        module_call!(self, "insert_event", "InsertEvent", (event,))
    }
    async fn close_segment(&self, segment_id: &str, now: f64) -> Result<(), MemoryError> {
        module_call!(self, "close_segment", "CloseSegment", (segment_id, now))
    }
    async fn set_segment_summary(
        &self,
        segment_id: &str,
        summary: &str,
        now: f64,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "set_segment_summary",
            "SetSegmentSummary",
            (segment_id, summary, now)
        )
    }
    async fn upsert_segment_embedding(
        &self,
        segment_id: &str,
        model_signature: &str,
        embedding: &[f32],
        created_at: f64,
    ) -> Result<(), MemoryError> {
        module_call!(
            self,
            "upsert_segment_embedding",
            "UpsertSegmentEmbedding",
            (segment_id, model_signature, embedding, created_at)
        )
    }
}
