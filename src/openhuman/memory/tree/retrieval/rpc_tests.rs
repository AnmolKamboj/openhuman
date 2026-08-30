//! Unit tests for the Phase 4 retrieval RPC handlers.
//!
//! Scope: the handler layer specifically — param parsing, default
//! fallbacks, `SourceKind` / `EntityKind` validation, the scope each call
//! forwards to the contract, `RpcOutcome` envelope shape, and PII-redacted
//! log formatting. Retrieval correctness is the driver's and is covered by
//! the engine's own tests; these deliberately do NOT re-verify it.
//!
//! All five handlers read through the bound driver, and a unit test cannot
//! load the compiled module — so every test that gets past parameter
//! validation binds one. [`bind_without_retrieval`] pins the degrade path;
//! [`bind_recording`] pins what the handler hands the contract and what it
//! does with the answer.
//!
//! One test binds the real in-process driver instead
//! ([`install_tinycortex_for_test`]): the source gate has to be proved end
//! to end, because "the handler passed a scope" and "a restricted profile
//! cannot read another source" are different claims and only the second one
//! is the security property. It is the driver the loadable module wraps,
//! which is as close to production as a test process can get.
//!
//! [`install_tinycortex_for_test`]: crate::openhuman::memory::test_support::install_tinycortex_for_test
use std::sync::{Arc, Mutex};

use super::*;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use crate::openhuman::memory::api::capabilities::Capabilities;
use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::health::MemoryHealth;
use crate::openhuman::memory::api::provider::retrieval::{
    FastRetrieveQuery, MemoryRetrieval, RetrievalNodeKind,
};
use crate::openhuman::memory::api::provider::types::{
    ExportPage, ExportRecord, ImportOutcome, SourceScope,
};
use crate::openhuman::memory::api::provider::{
    MemoryCore, MemoryPortability, MemoryProvider, MemoryRecall,
};
use crate::openhuman::memory::api::recall::OwnedRecallOpts;
use crate::openhuman::memory::api::types::{
    MemoryCategory, MemoryEntry, MemoryTaint, NamespaceMemoryHit, NamespaceSummary,
};
use crate::openhuman::memory::source_scope::with_source_scope;
// The engine-backed chunk writes these fixtures need live in
// `retrieval::test_support` rather than here. They are the engine's chunk
// store — `MemoryChunks` is read-only on the contract — and they are
// test-only, which is exactly the pair `direct_engine_refs_tests`'
// line-based scanner cannot tell apart when the reference sits inside an
// inline `#[cfg(test)]` module. See that module's docs.
use crate::openhuman::memory::tree::retrieval::test_support::{stage_test_chunks, upsert_chunks};
use tinymemory_api::chunks::{chunk_id, Chunk, Metadata, SourceRef};
use tinymemory_api::null::NullMemoryProvider;

fn test_config() -> (TempDir, Config) {
    let tmp = TempDir::new().unwrap();
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    // Inert embedder: the driver-backed test below must not reach a real
    // embedding endpoint, and none of these reads rank against a query.
    cfg.memory_tree.embedding_endpoint = None;
    cfg.memory_tree.embedding_model = None;
    cfg.memory_tree.embedding_strict = false;
    (tmp, cfg)
}

fn sample_chunk(source: &str, seq: u32) -> Chunk {
    let ts = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
    Chunk {
        id: chunk_id(SourceKind::Chat, source, seq, "test-content"),
        content: format!("content-{source}-{seq}"),
        metadata: Metadata {
            source_kind: SourceKind::Chat,
            source_id: source.into(),
            owner: "alice".into(),
            timestamp: ts,
            time_range: (ts, ts),
            tags: vec![],
            source_ref: Some(SourceRef::new(format!("slack://{source}/{seq}"))),
            path_scope: None,
        },
        token_count: 20,
        seq_in_source: seq,
        created_at: ts,
        partial_message: false,
    }
}

/// Bind a driver with no retrieval family as `cfg`'s memory driver.
///
/// `FixedDiagnostics` is `NullMemoryProvider`-backed and overrides only
/// `as_maintenance`, so `as_retrieval()` is `None` — the shape of a driver
/// that serves memory without exposing the engine's retrieval primitives.
/// Every test that reaches a handler past its parameter validation needs a
/// binding installed: without one, resolving a driver tries to load the
/// compiled module, which in a test process can block rather than fail.
fn bind_without_retrieval(cfg: &Config) {
    crate::openhuman::memory::binding::install_diagnostics_for_test(
        &cfg.workspace_dir,
        &cfg.subsystems.memory,
        Default::default(),
        Default::default(),
    );
}

