//! [`MemoryGraph`] tests.
//!
//! The load-bearing ones:
//!
//! - `kv_get_returns_a_record_with_a_timestamp` is why this family cannot
//!   delegate to `MemoryClient::kv_get`, which returns a bare `Value` and drops
//!   `updated_at`.
//! - `put_relation_round_trips_with_normalized_entities` pins the storage
//!   layer's upper-casing. It is inherited behaviour, asserted so nobody
//!   "fixes" it in the driver.
//! - the two `*_is_visible_to_*` tests are the same-store proofs.

use super::super::test_support::fresh_driver;
use super::*;

use serde_json::json;

fn relation(
    namespace: Option<&str>,
    subject: &str,
    predicate: &str,
    object: &str,
) -> GraphRelationRecord {
    GraphRelationRecord {
        namespace: namespace.map(str::to_string),
        subject: subject.to_string(),
        predicate: predicate.to_string(),
        object: object.to_string(),
        attrs: json!({"note": "from the contract"}),
        updated_at: 0.0,
        evidence_count: 1,
        order_index: None,
        document_ids: vec!["doc-1".to_string()],
        chunk_ids: vec![],
    }
}

#[tokio::test]
async fn kv_put_then_kv_get_returns_a_record_with_a_timestamp() {
    let (_tmp, provider) = fresh_driver();

    provider
        .kv_put(Some("kv_ns"), "theme", json!("dark"))
        .await
        .expect("kv_put");

    let record = provider
        .kv_get(Some("kv_ns"), "theme")
        .await
        .expect("kv_get")
        .expect("record exists");

    assert_eq!(record.key, "theme");
    assert_eq!(record.value, json!("dark"));
    assert_eq!(record.namespace.as_deref(), Some("kv_ns"));
    assert!(
        record.updated_at > 0.0,
        "the contract's record carries updated_at; the bare-value path cannot"
    );
}

#[tokio::test]
async fn kv_get_returns_none_for_unknown_key() {
    let (_tmp, provider) = fresh_driver();
    assert!(provider
        .kv_get(Some("kv_ns"), "absent")
        .await
        .expect("kv_get")
        .is_none());
}

#[tokio::test]
async fn kv_put_with_none_namespace_writes_the_global_slice() {
    let (_tmp, provider) = fresh_driver();

    provider
        .kv_put(None, "global_key", json!(7))
        .await
        .expect("kv_put");

    let record = provider
        .kv_get(None, "global_key")
        .await
        .expect("kv_get")
        .expect("record exists");
    assert_eq!(record.value, json!(7));
    assert!(
        record.namespace.is_none(),
        "a global row must report no namespace"
    );

    // And it must not leak into a namespace slice.
    assert!(provider
        .kv_get(Some("kv_ns"), "global_key")
        .await
        .expect("kv_get")
        .is_none());
}

#[tokio::test]
async fn kv_list_applies_prefix_and_limit() {
    let (_tmp, provider) = fresh_driver();
    for key in ["ui.theme", "ui.density", "net.proxy"] {
        provider
            .kv_put(Some("kv_ns"), key, json!(key))
            .await
            .expect("kv_put");
    }

    let all = provider
        .kv_list(Some("kv_ns"), None, 100)
        .await
        .expect("kv_list");
    assert_eq!(all.len(), 3);

    let ui = provider
        .kv_list(Some("kv_ns"), Some("ui."), 100)
        .await
        .expect("kv_list");
    assert_eq!(ui.len(), 2, "prefix must narrow the slice: {ui:?}");
    assert!(ui.iter().all(|record| record.key.starts_with("ui.")));

    let capped = provider
        .kv_list(Some("kv_ns"), None, 1)
        .await
        .expect("kv_list");
    assert_eq!(capped.len(), 1, "limit must truncate");
}

#[tokio::test]
async fn kv_list_with_none_namespace_reads_the_global_slice() {
    let (_tmp, provider) = fresh_driver();
    provider
        .kv_put(None, "g1", json!(1))
        .await
        .expect("kv_put global");
    provider
        .kv_put(Some("kv_ns"), "n1", json!(2))
        .await
        .expect("kv_put namespaced");

    let global = provider.kv_list(None, None, 100).await.expect("kv_list");
    let keys: Vec<&str> = global.iter().map(|record| record.key.as_str()).collect();
    assert!(keys.contains(&"g1"));
    assert!(
        !keys.contains(&"n1"),
        "the global slice must not include namespaced rows: {keys:?}"
    );
}

