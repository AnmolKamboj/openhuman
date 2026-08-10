//! Characterization tests for the THREE distinct `source_scope` predicates.
//!
//! These pin **current** behaviour — including behaviour that looks wrong. Do
//! not "fix" anything asserted here: a failure means a refactor changed one of
//! the predicates, which is exactly what these tests exist to catch.
//!
//! 1. `fetch.rs` → `source_scope::chunk_source_allowed_in`: fail-OPEN for
//!    chunks without the `memory_sources` tag, otherwise equality on
//!    `source_id` OR the `mem_src:{id}:` composite rule via
//!    `sync_events::extract_mem_src_id` (which returns `None` for an EMPTY
//!    item id, so `mem_src:src-abc:` is BLOCKED host-side).
//! 2. `source.rs` / `drill_down.rs` → `hits.retain(|h| set.contains(&h.tree_scope))`:
//!    PLAIN EQUALITY on a DIFFERENT field. No tag fail-open, no `mem_src:`
//!    prefix rule. For leaf hits `tree_scope` *is* the chunk's `source_id`
//!    (`tinycortex` `retrieval::{fetch,drill_down}`), so on leaves this is
//!    strictly narrower than predicate 1. `source.rs` / `cover.rs` additionally
//!    carry a *pre-filter* short circuit on the explicit `source_id` argument.
//! 3. `tinycortex::memory::chunks::store_list::append_source_scope` — a SQL
//!    predicate applied BEFORE `LIMIT`. Reached via `cover_window_scoped` and
//!    the raw `list_chunks` callers. It admits `mem_src:src-abc:` (empty item
//!    id), diverging from predicate 1.
//!
//! Note: `fast_retrieve` does NOT reach predicate 3 — it threads the scope into
//! `resolve_local` / `dense`, which apply the predicate-2 `tree_scope` retain.

#![cfg(test)]

use std::collections::HashSet;

use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use crate::openhuman::config::Config;
use crate::openhuman::memory::source_scope::{chunk_source_allowed_in, with_source_scope};
use crate::openhuman::memory::store::chunks::store::{
    list_chunks, upsert_chunks, upsert_staged_chunks_tx, with_connection, ListChunksQuery,
};
use crate::openhuman::memory::store::chunks::types::{
    chunk_id, Chunk, Metadata, SourceKind, SourceRef,
};
use crate::openhuman::memory::store::content as content_store;
use crate::openhuman::memory::store::trees::store::{insert_summary_tx, insert_tree};
use crate::openhuman::memory::store::trees::types::{SummaryNode, Tree, TreeKind, TreeStatus};
use crate::openhuman::memory::tree::retrieval::{
    cover_window, drill_down, fetch_leaves, query_source,
};

const BASE_MS: i64 = 1_700_000_000_000;
const MEMORY_SOURCES: &str = "memory_sources";

// ── fixtures ─────────────────────────────────────────────────────────────

fn test_config() -> (TempDir, Config) {
    let tmp = TempDir::new().unwrap();
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    // Inert embedder keeps these deterministic and avoids any real provider
    // call. Every retrieval call below passes `query: None`, so no embedder is
    // ever built.
    cfg.memory_tree.embedding_endpoint = None;
    cfg.memory_tree.embedding_model = None;
    cfg.memory_tree.embedding_strict = false;
    (tmp, cfg)
}

/// A chunk in `source`, tagged with `tags`, timestamped `ts_ms`.
fn src_chunk(source: &str, seq: u32, tags: &[&str], ts_ms: i64) -> Chunk {
    let ts = Utc.timestamp_millis_opt(ts_ms).unwrap();
    Chunk {
        id: chunk_id(SourceKind::Chat, source, seq, "test-content"),
        content: format!("content-{source}-{seq}"),
        metadata: Metadata {
            source_kind: SourceKind::Chat,
            source_id: source.into(),
            owner: "alice".into(),
            timestamp: ts,
            time_range: (ts, ts),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
            source_ref: Some(SourceRef::new(format!("slack://{source}/{seq}"))),
            path_scope: None,
        },
        token_count: 20,
        seq_in_source: seq,
        created_at: ts,
        partial_message: false,
    }
}