/// Bind `driver` as `cfg`'s memory driver and keep a handle on it.
fn bind_recording(cfg: &Config, driver: RecordingRetrieval) -> Arc<RecordingRetrieval> {
    let driver = Arc::new(driver);
    crate::openhuman::memory::binding::install_for_test(
        &cfg.workspace_dir,
        &cfg.subsystems.memory,
        Arc::clone(&driver) as Arc<dyn MemoryProvider>,
    );
    driver
}

/// One scripted hit, carrying the `tree_kind` every engine-produced hit has.
fn hit(node_id: &str) -> RetrievalHit {
    let ts = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
    RetrievalHit {
        node_id: node_id.to_string(),
        node_kind: RetrievalNodeKind::Leaf,
        tree_id: String::new(),
        tree_kind: Some("source".to_string()),
        tree_scope: String::new(),
        level: 0,
        content: format!("content-{node_id}"),
        entities: Vec::new(),
        topics: Vec::new(),
        time_range_start: ts,
        time_range_end: ts,
        score: 1.0,
        child_ids: Vec::new(),
        source_ref: None,
    }
}

/// What one contract call was handed.
#[derive(Default)]
struct Calls {
    /// The scope argument per member, in call order. Recorded because an
    /// absent scope means UNRESTRICTED on the far side: a handler that
    /// passed `None` where a turn had an allowlist would fail the source
    /// gate open, and the answer would look identical either way.
    scopes: Vec<(String, Option<SourceScope>)>,
    source_queries: Vec<SourceRetrievalQuery>,
    windows: Vec<CoverWindowQuery>,
}

/// A driver whose only advertised behaviour is retrieval: it records the
/// arguments it was handed and answers with scripted hits, or with a
/// rejection when one is scripted.
struct RecordingRetrieval {
    inner: NullMemoryProvider,
    hits: Vec<RetrievalHit>,
    invalid: Option<String>,
    calls: Mutex<Calls>,
}

impl RecordingRetrieval {
    fn new() -> Self {
        Self {
            inner: NullMemoryProvider::new(),
            hits: Vec::new(),
            invalid: None,
            calls: Mutex::new(Calls::default()),
        }
    }

    fn answering(mut self, hits: Vec<RetrievalHit>) -> Self {
        self.hits = hits;
        self
    }

    /// Reject every retrieval call with `message`, the way the engine
    /// rejects an inverted window.
    fn rejecting(mut self, message: &str) -> Self {
        self.invalid = Some(message.to_string());
        self
    }

    fn record(&self, member: &str, scope: Option<&SourceScope>) {
        self.calls
            .lock()
            .expect("calls lock")
            .scopes
            .push((member.to_string(), scope.cloned()));
    }

    /// The scope `member` was handed. The outer `Option` distinguishes
    /// "never called" from "called with no scope".
    fn scope_for(&self, member: &str) -> Option<Option<SourceScope>> {
        self.calls
            .lock()
            .expect("calls lock")
            .scopes
            .iter()
            .find(|(m, _)| m == member)
            .map(|(_, scope)| scope.clone())
    }

    fn source_query(&self) -> SourceRetrievalQuery {
        self.calls
            .lock()
            .expect("calls lock")
            .source_queries
            .first()
            .cloned()
            .expect("retrieve_source was called")
    }

    fn window(&self) -> CoverWindowQuery {
        self.calls
            .lock()
            .expect("calls lock")
            .windows
            .first()
            .cloned()
            .expect("cover_window was called")
    }

    fn answer(&self) -> Result<Vec<RetrievalHit>, MemoryError> {
        match &self.invalid {
            Some(message) => Err(MemoryError::Invalid(message.clone())),
            None => Ok(self.hits.clone()),
        }
    }

    fn page(&self) -> Result<RetrievalResponse, MemoryError> {
        let hits = self.answer()?;
        let total = hits.len();
        Ok(RetrievalResponse {
            hits,
            total,
            truncated: false,
        })
    }
}

