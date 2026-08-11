//! [`MemoryCore`] tests.
//!
//! Two carry weight beyond a round-trip:
//!
//! - `store_preserves_external_sync_taint_through_get` is the security test. It
//!   asserts the *value*, not merely that a taint exists, so it fails the
//!   moment anyone routes the contract's `store` onto `Memory::store` (which
//!   hard-codes `Internal`). Its `Internal` twin exists so it cannot pass by a
//!   constant.
//! - `list_with_no_namespace_spans_every_namespace` pins the divergence between
//!   the contract ("all `None` lists everything") and the engine (`None`
//!   normalises to the global namespace). A naive delegation fails it.

use super::super::test_support::fresh_driver;
use super::*;

use tinycortex_api::provider::MemoryProvider;

#[tokio::test]
async fn store_get_round_trips_through_the_contract() {
    let (_tmp, provider) = fresh_driver();

    provider
        .store(
            "ns_a",
            "k1",
            "value in a",
            MemoryCategory::Core,
            Some("sess-1"),
            MemoryTaint::Internal,
        )
        .await
        .expect("store");

    let got = provider
        .get("ns_a", "k1")
        .await
        .expect("get")
        .expect("entry exists");
    assert_eq!(got.key, "k1");
    assert_eq!(got.content, "value in a");
    assert_eq!(got.category, MemoryCategory::Core);
}

#[tokio::test]
async fn get_returns_none_for_an_absent_key() {
    let (_tmp, provider) = fresh_driver();
    assert!(provider.get("ns_a", "nope").await.expect("get").is_none());
}

#[tokio::test]
async fn forget_removes_the_entry_and_is_idempotent() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "k1",
            "value",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store");

    assert!(provider.forget("ns_a", "k1").await.expect("first forget"));
    assert!(provider.get("ns_a", "k1").await.expect("get").is_none());
    assert!(
        !provider.forget("ns_a", "k1").await.expect("second forget"),
        "forgetting an absent key is Ok(false), never an error"
    );
}

#[tokio::test]
async fn list_scoped_to_namespace_applies_category_and_session_filters() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "core-1",
            "c",
            MemoryCategory::Core,
            Some("sess-1"),
            MemoryTaint::Internal,
        )
        .await
        .expect("store core");
    provider
        .store(
            "ns_a",
            "daily-1",
            "d",
            MemoryCategory::Daily,
            Some("sess-2"),
            MemoryTaint::Internal,
        )
        .await
        .expect("store daily");

    let all = provider.list(Some("ns_a"), None, None).await.expect("list");
    assert_eq!(all.len(), 2);

    let core_only = provider
        .list(Some("ns_a"), Some(&MemoryCategory::Core), None)
        .await
        .expect("list by category");
    assert_eq!(core_only.len(), 1);
    assert_eq!(core_only[0].key, "core-1");

    let session_only = provider
        .list(Some("ns_a"), None, Some("sess-2"))
        .await
        .expect("list by session");
    assert_eq!(session_only.len(), 1);
    assert_eq!(session_only[0].key, "daily-1");
}

#[tokio::test]
async fn list_with_no_namespace_spans_every_namespace() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "a1",
            "in a",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store a");
    provider
        .store(
            "ns_b",
            "b1",
            "in b",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store b");

    let everything = provider.list(None, None, None).await.expect("list all");
    let keys: Vec<&str> = everything.iter().map(|e| e.key.as_str()).collect();
    assert!(keys.contains(&"a1"), "missing ns_a entry: {keys:?}");
    assert!(keys.contains(&"b1"), "missing ns_b entry: {keys:?}");
}

#[tokio::test]
async fn namespaces_reports_per_namespace_counts() {
    let (_tmp, provider) = fresh_driver();
    for key in ["a1", "a2"] {
        provider
            .store(
                "ns_a",
                key,
                "x",
                MemoryCategory::Core,
                None,
                MemoryTaint::Internal,
            )
            .await
            .expect("store");
    }
    provider
        .store(
            "ns_b",
            "b1",
            "x",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store");

    let summaries = provider.namespaces().await.expect("namespaces");
    let a = summaries
        .iter()
        .find(|s| s.namespace == "ns_a")
        .expect("ns_a summary");
    let b = summaries
        .iter()
        .find(|s| s.namespace == "ns_b")
        .expect("ns_b summary");
    assert_eq!(a.count, 2);
    assert_eq!(b.count, 1);
}

/// SECURITY: the contract stamps provenance before the call; the driver must
/// persist exactly what it was handed. Routing onto `Memory::store` would
/// launder this to `Internal`.
#[tokio::test]
async fn store_preserves_external_sync_taint_through_get() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "synced",
            "from an external source",
            MemoryCategory::Core,
            None,
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("store");

    let got = provider
        .get("ns_a", "synced")
        .await
        .expect("get")
        .expect("entry exists");
    assert_eq!(got.taint, MemoryTaint::ExternalSync);
}

/// The negative half of the taint pair — without it the assertion above could
/// pass against a driver that hard-coded `ExternalSync`.
#[tokio::test]
async fn store_preserves_internal_taint_through_get() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "typed",
            "written by the user",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store");

    let got = provider
        .get("ns_a", "typed")
        .await
        .expect("get")
        .expect("entry exists");
    assert_eq!(got.taint, MemoryTaint::Internal);
}

#[tokio::test]
async fn core_calls_resolve_the_client_lazily_and_only_once() {
    let (_tmp, provider) = fresh_driver();
    assert!(
        provider.workspace_dir().parent().is_some(),
        "sanity: workspace is nested under the temp dir"
    );
    // Before any call the workspace does not exist; the first contract call
    // creates it.
    assert!(!provider.workspace_dir().exists());
    provider.namespaces().await.expect("namespaces");
    assert!(provider.workspace_dir().exists());
    assert_eq!(provider.driver_id(), "tinycortex");
}
