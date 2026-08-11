//! [`MemoryTree`] tests for the embedded driver.
//!
//! The scope tests below deliberately mirror
//! `tree::retrieval::source_scope_tests`' predicate-3 cases, but reach them
//! *through the driver*. Those 25 characterization tests keep asserting the
//! host predicate directly and are untouched; these assert that routing a
//! contract `SourceScope` into `ListChunksQuery.source_scope` preserves it.

use super::super::test_support::fresh_driver;

use chrono::{TimeZone, Utc};
use tinycortex_api::provider::types::SourceScope;
use tinycortex_api::provider::MemoryTree;
use tinycortex_api::tree::IngestRequest;

use crate::openhuman::config::Config;
use crate::openhuman::memory::store::chunks::store::{
    upsert_chunks, upsert_staged_chunks_tx, with_connection,
};
use crate::openhuman::memory::store::chunks::types::{
    chunk_id, Chunk, Metadata, SourceKind, SourceRef,
};
use crate::openhuman::memory::store::content as content_store;

const BASE_MS: i64 = 1_700_000_000_000;
const MEMORY_SOURCES: &str = "memory_sources";

fn request(namespace: &str, content: &str) -> IngestRequest {
    IngestRequest {
        namespace: namespace.to_string(),
        content: content.to_string(),
        timestamp: Some(Utc.timestamp_millis_opt(BASE_MS).unwrap()),
        metadata: None,
    }
}

/// A chunk in `source`, tagged with `tags`, timestamped `ts_ms`. Same shape as
/// the `source_scope_tests` fixture so the two suites stay comparable.
fn src_chunk(source: &str, seq: u32, tags: &[&str], ts_ms: i64) -> Chunk {
    let ts = Utc.timestamp_millis_opt(ts_ms).unwrap();
    Chunk {
        id: chunk_id(SourceKind::Chat, source, seq, "driver-content"),
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

fn seed_chunks(config: &Config, chunks: &[Chunk]) {
    upsert_chunks(config, chunks).expect("upsert_chunks");
    let content_root = config.memory_tree_content_root();
    std::fs::create_dir_all(&content_root).expect("create content_root");
    let staged = content_store::stage_chunks(&content_root, chunks).expect("stage_chunks");
    with_connection(config, |conn| {
        let tx = conn.unchecked_transaction()?;
        upsert_staged_chunks_tx(&tx, &staged)?;
        tx.commit()?;
        Ok(())
    })
    .expect("persist staged chunk pointers");
}

// ── append ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn tree_append_buffers_content_for_namespace() {
    let (_tmp, provider) = fresh_driver();
    provider
        .append(request("work", "phoenix launch is friday"))
        .await
        .expect("append");

    let config = provider.config().await.expect("config");
    let buffered = crate::openhuman::memory::tree::tree_runtime::store::buffer_read(config, "work")
        .expect("buffer_read");
    assert_eq!(buffered.len(), 1, "one buffered entry");
    assert!(
        buffered[0].1.contains("phoenix launch is friday"),
        "buffered body must carry the content, got {:?}",
        buffered[0].1
    );
}

#[tokio::test]
async fn tree_append_rejects_empty_content_as_invalid() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .append(request("work", "   \n  "))
        .await
        .expect_err("whitespace-only content must be refused");
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::Invalid(_)),
        "expected Invalid, got {error:?}"
    );
}

#[tokio::test]
async fn tree_append_rejects_traversing_namespace_as_invalid() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .append(request("../escape", "body"))
        .await
        .expect_err("traversal namespace must be refused");
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::Invalid(_)),
        "expected Invalid, got {error:?}"
    );
}

// ── drill_down ───────────────────────────────────────────────────────────

#[tokio::test]
async fn tree_drill_down_unknown_node_is_not_found() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .drill_down("work", "2024/03/15/09")
        .await
        .expect_err("an absent node must not be Ok");
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::NotFound(_)),
        "the contract mandates NotFound here, got {error:?}"
    );
}

#[tokio::test]
async fn tree_drill_down_returns_node_with_direct_children() {
    use crate::openhuman::memory::tree::tree_runtime::store::write_node;
    use tinycortex::memory::tree::runtime::{NodeLevel, TreeNode};

    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    let ts = Utc.timestamp_millis_opt(BASE_MS).unwrap();

    let node = |node_id: &str, level: NodeLevel, parent: Option<&str>| TreeNode {
        node_id: node_id.to_string(),
        namespace: "work".to_string(),
        level,
        parent_id: parent.map(str::to_string),
        summary: format!("summary for {node_id}"),
        token_count: 10,
        child_count: 0,
        created_at: ts,
        updated_at: ts,
        metadata: None,
    };

    write_node(&config, &node("2024", NodeLevel::Year, Some("root"))).expect("write year");
    write_node(&config, &node("2024/03", NodeLevel::Month, Some("2024"))).expect("write month");

    let result = provider
        .drill_down("work", "2024")
        .await
        .expect("drill_down");
    assert_eq!(result.node.node_id, "2024");
    assert_eq!(
        result
            .children
            .iter()
            .map(|child| child.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["2024/03"],
    );
}

#[tokio::test]
async fn tree_drill_down_rejects_traversing_node_id_as_invalid() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .drill_down("work", "../../etc")
        .await
        .expect_err("traversal node id must be refused");
    assert!(
        matches!(error, tinycortex_api::error::MemoryError::Invalid(_)),
        "expected Invalid, got {error:?}"
    );
}