/// Persist chunk rows AND their staged content bodies, mirroring `rpc.rs`.
fn seed_chunks(cfg: &Config, chunks: &[Chunk]) {
    upsert_chunks(cfg, chunks).expect("upsert_chunks");
    let content_root = cfg.memory_tree_content_root();
    std::fs::create_dir_all(&content_root).expect("create content_root for test");
    let staged = content_store::stage_chunks(&content_root, chunks).expect("stage_chunks");
    with_connection(cfg, |conn| {
        let tx = conn.unchecked_transaction()?;
        upsert_staged_chunks_tx(&tx, &staged)?;
        tx.commit()?;
        Ok(())
    })
    .expect("persist staged chunk pointers");
}

fn seed_tree(cfg: &Config, id: &str, scope: &str, root_id: &str, max_level: u32) {
    let ts = Utc.timestamp_millis_opt(BASE_MS).unwrap();
    let tree = Tree {
        id: id.to_string(),
        kind: TreeKind::Source,
        scope: scope.to_string(),
        ask: None,
        root_id: Some(root_id.to_string()),
        max_level,
        status: TreeStatus::Active,
        created_at: ts,
        last_sealed_at: Some(ts),
    };
    insert_tree(cfg, &tree).expect("insert_tree");
}

fn seed_summary(cfg: &Config, id: &str, tree_id: &str, level: u32, children: &[&str]) {
    let ts = Utc.timestamp_millis_opt(BASE_MS).unwrap();
    let node = SummaryNode {
        id: id.to_string(),
        tree_id: tree_id.to_string(),
        tree_kind: TreeKind::Source,
        level,
        parent_id: None,
        child_ids: children.iter().map(|c| (*c).to_string()).collect(),
        content: format!("seal-{id}"),
        token_count: 100,
        entities: vec![],
        topics: vec![],
        time_range_start: ts,
        time_range_end: ts,
        score: 0.5,
        sealed_at: ts,
        deleted: false,
        embedding: None,
        doc_id: None,
        version_ms: None,
    };
    with_connection(cfg, |conn| {
        let tx = conn.unchecked_transaction()?;
        insert_summary_tx(&tx, &node, None, "test")?;
        tx.commit()?;
        Ok(())
    })
    .expect("insert summary");
}