#[async_trait]
impl MemoryRetrieval for RecordingRetrieval {
    async fn cover_window(
        &self,
        window: &CoverWindowQuery,
        scope: Option<&SourceScope>,
    ) -> Result<RetrievalResponse, MemoryError> {
        self.record("cover_window", scope);
        self.calls
            .lock()
            .expect("calls lock")
            .windows
            .push(window.clone());
        self.page()
    }

    async fn retrieve_source(
        &self,
        query: &SourceRetrievalQuery,
        scope: Option<&SourceScope>,
    ) -> Result<RetrievalResponse, MemoryError> {
        self.record("retrieve_source", scope);
        self.calls
            .lock()
            .expect("calls lock")
            .source_queries
            .push(query.clone());
        self.page()
    }

    async fn retrieve_children(
        &self,
        _node_id: &str,
        _max_depth: u32,
        _query: Option<&str>,
        _limit: Option<usize>,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<RetrievalHit>, MemoryError> {
        self.record("retrieve_children", scope);
        self.answer()
    }

    async fn retrieve_leaves(
        &self,
        _chunk_ids: &[String],
        scope: Option<&SourceScope>,
    ) -> Result<Vec<RetrievalHit>, MemoryError> {
        self.record("retrieve_leaves", scope);
        self.answer()
    }

    // The family's remaining members are not reachable from these handlers.
    // They say so rather than returning a plausible empty value that could
    // make a future test pass for the wrong reason.
    async fn fast_retrieve(
        &self,
        _query: &str,
        _options: FastRetrieveQuery,
        _scope: Option<&SourceScope>,
    ) -> Result<RetrievalResponse, MemoryError> {
        unimplemented!("no retrieval RPC reaches fast_retrieve")
    }

    async fn recall_namespace_scored(
        &self,
        _namespace: &str,
        _query: &str,
        _limit: usize,
        _exclude_session_id: Option<&str>,
    ) -> Result<Vec<NamespaceMemoryHit>, MemoryError> {
        unimplemented!("no retrieval RPC reaches recall_namespace_scored")
    }

    async fn recall_namespace_recent(
        &self,
        _namespace: &str,
        _limit: usize,
    ) -> Result<Vec<NamespaceMemoryHit>, MemoryError> {
        unimplemented!("no retrieval RPC reaches recall_namespace_recent")
    }

    async fn search_entities(
        &self,
        _query: &str,
        _kinds: Option<&[String]>,
        _limit: usize,
    ) -> Result<Vec<EntityMatch>, MemoryError> {
        unimplemented!("search_entities has its own degrade-path tests")
    }
}

// The mandatory three are supertraits of `MemoryProvider`, so a stub cannot
// skip them. Delegated to the null driver: this double exists to observe
// retrieval, and nothing here stores or recalls.
#[async_trait]
impl MemoryCore for RecordingRetrieval {
    async fn store(
        &self,
        namespace: &str,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
        taint: MemoryTaint,
    ) -> Result<(), MemoryError> {
        self.inner
            .store(namespace, key, content, category, session_id, taint)
            .await
    }

    async fn get(&self, namespace: &str, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        self.inner.get(namespace, key).await
    }

    async fn forget(&self, namespace: &str, key: &str) -> Result<bool, MemoryError> {
        self.inner.forget(namespace, key).await
    }

    async fn list(
        &self,
        namespace: Option<&str>,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.inner.list(namespace, category, session_id).await
    }

    async fn namespaces(&self) -> Result<Vec<NamespaceSummary>, MemoryError> {
        self.inner.namespaces().await
    }
}

#[async_trait]
impl MemoryRecall for RecordingRetrieval {
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        opts: &OwnedRecallOpts,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.inner.recall(query, limit, opts, scope).await
    }
}

#[async_trait]
impl MemoryPortability for RecordingRetrieval {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ExportPage, MemoryError> {
        self.inner.export_page(cursor, limit).await
    }

    async fn import_records(
        &self,
        records: Vec<ExportRecord>,
    ) -> Result<ImportOutcome, MemoryError> {
        self.inner.import_records(records).await
    }
}

#[async_trait]
impl MemoryProvider for RecordingRetrieval {
    fn driver_id(&self) -> &str {
        "recording-retrieval"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::all()
    }