// ── query_source + scope ─────────────────────────────────────────────────

#[tokio::test]
async fn tree_query_source_returns_that_sources_chunks_newest_first() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_chunks(
        &config,
        &[
            src_chunk("src-abc", 1, &[], BASE_MS),
            src_chunk("src-abc", 2, &[], BASE_MS + 1_000),
            src_chunk("src-xyz", 1, &[], BASE_MS + 2_000),
        ],
    );

    let hits = provider
        .query_source("work", "src-abc", 10, None)
        .await
        .expect("query_source");

    assert_eq!(hits.len(), 2, "only src-abc's chunks");
    assert!(
        hits[0].metadata.timestamp >= hits[1].metadata.timestamp,
        "newest first"
    );
}

#[tokio::test]
async fn tree_query_source_unknown_source_is_empty_not_an_error() {
    let (_tmp, provider) = fresh_driver();
    let hits = provider
        .query_source("work", "src-nope", 10, None)
        .await
        .expect("an unknown source must yield an empty vector, not an error");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn tree_query_source_scope_admits_the_listed_source() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_chunks(
        &config,
        &[src_chunk("src-abc", 1, &[MEMORY_SOURCES], BASE_MS)],
    );

    let scope = SourceScope::new(["src-abc"]);
    let hits = provider
        .query_source("work", "src-abc", 10, Some(&scope))
        .await
        .expect("query_source");
    assert_eq!(hits.len(), 1);

    let other = SourceScope::new(["src-other"]);
    let hits = provider
        .query_source("work", "src-abc", 10, Some(&other))
        .await
        .expect("query_source");
    assert!(
        hits.is_empty(),
        "a source-tagged chunk outside scope is denied"
    );
}

#[tokio::test]
async fn tree_query_source_scope_admits_mem_src_prefix() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_chunks(
        &config,
        &[src_chunk(
            "mem_src:src-abc:item-1",
            1,
            &[MEMORY_SOURCES],
            BASE_MS,
        )],
    );

    let scope = SourceScope::new(["src-abc"]);
    let hits = provider
        .query_source("work", "mem_src:src-abc:item-1", 10, Some(&scope))
        .await
        .expect("query_source");
    assert_eq!(
        hits.len(),
        1,
        "the `mem_src:{{allowed}}:` prefix rule must survive the driver hop"
    );
}

#[tokio::test]
async fn tree_query_source_empty_scope_keeps_only_untagged_chunks() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_chunks(
        &config,
        &[
            src_chunk("src-abc", 1, &[MEMORY_SOURCES], BASE_MS),
            src_chunk("src-plain", 1, &[], BASE_MS),
        ],
    );

    let empty = SourceScope::default();
    assert!(
        provider
            .query_source("work", "src-abc", 10, Some(&empty))
            .await
            .expect("query_source")
            .is_empty(),
        "an empty allow list denies all source-attributed content"
    );
    assert_eq!(
        provider
            .query_source("work", "src-plain", 10, Some(&empty))
            .await
            .expect("query_source")
            .len(),
        1,
        "untagged content fails open, exactly as the SQL predicate does"
    );
}

#[tokio::test]
async fn tree_query_source_scope_is_applied_before_limit() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    // Two out-of-scope chunks are NEWER than the in-scope one. A post-filter
    // would spend the limit on them and return nothing; a SQL predicate before
    // LIMIT returns the in-scope row.
    seed_chunks(
        &config,
        &[
            src_chunk("src-abc", 1, &[MEMORY_SOURCES], BASE_MS),
            src_chunk("src-abc", 2, &[MEMORY_SOURCES], BASE_MS + 1_000),
            src_chunk("src-abc", 3, &[MEMORY_SOURCES], BASE_MS + 2_000),
        ],
    );

    let scope = SourceScope::new(["src-abc"]);
    let hits = provider
        .query_source("work", "src-abc", 1, Some(&scope))
        .await
        .expect("query_source");
    assert_eq!(hits.len(), 1, "limit is honoured");
}

// ── seal / cascade ───────────────────────────────────────────────────────

#[tokio::test]
async fn tree_seal_on_empty_buffer_is_a_successful_noop() {
    let (_tmp, provider) = fresh_driver();
    // The default test config resolves NO summarisation provider. Sealing
    // nothing must still succeed, which is why the empty-buffer check runs
    // before provider resolution.
    let status = provider.seal("work").await.expect("seal an empty buffer");
    assert_eq!(status.namespace, "work");
    assert_eq!(status.total_nodes, 0);
}

#[tokio::test]
async fn tree_seal_with_buffered_content_needs_a_summarization_provider() {
    let (_tmp, provider) = fresh_driver();
    provider
        .append(request("work", "something to summarise"))
        .await
        .expect("append");

    let error = provider
        .seal("work")
        .await
        .expect_err("no local AI and no cloud opt-in means no provider");
    match error {
        tinycortex_api::error::MemoryError::Invalid(reason) => assert!(
            reason.contains("summarization provider"),
            "the operator-facing resolver message must survive, got {reason}"
        ),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[tokio::test]
async fn tree_cascade_on_empty_tree_is_a_successful_noop() {
    let (_tmp, provider) = fresh_driver();
    let status = provider
        .cascade("work")
        .await
        .expect("cascade an empty tree");
    assert_eq!(status.total_nodes, 0);
}