fn set_of(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn scoped_query(scope: Option<&[&str]>) -> ListChunksQuery {
    ListChunksQuery {
        source_scope: scope.map(set_of),
        exclude_dropped: false,
        ..Default::default()
    }
}

fn ids_of(chunks: &[Chunk]) -> Vec<String> {
    chunks.iter().map(|c| c.id.clone()).collect()
}

// ═════════════════════════════════════════════════════════════════════════
// Group 1 — predicate 1: `chunk_source_allowed_in`, via `fetch_leaves`.
// ═════════════════════════════════════════════════════════════════════════

/// Every group-1 fixture at once: one chunk per interesting source shape.
fn group1_chunks() -> Vec<Chunk> {
    vec![
        // Untagged → fail-open under predicate 1.
        src_chunk("gmail:alice", 0, &[], BASE_MS),
        // Tagged, exact source-id match.
        src_chunk("slack:#eng", 1, &[MEMORY_SOURCES], BASE_MS + 1_000),
        // Tagged, `mem_src:` composite with a non-empty item id.
        src_chunk(
            "mem_src:src-abc:item-1",
            2,
            &[MEMORY_SOURCES],
            BASE_MS + 2_000,
        ),
        // Tagged, longer registry id — must NOT be smeared into by `src-abc`.
        src_chunk(
            "mem_src:src-abcdef:item-1",
            3,
            &[MEMORY_SOURCES],
            BASE_MS + 3_000,
        ),
        // Tagged, EMPTY item id — `extract_mem_src_id` returns None here.
        src_chunk("mem_src:src-abc:", 4, &[MEMORY_SOURCES], BASE_MS + 4_000),
    ]
}

async fn fetch_ids_under(
    cfg: &Config,
    chunks: &[Chunk],
    scope: Option<Vec<String>>,
) -> Vec<String> {
    let ids = ids_of(chunks);
    let hits = with_source_scope(scope, async { fetch_leaves(cfg, &ids).await })
        .await
        .expect("fetch_leaves");
    hits.into_iter().map(|h| h.node_id).collect()
}

#[tokio::test]
async fn fetch_leaves_fails_open_for_untagged_chunk() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = fetch_ids_under(&cfg, &chunks, Some(vec!["src-abc".into()])).await;
    assert!(
        got.contains(&chunks[0].id),
        "untagged chunk must fail OPEN through predicate 1: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_allows_exact_source_id_match() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = fetch_ids_under(&cfg, &chunks, Some(vec!["slack:#eng".into()])).await;
    assert!(
        got.contains(&chunks[1].id),
        "exact source_id match: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_allows_mem_src_prefix_match() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = fetch_ids_under(&cfg, &chunks, Some(vec!["src-abc".into()])).await;
    assert!(
        got.contains(&chunks[2].id),
        "mem_src:src-abc:item-1 must resolve to registry id src-abc: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_prefix_does_not_smear_to_longer_source_id() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = fetch_ids_under(&cfg, &chunks, Some(vec!["src-abc".into()])).await;
    assert!(
        !got.contains(&chunks[3].id),
        "src-abc must not smear into src-abcdef: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_blocks_mem_src_with_empty_item_id() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    // `extract_mem_src_id` bails when nothing follows the registry-id colon
    // (`colon_pos + 1 >= rest.len()`), so the composite never resolves and the
    // tagged chunk is blocked — even though the SQL predicate admits it (see
    // `list_chunks_scope_admits_empty_item_id_unlike_the_host_predicate`).
    let set = set_of(&["src-abc"]);
    let tags = vec![MEMORY_SOURCES.to_string()];
    assert!(!chunk_source_allowed_in(&set, &tags, "mem_src:src-abc:"));

    let got = fetch_ids_under(&cfg, &chunks, Some(vec!["src-abc".into()])).await;
    assert!(
        !got.contains(&chunks[4].id),
        "empty-item-id composite must be blocked host-side: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_empty_allowlist_blocks_tagged_but_not_untagged() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = fetch_ids_under(&cfg, &chunks, Some(vec![])).await;
    assert_eq!(
        got,
        vec![chunks[0].id.clone()],
        "an empty allowlist keeps only the fail-open untagged chunk: {got:?}"
    );
}

#[tokio::test]
async fn fetch_leaves_without_scope_returns_every_chunk() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let ids = ids_of(&chunks);
    let hits = fetch_leaves(&cfg, &ids).await.expect("fetch_leaves");
    assert_eq!(hits.len(), chunks.len(), "absent scope is unrestricted");
}

// ═════════════════════════════════════════════════════════════════════════
// Group 2 — predicate 2: plain equality on `tree_scope`.
// ═════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn query_source_retains_only_exact_tree_scope_matches() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-eng", 1);
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-secret", 1);
    seed_summary(&cfg, "s-eng", "tree-eng", 1, &["leaf-a"]);
    seed_summary(&cfg, "s-secret", "tree-secret", 1, &["leaf-b"]);

    let resp = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        query_source(&cfg, None, None, None, None, 10).await
    })
    .await
    .expect("query_source");

    assert_eq!(resp.hits.len(), 1, "hits: {:?}", resp.hits);
    assert_eq!(resp.hits[0].tree_scope, "slack:#eng");
    assert_eq!(resp.hits[0].node_id, "s-eng");
}

#[tokio::test]
async fn query_source_tree_scope_filter_has_no_mem_src_prefix_rule() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-m", "mem_src:src-abc:item-1", "s-m", 1);
    seed_summary(&cfg, "s-m", "tree-m", 1, &["leaf-a"]);

    // Predicate 1 WOULD admit this identifier…
    let set = set_of(&["src-abc"]);
    let tags = vec![MEMORY_SOURCES.to_string()];
    assert!(chunk_source_allowed_in(
        &set,
        &tags,
        "mem_src:src-abc:item-1"
    ));

    // …but predicate 2 is plain equality on `tree_scope`, so it does not.
    let resp = with_source_scope(Some(vec!["src-abc".into()]), async {
        query_source(&cfg, None, None, None, None, 10).await
    })
    .await
    .expect("query_source");
    assert!(
        resp.hits.is_empty(),
        "tree_scope retain has no mem_src rule: {:?}",
        resp.hits
    );
}

