//! [`MemoryDocuments`] tests.
//!
//! Two carry weight beyond a round-trip:
//!
//! - `get_document_finds_a_key_the_write_path_canonicalized` is the #5164
//!   regression. The write path rewrites a PII-shaped key before storing it, so
//!   a read that addresses the raw key misses, the caller treats the row as
//!   absent, and writes it again. It passes only because
//!   `get_document_by_key` canonicalizes through the same helper.
//! - `put_document_through_the_contract_is_visible_to_list_documents` is the
//!   same-store proof: the contract write lands where the existing RPC read
//!   path looks, not in a parallel store.

use super::super::test_support::fresh_driver;
use super::*;

use serde_json::json;

fn doc(namespace: &str, key: &str, title: &str, content: &str) -> NamespaceDocumentInput {
    NamespaceDocumentInput {
        namespace: namespace.to_string(),
        key: key.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        source_type: "chat".to_string(),
        priority: "normal".to_string(),
        tags: vec!["t1".to_string()],
        metadata: json!({"origin": "test"}),
        category: "core".to_string(),
        session_id: None,
        document_id: None,
        taint: Default::default(),
    }
}

#[tokio::test]
async fn put_document_then_get_document_returns_the_same_body() {
    let (_tmp, provider) = fresh_driver();

    let id = provider
        .put_document(doc("docs_ns", "readme", "Readme", "the whole body"))
        .await
        .expect("put_document");
    assert!(!id.is_empty(), "put_document must return a document id");

    let got = provider
        .get_document("docs_ns", "readme")
        .await
        .expect("get_document")
        .expect("document exists");

    assert_eq!(got.key, "readme");
    assert_eq!(got.title, "Readme");
    // The body is the point: `list_documents` cannot back this method because
    // its SELECT has no `content` column.
    assert_eq!(got.content, "the whole body");
    assert_eq!(got.tags, vec!["t1".to_string()]);
    assert_eq!(got.metadata, json!({"origin": "test"}));
}

#[tokio::test]
async fn put_document_upserts_on_the_same_key() {
    let (_tmp, provider) = fresh_driver();

    provider
        .put_document(doc("docs_ns", "readme", "First", "first body"))
        .await
        .expect("first put");
    provider
        .put_document(doc("docs_ns", "readme", "Second", "second body"))
        .await
        .expect("second put");

    let got = provider
        .get_document("docs_ns", "readme")
        .await
        .expect("get_document")
        .expect("document exists");
    assert_eq!(got.content, "second body");
}

#[tokio::test]
async fn get_document_returns_none_for_unknown_key() {
    let (_tmp, provider) = fresh_driver();
    assert!(provider
        .get_document("docs_ns", "nope")
        .await
        .expect("get_document")
        .is_none());
}

#[tokio::test]
async fn get_document_finds_a_key_the_write_path_canonicalized() {
    use crate::openhuman::memory::store::safety::canonical_document_key;

    let (_tmp, provider) = fresh_driver();
    // A strict-gated PII shape — `canonical_identifier` deliberately leaves
    // scanner-built identifiers (JIDs, E.164 chat ids, timestamps) alone.
    let raw_key = "ssn-123-45-6789";
    // Guard the premise: if this key stops being rewritten the test still
    // passes but stops testing anything, so assert the rewrite happens.
    assert_ne!(
        canonical_document_key(raw_key),
        raw_key,
        "this key must be PII-shaped for the regression to mean anything"
    );

    provider
        .put_document(doc("docs_ns", raw_key, "Contact", "a phone number"))
        .await
        .expect("put_document");

    let got = provider
        .get_document("docs_ns", raw_key)
        .await
        .expect("get_document");
    assert!(
        got.is_some(),
        "a canonicalized key must stay addressable by its raw form (#5164)"
    );
}

#[tokio::test]
async fn query_documents_returns_hits_and_rendered_context() {
    let (_tmp, provider) = fresh_driver();

    provider
        .put_document(doc(
            "docs_ns",
            "kettle",
            "Kettle",
            "the kettle boils at one hundred degrees",
        ))
        .await
        .expect("put_document");

    let context = provider
        .query_documents("docs_ns", "kettle", 5)
        .await
        .expect("query_documents");

    assert_eq!(context.namespace, "docs_ns");
    assert_eq!(context.query.as_deref(), Some("kettle"));
    assert!(
        !context.hits.is_empty(),
        "the stored document must be retrievable"
    );
    assert!(
        !context.context_text.is_empty(),
        "the driver must return the host's rendered context, not re-assemble it"
    );
}

#[tokio::test]
async fn query_documents_on_an_empty_namespace_is_not_an_error() {
    let (_tmp, provider) = fresh_driver();
    let context = provider
        .query_documents("empty_ns", "anything", 5)
        .await
        .expect("query_documents");
    assert!(context.hits.is_empty());
}

#[tokio::test]
async fn put_document_through_the_contract_is_visible_to_list_documents() {
    let (_tmp, provider) = fresh_driver();

    provider
        .put_document(doc("docs_ns", "shared", "Shared", "body"))
        .await
        .expect("put_document");

    // The existing RPC read path, reached through the same client.
    let listed = provider
        .client()
        .await
        .expect("client")
        .list_documents(Some("docs_ns"))
        .await
        .expect("list_documents");
    let keys: Vec<String> = listed
        .get("documents")
        .and_then(serde_json::Value::as_array)
        .expect("documents array")
        .iter()
        .filter_map(|d| d.get("key").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect();

    assert!(
        keys.contains(&"shared".to_string()),
        "contract write must land in the store the RPC path reads: {keys:?}"
    );
}
