//! [`MemorySourceSink`] tests.
//!
//! `accept_source_items_persists_the_caller_supplied_taint` is the security test
//! of this family, and the reason it writes through the document tier at all: a
//! sink that quietly downgraded `external_sync` to `internal` would erase a
//! prompt-injection trust boundary while every other assertion here still
//! passed.

use super::super::test_support::fresh_driver;
use super::*;

fn item(item_id: &str, title: &str, content: &str) -> SourceItem {
    SourceItem {
        item_id: item_id.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        mime: Some("text/plain".to_string()),
        url: Some(format!("https://example.invalid/{item_id}")),
        updated_at_ms: Some(1_700_000_000_000),
        tags: vec!["synced".to_string()],
    }
}

#[tokio::test]
async fn accept_source_items_writes_one_document_per_item() {
    let (_tmp, provider) = fresh_driver();

    let outcome = provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![
                item("i1", "First", "first body"),
                item("i2", "Second", "second body"),
            ],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("accept_source_items");

    assert_eq!(outcome.written, 2);
    // See the module docs: `put_doc` gives no already-present signal, so a
    // non-zero `skipped` here would be invented.
    assert_eq!(outcome.skipped, 0);
    assert_eq!(outcome.ids.len(), 2);
}

#[tokio::test]
async fn accept_source_items_persists_the_caller_supplied_taint() {
    use tinycortex_api::provider::MemoryDocuments;

    let (_tmp, provider) = fresh_driver();
    provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![item("i1", "First", "first body")],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("accept_source_items");

    let stored = provider
        .get_document("source:src_a", "i1")
        .await
        .expect("get_document")
        .expect("document exists");
    assert_eq!(
        stored.taint,
        MemoryTaint::ExternalSync,
        "the sink must persist the host-stamped provenance, never downgrade it"
    );
    assert_eq!(stored.content, "first body");
}

#[tokio::test]
async fn accept_source_items_upserts_on_the_item_id() {
    use tinycortex_api::provider::MemoryDocuments;

    let (_tmp, provider) = fresh_driver();
    provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![item("i1", "First", "v1")],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("first accept");
    provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![item("i1", "First", "v2")],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("second accept");

    let stored = provider
        .get_document("source:src_a", "i1")
        .await
        .expect("get_document")
        .expect("document exists");
    assert_eq!(stored.content, "v2", "item_id is the dedupe key");
}

#[tokio::test]
async fn accept_source_items_refuses_an_empty_item_id() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![item("", "No id", "body")],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect_err("an empty dedupe key must be refused, not collapsed");
    assert!(matches!(error, MemoryError::Invalid(_)), "got: {error:?}");
}

#[tokio::test]
async fn accept_an_empty_batch_is_a_no_op() {
    let (_tmp, provider) = fresh_driver();
    let outcome = provider
        .accept_source_items("src_a", "folder", Vec::new(), MemoryTaint::ExternalSync)
        .await
        .expect("accept_source_items");
    assert_eq!(outcome, IngestOutcome::default());
}

#[tokio::test]
async fn forget_source_removes_what_the_sink_wrote_and_is_idempotent() {
    use tinycortex_api::provider::MemoryDocuments;

    let (_tmp, provider) = fresh_driver();
    provider
        .accept_source_items(
            "src_a",
            "folder",
            vec![item("i1", "First", "a"), item("i2", "Second", "b")],
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("accept_source_items");

    let removed = provider
        .forget_source("src_a")
        .await
        .expect("forget_source");
    assert_eq!(removed, 2);

    assert!(provider
        .get_document("source:src_a", "i1")
        .await
        .expect("get_document")
        .is_none());

    // Idempotent, per the contract.
    assert_eq!(
        provider
            .forget_source("src_a")
            .await
            .expect("second forget"),
        0
    );
}

#[tokio::test]
async fn forget_source_on_an_unknown_source_is_zero_not_an_error() {
    let (_tmp, provider) = fresh_driver();
    assert_eq!(
        provider
            .forget_source("never-synced")
            .await
            .expect("forget_source must be idempotent"),
        0
    );
}

#[tokio::test]
async fn forget_source_leaves_a_sibling_source_alone() {
    use tinycortex_api::provider::MemoryDocuments;

    let (_tmp, provider) = fresh_driver();
    for source in ["src_a", "src_a_extra"] {
        provider
            .accept_source_items(
                source,
                "folder",
                vec![item("i1", "First", "body")],
                MemoryTaint::ExternalSync,
            )
            .await
            .expect("accept_source_items");
    }

    provider
        .forget_source("src_a")
        .await
        .expect("forget_source");

    // Exact match, never a prefix — `src_a_extra` shares a prefix with `src_a`.
    assert!(provider
        .get_document("source:src_a_extra", "i1")
        .await
        .expect("get_document")
        .is_some());
}