#[tokio::test]
async fn query_source_empty_allowlist_returns_no_hits() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-eng", 1);
    seed_summary(&cfg, "s-eng", "tree-eng", 1, &["leaf-a"]);

    let resp = with_source_scope(Some(vec![]), async {
        query_source(&cfg, None, None, None, None, 10).await
    })
    .await
    .expect("query_source");
    assert!(resp.hits.is_empty());
    assert_eq!(resp.total, 0);
}

#[tokio::test]
async fn query_source_without_scope_returns_every_tree() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-eng", 1);
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-secret", 1);
    seed_summary(&cfg, "s-eng", "tree-eng", 1, &["leaf-a"]);
    seed_summary(&cfg, "s-secret", "tree-secret", 1, &["leaf-b"]);

    let resp = query_source(&cfg, None, None, None, None, 10)
        .await
        .expect("query_source");
    assert_eq!(resp.hits.len(), 2, "absent scope is unrestricted");
}

#[tokio::test]
async fn query_source_explicit_source_id_outside_scope_short_circuits() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-secret", 1);
    seed_summary(&cfg, "s-secret", "tree-secret", 1, &["leaf-b"]);

    // The `source.rs` PRE-filter: plain equality on the request argument,
    // returning `QueryResponse::empty()` before the engine is even called.
    // This is a fourth predicate, distinct from the post-filter retain.
    let resp = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        query_source(&cfg, Some("slack:#secret"), None, None, None, 10).await
    })
    .await
    .expect("query_source");
    assert!(resp.hits.is_empty());
    assert_eq!(resp.total, 0);
    assert!(!resp.truncated);
}

#[tokio::test]
async fn drill_down_retains_only_exact_tree_scope_matches() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-root", 2);
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-b", 1);
    seed_summary(&cfg, "s-root", "tree-eng", 2, &["s-a", "s-b"]);
    seed_summary(&cfg, "s-a", "tree-eng", 1, &["leaf-a"]);
    seed_summary(&cfg, "s-b", "tree-secret", 1, &["leaf-b"]);

    let hits = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        drill_down(&cfg, "s-root", 1, None, None).await
    })
    .await
    .expect("drill_down");

    let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
    assert_eq!(ids, vec!["s-a"], "hits: {ids:?}");
}

#[tokio::test]
async fn drill_down_without_scope_keeps_every_hit() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-root", 2);
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-b", 1);
    seed_summary(&cfg, "s-root", "tree-eng", 2, &["s-a", "s-b"]);
    seed_summary(&cfg, "s-a", "tree-eng", 1, &["leaf-a"]);
    seed_summary(&cfg, "s-b", "tree-secret", 1, &["leaf-b"]);

    let hits = drill_down(&cfg, "s-root", 1, None, None)
        .await
        .expect("drill_down");
    let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
    assert_eq!(ids, vec!["s-a", "s-b"], "hits: {ids:?}");
}