    async fn health(&self) -> MemoryHealth {
        MemoryHealth::Ready
    }

    fn as_retrieval(&self) -> Option<&dyn MemoryRetrieval> {
        Some(self)
    }
}

// ── query_source_rpc ──────────────────────────────────────────────

/// An unknown kind is rejected **before** a driver is resolved, which is why
/// this test installs no binding: a caller mistake must not need a working
/// driver to be reported, and must not reach one and come back as an empty
/// store instead.
#[tokio::test]
async fn query_source_rpc_rejects_invalid_source_kind() {
    let (_tmp, cfg) = test_config();
    let req = QuerySourceRequest {
        source_id: None,
        source_kind: Some("bogus".into()),
        time_window_days: None,
        query: None,
        limit: None,
    };
    let err = query_source_rpc(&cfg, req).await.unwrap_err();
    assert!(err.contains("unknown source kind: bogus"), "got {err}");
}

/// The read degrades rather than fails when the bound driver has no
/// retrieval family, and still logs the count it served — a silent empty
/// and a degraded empty look identical downstream otherwise.
#[tokio::test]
async fn query_source_rpc_degrades_to_empty_without_the_retrieval_family() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let outcome = query_source_rpc(&cfg, QuerySourceRequest::default())
        .await
        .expect("a driver without the retrieval family is not an error");
    assert!(outcome.value.hits.is_empty());
    assert_eq!(outcome.value.total, 0);
    assert_eq!(outcome.logs.len(), 1);
    let log = &outcome.logs[0];
    assert!(log.contains("has_source_id=false"), "log: {log}");
    assert!(log.contains("source_kind=None"), "log: {log}");
    assert!(log.contains("has_query=false"), "log: {log}");
    assert!(log.contains("hits=0"), "log: {log}");
}

#[tokio::test]
async fn query_source_rpc_redacts_source_id_from_its_log() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = QuerySourceRequest {
        source_id: Some("slack:#eng".into()),
        source_kind: Some("chat".into()),
        time_window_days: None,
        query: None,
        limit: Some(5),
    };
    let outcome = query_source_rpc(&cfg, req).await.unwrap();
    assert!(outcome.value.hits.is_empty());
    let log = &outcome.logs[0];
    assert!(log.contains("has_source_id=true"), "log: {log}");
    assert!(log.contains("source_kind=Some(\"chat\")"), "log: {log}");
    // PII redaction: the raw source_id must NOT leak into the log.
    assert!(!log.contains("slack:#eng"), "log leaked source_id: {log}");
}

/// The filters reach the contract intact, and so does the turn's source
/// allowlist — the gate this handler is the only thing applying.
#[tokio::test]
async fn query_source_rpc_forwards_its_filters_and_the_turn_scope() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(&cfg, RecordingRetrieval::new());
    let req = QuerySourceRequest {
        source_id: Some("slack:#eng".into()),
        source_kind: Some("chat".into()),
        time_window_days: Some(7),
        query: Some("phoenix".into()),
        limit: Some(5),
    };
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        query_source_rpc(&cfg, req).await.unwrap()
    })
    .await;

    let query = driver.source_query();
    assert_eq!(query.source_id.as_deref(), Some("slack:#eng"));
    assert_eq!(query.source_kind, Some(SourceKind::Chat));
    assert_eq!(query.time_window_days, Some(7));
    assert_eq!(query.query.as_deref(), Some("phoenix"));
    assert_eq!(query.limit, 5);

    let scope = driver
        .scope_for("retrieve_source")
        .expect("retrieve_source was called")
        .expect("a restricted turn must not reach the driver as unrestricted");
    assert_eq!(scope.allow, vec!["slack:#eng".to_string()]);
}

/// Outside a turn there is genuinely no restriction, and that has to travel
/// as `None`: an **empty** `SourceScope` denies every source-attributed row,
/// so mapping "unrestricted" onto one would blank recall out instead.
#[tokio::test]
async fn query_source_rpc_leaves_the_scope_absent_when_unrestricted() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(&cfg, RecordingRetrieval::new());
    query_source_rpc(&cfg, QuerySourceRequest::default())
        .await
        .unwrap();
    assert_eq!(
        driver.scope_for("retrieve_source"),
        Some(None),
        "no ambient allowlist must reach the driver as None, not Some(empty)"
    );
}