#[tokio::test]
async fn put_relation_round_trips_with_normalized_entities() {
    let (_tmp, provider) = fresh_driver();

    provider
        .put_relation(relation(Some("g_ns"), "Alice", "owns", "Phoenix"))
        .await
        .expect("put_relation");

    let rows = provider
        .relations(Some("g_ns"), None, None, 50)
        .await
        .expect("relations");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    // Inherited from `normalize_graph_entity` / `normalize_graph_predicate`.
    assert_eq!(row.subject, "ALICE");
    assert_eq!(row.predicate, "OWNS");
    assert_eq!(row.object, "PHOENIX");
    assert_eq!(row.namespace.as_deref(), Some("g_ns"));
    // The structured fields survive the attrs round-trip.
    assert_eq!(row.document_ids, vec!["doc-1".to_string()]);
    assert!(row.evidence_count >= 1);
    assert_eq!(row.attrs.get("note"), Some(&json!("from the contract")));
    assert!(
        row.updated_at > 0.0,
        "the store stamps its own updated_at; the caller's 0.0 must not survive"
    );
}

#[tokio::test]
async fn relations_filters_by_subject_and_predicate() {
    let (_tmp, provider) = fresh_driver();
    provider
        .put_relation(relation(Some("g_ns"), "alice", "owns", "phoenix"))
        .await
        .expect("put_relation");
    provider
        .put_relation(relation(Some("g_ns"), "alice", "likes", "tea"))
        .await
        .expect("put_relation");
    provider
        .put_relation(relation(Some("g_ns"), "bob", "owns", "kettle"))
        .await
        .expect("put_relation");

    let alice = provider
        .relations(Some("g_ns"), Some("alice"), None, 50)
        .await
        .expect("relations");
    assert_eq!(alice.len(), 2, "{alice:?}");

    let owns = provider
        .relations(Some("g_ns"), None, Some("owns"), 50)
        .await
        .expect("relations");
    assert_eq!(owns.len(), 2, "{owns:?}");

    let both = provider
        .relations(Some("g_ns"), Some("alice"), Some("owns"), 50)
        .await
        .expect("relations");
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].object, "PHOENIX");
}

#[tokio::test]
async fn relations_truncates_to_the_limit() {
    let (_tmp, provider) = fresh_driver();
    for object in ["one", "two", "three"] {
        provider
            .put_relation(relation(Some("g_ns"), "alice", "owns", object))
            .await
            .expect("put_relation");
    }
    let rows = provider
        .relations(Some("g_ns"), None, None, 2)
        .await
        .expect("relations");
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn relations_with_none_namespace_spans_namespaces_and_global() {
    let (_tmp, provider) = fresh_driver();
    provider
        .put_relation(relation(Some("ns_one"), "alice", "owns", "phoenix"))
        .await
        .expect("put_relation namespaced");
    provider
        .put_relation(relation(None, "bob", "owns", "kettle"))
        .await
        .expect("put_relation global");

    let all = provider
        .relations(None, None, None, 100)
        .await
        .expect("relations");
    let subjects: Vec<&str> = all.iter().map(|row| row.subject.as_str()).collect();
    assert!(subjects.contains(&"ALICE"), "{subjects:?}");
    assert!(subjects.contains(&"BOB"), "{subjects:?}");

    // The namespaced row must still be scoped, not global.
    let global_only: Vec<&str> = all
        .iter()
        .filter(|row| row.namespace.is_none())
        .map(|row| row.subject.as_str())
        .collect();
    assert_eq!(global_only, vec!["BOB"]);
}

#[tokio::test]
async fn kv_put_through_the_contract_is_visible_to_memory_client_kv_get() {
    let (_tmp, provider) = fresh_driver();
    provider
        .kv_put(Some("kv_ns"), "theme", json!("dark"))
        .await
        .expect("kv_put");

    let via_client = provider
        .client()
        .await
        .expect("client")
        .kv_get(Some("kv_ns"), "theme")
        .await
        .expect("kv_get");
    assert_eq!(via_client, Some(json!("dark")));
}

#[tokio::test]
async fn put_relation_through_the_contract_is_visible_to_memory_client_graph_query() {
    let (_tmp, provider) = fresh_driver();
    provider
        .put_relation(relation(Some("g_ns"), "alice", "owns", "phoenix"))
        .await
        .expect("put_relation");

    let via_client = provider
        .client()
        .await
        .expect("client")
        .graph_query(Some("g_ns"), None, None)
        .await
        .expect("graph_query");
    assert_eq!(via_client.len(), 1, "{via_client:?}");
    assert_eq!(
        via_client[0].get("subject").and_then(|v| v.as_str()),
        Some("ALICE")
    );
}