#[tokio::test]
async fn drill_down_chunk_leaves_are_scoped_by_source_id_not_by_tag() {
    let (_tmp, cfg) = test_config();
    // An UNTAGGED chunk and a TAGGED `mem_src:` chunk hanging off one L1 node.
    let untagged = src_chunk("gmail:alice", 0, &[], BASE_MS);
    let tagged = src_chunk(
        "mem_src:src-abc:item-1",
        1,
        &[MEMORY_SOURCES],
        BASE_MS + 1_000,
    );
    seed_chunks(&cfg, &[untagged.clone(), tagged.clone()]);
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-leaves", 1);
    seed_summary(
        &cfg,
        "s-leaves",
        "tree-eng",
        1,
        &[untagged.id.as_str(), tagged.id.as_str()],
    );

    // Leaves carry `tree_scope = chunk.metadata.source_id`, so an allowlist
    // naming that source id keeps the chunk — with NO tag fail-open for the
    // untagged one, which is why the tagged sibling drops out here.
    let hits = with_source_scope(Some(vec!["gmail:alice".into()]), async {
        drill_down(&cfg, "s-leaves", 1, None, None).await
    })
    .await
    .expect("drill_down");
    let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
    assert_eq!(ids, vec![untagged.id.as_str()], "hits: {ids:?}");

    // And the `mem_src:` prefix rule does NOT apply on this path either:
    // predicate 1 would admit `mem_src:src-abc:item-1` under `src-abc`.
    let hits = with_source_scope(Some(vec!["src-abc".into()]), async {
        drill_down(&cfg, "s-leaves", 1, None, None).await
    })
    .await
    .expect("drill_down");
    assert!(
        hits.is_empty(),
        "leaf retain is plain equality on source_id: {hits:?}"
    );
}

#[tokio::test]
async fn drill_down_scope_widens_engine_limit_so_a_blocked_prefix_cannot_starve_results() {
    let (_tmp, cfg) = test_config();
    seed_tree(&cfg, "tree-eng", "slack:#eng", "s-root", 2);
    seed_tree(&cfg, "tree-secret", "slack:#secret", "s-b1", 1);
    // BFS order puts the two blocked children FIRST.
    seed_summary(&cfg, "s-root", "tree-eng", 2, &["s-b1", "s-b2", "s-a"]);
    seed_summary(&cfg, "s-b1", "tree-secret", 1, &["leaf-1"]);
    seed_summary(&cfg, "s-b2", "tree-secret", 1, &["leaf-2"]);
    seed_summary(&cfg, "s-a", "tree-eng", 1, &["leaf-3"]);

    // `drill_down.rs` forces the ENGINE limit to `None` whenever a scope is
    // active, then applies the caller's limit after the retain. Without that,
    // the engine would return only `s-b1` and the retain would empty it.
    let hits = with_source_scope(Some(vec!["slack:#eng".into()]), async {
        drill_down(&cfg, "s-root", 1, None, Some(1)).await
    })
    .await
    .expect("drill_down");
    let ids: Vec<&str> = hits.iter().map(|h| h.node_id.as_str()).collect();
    assert_eq!(ids, vec!["s-a"], "hits: {ids:?}");
}

// ═════════════════════════════════════════════════════════════════════════
// Group 3 — predicate 3: the SQL `append_source_scope`, applied before LIMIT.
// ═════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_chunks_scope_fails_open_for_untagged_chunk() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(Some(&["src-abc"]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert!(
        ids.contains(&chunks[0].id.as_str()),
        "SQL `NOT EXISTS json_each(...)` fail-open: {ids:?}"
    );
}

#[tokio::test]
async fn list_chunks_scope_matches_exact_source_id() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(Some(&["slack:#eng"]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&chunks[1].id.as_str()), "ids: {ids:?}");
}

#[tokio::test]
async fn list_chunks_scope_matches_mem_src_prefix() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(Some(&["src-abc"]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&chunks[2].id.as_str()), "ids: {ids:?}");
}