/// An absent `limit` stays the engine's default rather than becoming a
/// request for zero rows.
#[tokio::test]
async fn query_source_rpc_maps_an_absent_limit_to_the_engine_sentinel() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(&cfg, RecordingRetrieval::new());
    query_source_rpc(&cfg, QuerySourceRequest::default())
        .await
        .unwrap();
    assert_eq!(driver.source_query().limit, 0);
}

// ── cover_window_rpc ──────────────────────────────────────────────

#[tokio::test]
async fn cover_window_rpc_rejects_invalid_source_kind() {
    let (_tmp, cfg) = test_config();
    let req = CoverWindowRequest {
        since_ms: 0,
        until_ms: 1,
        source_id: None,
        source_kind: Some("bogus".into()),
        limit: None,
    };
    let err = cover_window_rpc(&cfg, req).await.unwrap_err();
    assert!(err.contains("cover_window:"), "got {err}");
    assert!(err.contains("unknown source kind: bogus"), "got {err}");
}

#[tokio::test]
async fn cover_window_rpc_degrades_to_empty_and_redacts_its_log() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = CoverWindowRequest {
        since_ms: 0,
        until_ms: 4_000_000_000_000,
        source_id: Some("slack:#eng".into()),
        source_kind: Some("chat".into()),
        limit: None,
    };
    let outcome = cover_window_rpc(&cfg, req)
        .await
        .expect("a driver without the retrieval family is not an error");
    assert!(outcome.value.hits.is_empty());
    assert_eq!(outcome.value.total, 0);
    assert_eq!(outcome.logs.len(), 1);
    let log = &outcome.logs[0];
    assert!(log.contains("has_source_id=true"), "log: {log}");
    assert!(log.contains("source_kind=Some(\"chat\")"), "log: {log}");
    assert!(log.contains("hits=0"), "log: {log}");
    // PII redaction: the raw source_id must NOT leak into the log.
    assert!(!log.contains("slack:#eng"), "log leaked source_id: {log}");
}

#[tokio::test]
async fn cover_window_rpc_forwards_its_window_and_the_turn_scope() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(&cfg, RecordingRetrieval::new());
    let req = CoverWindowRequest {
        since_ms: 10,
        until_ms: 20,
        source_id: Some("slack:#eng".into()),
        source_kind: Some("chat".into()),
        limit: Some(3),
    };
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        cover_window_rpc(&cfg, req).await.unwrap()
    })
    .await;

    let window = driver.window();
    assert_eq!(window.since_ms, 10);
    assert_eq!(window.until_ms, 20);
    assert_eq!(window.source_id.as_deref(), Some("slack:#eng"));
    assert_eq!(window.source_kind, Some(SourceKind::Chat));
    // Forwarded as sent: the driver, not this handler, maps absence onto
    // the engine's 0 sentinel.
    assert_eq!(window.limit, Some(3));

    let scope = driver
        .scope_for("cover_window")
        .expect("cover_window was called")
        .expect("a restricted turn must not reach the driver as unrestricted");
    assert_eq!(scope.allow, vec!["slack:#eng".to_string()]);
}

/// An inverted window is now the driver's rejection, and it has to arrive as
/// an error rather than as an empty page — the two are indistinguishable to
/// a caller otherwise, and one of them is a bug report.
#[tokio::test]
async fn cover_window_rpc_surfaces_a_driver_rejection() {
    let (_tmp, cfg) = test_config();
    bind_recording(
        &cfg,
        RecordingRetrieval::new().rejecting("until_ms 50 is before since_ms 100"),
    );
    let req = CoverWindowRequest {
        since_ms: 100,
        until_ms: 50,
        source_id: None,
        source_kind: None,
        limit: None,
    };
    let err = cover_window_rpc(&cfg, req).await.unwrap_err();
    assert!(err.contains("cover_window:"), "got {err}");
    assert!(err.contains("until_ms"), "got {err}");
    assert!(err.contains("since_ms"), "got {err}");
}

