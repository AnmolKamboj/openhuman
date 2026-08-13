//! The wrapped-accessor property — the reason this milestone exists — plus
//! step 2, which lives on `GuardedTree::query_source`.

use crate::openhuman::memory::api::provider::types::SourceScope;
use crate::openhuman::memory::api::provider::{MemoryProvider, MemoryTree};
use crate::openhuman::memory::api::tree::IngestRequest;
use crate::openhuman::memory::api::types::MemoryTaint;

use crate::openhuman::memory::guard::test_support::{
    document, embedded_policy, external_policy, guarded,
};
use crate::openhuman::memory::source_scope::with_source_scope;
use crate::openhuman::security::live_policy;
use crate::openhuman::security::policy::{AutonomyLevel, SecurityPolicy};

fn ingest_request(content: &str) -> IngestRequest {
    IngestRequest {
        namespace: "ns".into(),
        content: content.into(),
        timestamp: None,
        metadata: None,
    }
}

// ── The wrapped-accessor property ───────────────────────────────────────────

#[tokio::test]
async fn guard_as_tree_is_not_the_raw_driver_handle() {
    let (driver, guard) = guarded(embedded_policy());
    let via_guard = guard.as_tree().expect("tree family") as *const dyn MemoryTree;
    let raw = driver.as_tree().expect("tree family") as *const dyn MemoryTree;
    assert!(
        !std::ptr::eq(via_guard, raw),
        "the accessor handed out the driver's own handle — the guard is bypassable"
    );
}

/// The assertion that actually matters. Pointer inequality only proves *some*
/// wrapper exists; this proves the wrapper still enforces.
#[tokio::test]
async fn guard_as_tree_still_applies_policy_reached_through_the_accessor() {
    let dir = std::env::temp_dir();
    let _tier = live_policy::install_scoped(
        std::sync::Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        }),
        dir.clone(),
        dir,
    );

    let (driver, guard) = guarded(embedded_policy());
    let err = guard
        .as_tree()
        .expect("tree family")
        .append(ingest_request("hello"))
        .await
        .expect_err("a readonly tier must refuse a tree write");
    assert!(err.to_string().contains("memory guard: "), "{err}");
    assert_eq!(
        driver.call_count(),
        0,
        "the driver must not be reached at all"
    );
}

#[tokio::test]
async fn every_optional_family_accessor_enforces_the_tier() {
    let dir = std::env::temp_dir();
    let _tier = live_policy::install_scoped(
        std::sync::Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        }),
        dir.clone(),
        dir,
    );
    let (driver, guard) = guarded(embedded_policy());

    // One representative *write* per optional family. Each must be refused
    // before the driver sees it — a family whose decorator forwarded raw would
    // record a call here.
    guard
        .as_ingest()
        .unwrap()
        .ingest_chat(vec![])
        .await
        .expect_err("ingest");
    guard
        .as_documents()
        .unwrap()
        .put_document(document("x", MemoryTaint::Internal))
        .await
        .expect_err("documents");
    guard.as_tree().unwrap().seal("ns").await.expect_err("tree");
    guard
        .as_entities()
        .unwrap()
        .touch_entities("ns", &[])
        .await
        .expect_err("entities");
    guard
        .as_graph()
        .unwrap()
        .kv_put(None, "k", serde_json::Value::Null)
        .await
        .expect_err("graph");
    guard
        .as_diff()
        .unwrap()
        .capture_snapshot("src")
        .await
        .expect_err("diff");
    guard
        .as_goals()
        .unwrap()
        .set_goals(Default::default())
        .await
        .expect_err("goals");
    guard
        .as_tool_memory()
        .unwrap()
        .delete_tool_rule("t", "r")
        .await
        .expect_err("tool_memory");
    guard
        .as_sources()
        .unwrap()
        .forget_source("src")
        .await
        .expect_err("sources");
    guard
        .as_maintenance()
        .unwrap()
        .compact()
        .await
        .expect_err("maintenance");

    assert_eq!(
        driver.call_count(),
        0,
        "at least one family decorator forwarded an unguarded handle: {:?}",
        driver.calls()
    );
}

// ── Step 2 ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn guard_fills_query_source_scope_from_the_task_local() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_tree()
            .unwrap()
            .query_source("ns", "src", 10, None)
            .await
            .expect("query_source");
    })
    .await;
    let call = driver.only_call();
    assert_eq!(call.scoped, Some(true));
    assert_eq!(call.content.as_deref(), Some("slack:#eng"));
}

#[tokio::test]
async fn guard_explicit_scope_is_intersected_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_tree()
            .unwrap()
            .query_source("ns", "src", 10, Some(&explicit))
            .await
            .expect("query_source");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some(""),
        "a request outside the ambient allowlist must fail closed"
    );
}

#[tokio::test]
async fn guard_leaves_query_source_unscoped_outside_a_source_scope() {
    let (driver, guard) = guarded(embedded_policy());
    guard
        .as_tree()
        .unwrap()
        .query_source("ns", "src", 10, None)
        .await
        .expect("query_source");
    assert_eq!(driver.only_call().scoped, Some(false));
}

// ── Steps 3 + 4 through a family accessor ───────────────────────────────────

#[tokio::test]
async fn family_writes_are_taint_stamped_too() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_documents()
            .unwrap()
            .put_document(document("body", MemoryTaint::Internal))
            .await
            .expect("put_document");
    })
    .await;
    assert_eq!(driver.only_call().taint, Some(MemoryTaint::ExternalSync));
}

#[tokio::test]
async fn family_writes_are_not_redacted_for_an_embedded_driver() {
    let secrety = "Authorization: Bearer abcdefghijklmnop";
    let (driver, guard) = guarded(embedded_policy());
    guard
        .as_documents()
        .unwrap()
        .put_document(document(secrety, MemoryTaint::Internal))
        .await
        .expect("put_document");
    assert_eq!(driver.only_call().content.as_deref(), Some(secrety));
}

#[tokio::test]
async fn family_calls_are_refused_for_an_untrusted_external_driver() {
    let (driver, guard) = guarded(external_policy("untrusted"));
    guard
        .as_tree()
        .unwrap()
        .query_source("ns", "src", 10, None)
        .await
        .expect_err("fail-closed");
    assert_eq!(driver.call_count(), 0);
}