#[tokio::test]
async fn list_chunks_scope_does_not_smear_prefix() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(Some(&["src-abc"]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert!(
        !ids.contains(&chunks[3].id.as_str()),
        "substr(source_id, 1, length('mem_src:src-abc:')) must not match \
         mem_src:src-abcdef:item-1: {ids:?}"
    );
}

#[tokio::test]
async fn list_chunks_scope_admits_empty_item_id_unlike_the_host_predicate() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    // THE headline divergence, characterized as observed (not endorsed):
    // the SQL prefix test is a pure `substr` compare with no "item id must be
    // non-empty" rule, so `mem_src:src-abc:` passes here…
    let got = list_chunks(&cfg, &scoped_query(Some(&["src-abc"]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert!(
        ids.contains(&chunks[4].id.as_str()),
        "SQL admits mem_src:src-abc: : {ids:?}"
    );

    // …while the host predicate blocks the very same source_id.
    let set = set_of(&["src-abc"]);
    let tags = vec![MEMORY_SOURCES.to_string()];
    assert!(!chunk_source_allowed_in(&set, &tags, "mem_src:src-abc:"));
}

#[tokio::test]
async fn list_chunks_empty_allowlist_keeps_only_untagged_chunks() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(Some(&[]))).expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec![chunks[0].id.as_str()], "ids: {ids:?}");
}

#[tokio::test]
async fn list_chunks_absent_scope_returns_everything() {
    let (_tmp, cfg) = test_config();
    let chunks = group1_chunks();
    seed_chunks(&cfg, &chunks);

    let got = list_chunks(&cfg, &scoped_query(None)).expect("list_chunks");
    assert_eq!(got.len(), chunks.len());
}

#[tokio::test]
async fn list_chunks_scope_is_applied_before_limit() {
    let (_tmp, cfg) = test_config();
    // Three blocked chunks NEWER than the single allowed one. Ordering is
    // `timestamp_ms DESC`, so a post-filter with LIMIT 1 would return nothing.
    let blocked: Vec<Chunk> = (0..3)
        .map(|i| {
            src_chunk(
                "slack:#secret",
                i,
                &[MEMORY_SOURCES],
                BASE_MS + 10_000 + i64::from(i) * 1_000,
            )
        })
        .collect();
    let allowed = src_chunk("slack:#eng", 9, &[MEMORY_SOURCES], BASE_MS);
    let mut all = blocked.clone();
    all.push(allowed.clone());
    seed_chunks(&cfg, &all);

    let got = list_chunks(
        &cfg,
        &ListChunksQuery {
            source_scope: Some(set_of(&["slack:#eng"])),
            limit: Some(1),
            exclude_dropped: false,
            ..Default::default()
        },
    )
    .expect("list_chunks");
    let ids: Vec<&str> = got.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec![allowed.id.as_str()], "ids: {ids:?}");
}

#[tokio::test]
async fn cover_window_scope_matches_mem_src_prefix() {
    let (_tmp, cfg) = test_config();
    let allowed = src_chunk("mem_src:src-abc:item-1", 0, &[MEMORY_SOURCES], BASE_MS);
    let blocked = src_chunk(
        "mem_src:src-zzz:item-1",
        1,
        &[MEMORY_SOURCES],
        BASE_MS + 1_000,
    );
    seed_chunks(&cfg, &[allowed.clone(), blocked.clone()]);

    // `cover_window` hands the allowlist straight to `cover_window_scoped`,
    // which applies predicate 3 in SQL — so the `mem_src:` prefix rule holds
    // here, unlike on the `tree_scope` paths above.
    let resp = with_source_scope(Some(vec!["src-abc".into()]), async {
        cover_window(&cfg, 0, 4_000_000_000_000, None, None, 0).await
    })
    .await
    .expect("cover_window");
    let ids: Vec<&str> = resp.hits.iter().map(|h| h.node_id.as_str()).collect();
    assert!(ids.contains(&allowed.id.as_str()), "ids: {ids:?}");
    assert!(!ids.contains(&blocked.id.as_str()), "ids: {ids:?}");
}