/// The source gate, end to end through the real driver.
///
/// The tests above prove the handler *passes* a scope; this one proves a
/// restricted profile cannot read a source it was not granted. It has to
/// bind the in-process driver rather than the double, because the filtering
/// is the engine's — and `binding.provider()` is unguarded, so the scope
/// this handler passes is the only thing standing between the two.
#[tokio::test]
async fn cover_window_rpc_honors_profile_source_scope() {
    let (_tmp, cfg) = test_config();
    // Two memory-source chunks in different sources, both inside the window.
    let mut allowed = sample_chunk("slack:#eng", 0);
    allowed.metadata.tags = vec!["memory_sources".into(), "chat".into()];
    let mut blocked = sample_chunk("slack:#secret", 0);
    blocked.metadata.tags = vec!["memory_sources".into(), "chat".into()];
    upsert_chunks(&cfg, &[allowed.clone(), blocked.clone()]).unwrap();
    stage_test_chunks(&cfg, &[allowed.clone(), blocked.clone()]);
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);

    let req = || CoverWindowRequest {
        since_ms: 0,
        until_ms: 4_000_000_000_000,
        source_id: None,
        source_kind: None,
        limit: None,
    };

    let resp = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        cover_window_rpc(&cfg, req()).await
    })
    .await
    .unwrap();
    let ids: Vec<&str> = resp.value.hits.iter().map(|h| h.node_id.as_str()).collect();
    assert!(
        ids.contains(&allowed.id.as_str()),
        "allowlisted source must be present: {ids:?}"
    );
    assert!(
        !ids.contains(&blocked.id.as_str()),
        "disallowed source must be filtered out: {ids:?}"
    );

    // With no profile scope active, both sources are visible — which is what
    // makes the assertion above a filter rather than an empty store.
    let unrestricted = cover_window_rpc(&cfg, req()).await.unwrap();
    assert_eq!(unrestricted.value.hits.len(), 2);
}

// ── search_entities_rpc ───────────────────────────────────────────

/// The search degrades rather than fails when the bound driver has no
/// retrieval family, and still logs the count it served — a silent empty
/// and a degraded empty look identical downstream otherwise.
#[tokio::test]
async fn search_entities_rpc_passes_through_kinds_none() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = SearchEntitiesRequest {
        query: "alice".into(),
        kinds: None,
        limit: None,
    };
    let outcome = search_entities_rpc(&cfg, req)
        .await
        .expect("a driver without the retrieval family is not an error");
    assert!(outcome.value.matches.is_empty());
    let log = &outcome.logs[0];
    assert!(log.contains("query_len=5"), "log: {log}");
    assert!(log.contains("has_kinds=false"), "log: {log}");
    assert!(log.contains("n=0"), "log: {log}");
    // PII redaction — the raw query value must NOT appear in the log.
    assert!(!log.contains("alice"), "log leaked raw query: {log}");
}

#[tokio::test]
async fn search_entities_rpc_parses_valid_kinds_list() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = SearchEntitiesRequest {
        query: "x".into(),
        kinds: Some(vec!["email".into(), "topic".into()]),
        limit: Some(10),
    };
    let outcome = search_entities_rpc(&cfg, req).await.unwrap();
    assert!(outcome.value.matches.is_empty());
    assert!(
        outcome.logs[0].contains("has_kinds=true"),
        "log: {}",
        outcome.logs[0]
    );
}

/// An unknown kind is rejected **before** a driver is resolved, which is why
/// this test installs no binding: a caller mistake must not need a working
/// driver to be reported, and must not reach one and come back as an empty
/// index instead.
#[tokio::test]
async fn search_entities_rpc_rejects_unknown_entity_kind() {
    let (_tmp, cfg) = test_config();
    let req = SearchEntitiesRequest {
        query: "x".into(),
        kinds: Some(vec!["email".into(), "bogus".into()]),
        limit: None,
    };
    let err = search_entities_rpc(&cfg, req).await.unwrap_err();
    assert!(err.contains("unknown entity kind: bogus"), "got {err}");
}

// ── drill_down_rpc ────────────────────────────────────────────────

#[tokio::test]
async fn drill_down_rpc_defaults_max_depth_to_one_when_unset() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = DrillDownRequest {
        node_id: "chat:missing".into(),
        max_depth: None,
        query: None,
        limit: None,
    };
    let outcome = drill_down_rpc(&cfg, req).await.unwrap();
    assert!(
        outcome.logs[0].contains("depth=1"),
        "log: {}",
        outcome.logs[0]
    );
}

#[tokio::test]
async fn drill_down_rpc_logs_node_kind_prefix_for_colon_separated_id() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = DrillDownRequest {
        node_id: "chat:slack:#eng:0".into(),
        max_depth: Some(2),
        query: None,
        limit: None,
    };
    let outcome = drill_down_rpc(&cfg, req).await.unwrap();
    let log = &outcome.logs[0];
    assert!(log.contains("node_kind=chat"), "log: {log}");
    // PII redaction — scope segments beyond the kind prefix must not leak.
    assert!(!log.contains("slack"), "log leaked scope: {log}");
    assert!(!log.contains("#eng"), "log leaked scope: {log}");
}

#[tokio::test]
async fn drill_down_rpc_logs_unknown_when_node_id_has_no_colon() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = DrillDownRequest {
        node_id: "rootnode".into(),
        max_depth: None,
        query: None,
        limit: None,
    };
    let outcome = drill_down_rpc(&cfg, req).await.unwrap();
    assert!(
        outcome.logs[0].contains("node_kind=unknown"),
        "log: {}",
        outcome.logs[0]
    );
}

/// A node id names a node, not a permission: the walk still has to run under
/// the turn's allowlist.
#[tokio::test]
async fn drill_down_rpc_forwards_the_turn_scope() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(&cfg, RecordingRetrieval::new());
    let req = DrillDownRequest {
        node_id: "chat:slack:#eng:0".into(),
        max_depth: None,
        query: None,
        limit: None,
    };
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        drill_down_rpc(&cfg, req).await.unwrap()
    })
    .await;
    let scope = driver
        .scope_for("retrieve_children")
        .expect("retrieve_children was called")
        .expect("a restricted turn must not reach the driver as unrestricted");
    assert_eq!(scope.allow, vec!["slack:#eng".to_string()]);
}

// ── fetch_leaves_rpc ──────────────────────────────────────────────

#[tokio::test]
async fn fetch_leaves_rpc_returns_empty_response_for_empty_input() {
    let (_tmp, cfg) = test_config();
    bind_without_retrieval(&cfg);
    let req = FetchLeavesRequest { chunk_ids: vec![] };
    let outcome = fetch_leaves_rpc(&cfg, req).await.unwrap();
    assert!(outcome.value.hits.is_empty());
    assert!(outcome.logs[0].contains("n=0"), "log: {}", outcome.logs[0]);
}

/// Naming chunk ids directly must not read around the source gate, so the
/// scope travels with the ids.
#[tokio::test]
async fn fetch_leaves_rpc_returns_driver_hits_under_the_turn_scope() {
    let (_tmp, cfg) = test_config();
    let driver = bind_recording(
        &cfg,
        RecordingRetrieval::new().answering(vec![hit("chunk-a"), hit("chunk-b")]),
    );
    let req = FetchLeavesRequest {
        chunk_ids: vec!["chunk-a".into(), "chunk-b".into(), "ghost".into()],
    };
    let outcome = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        fetch_leaves_rpc(&cfg, req).await.unwrap()
    })
    .await;
    assert_eq!(outcome.value.hits.len(), 2);
    assert!(outcome.logs[0].contains("n=2"), "log: {}", outcome.logs[0]);

    let scope = driver
        .scope_for("retrieve_leaves")
        .expect("retrieve_leaves was called")
        .expect("a restricted turn must not reach the driver as unrestricted");
    assert_eq!(scope.allow, vec!["slack:#eng".to_string()]);
}

/// `tree_kind` is `skip_serializing_if = "Option::is_none"`, so a hit that
/// lost its kind on the way through would take the key out of the response
/// rather than fail — the silent field loss that held this migration back
/// while the artifact predated the field.
#[tokio::test]
async fn retrieval_hits_keep_tree_kind_on_the_wire() {
    let (_tmp, cfg) = test_config();
    bind_recording(
        &cfg,
        RecordingRetrieval::new().answering(vec![hit("chunk-a")]),
    );
    let outcome = fetch_leaves_rpc(
        &cfg,
        FetchLeavesRequest {
            chunk_ids: vec!["chunk-a".into()],
        },
    )
    .await
    .unwrap();
    let json = serde_json::to_value(&outcome.value).unwrap();
    assert_eq!(json["hits"][0]["tree_kind"], serde_json::json!("source"));
}
