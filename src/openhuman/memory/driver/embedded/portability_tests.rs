//! [`MemoryPortability`] tests.
//!
//! `export_import_round_trips_content_category_session_and_taint` is the one
//! that makes the binding reversible in practice, and
//! `import_does_not_restamp_provenance` is its security half — an import that
//! stamped `Internal` would silently upgrade externally-sourced content on
//! every migration.

use super::super::test_support::fresh_driver;
use super::*;

use tempfile::TempDir;
use tinycortex_api::provider::MemoryCore;
use tinycortex_api::types::MemoryTaint;

use crate::openhuman::memory::driver::embedded::EmbeddedMemoryProvider;

/// Drains the whole export, asserting the loop terminates on a `None` cursor.
async fn export_all(provider: &EmbeddedMemoryProvider, limit: usize) -> Vec<ExportRecord> {
    let mut cursor: Option<String> = None;
    let mut out = Vec::new();
    for _ in 0..64 {
        let page = provider
            .export_page(cursor.as_deref(), limit)
            .await
            .expect("export page");
        out.extend(page.records);
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return out,
        }
    }
    panic!("export did not terminate within 64 pages");
}

async fn seed(provider: &EmbeddedMemoryProvider) {
    provider
        .store(
            "ns_a",
            "a1",
            "first in a",
            MemoryCategory::Core,
            Some("sess-1"),
            MemoryTaint::Internal,
        )
        .await
        .expect("store a1");
    provider
        .store(
            "ns_a",
            "a2",
            "second in a",
            MemoryCategory::Custom("notes".into()),
            None,
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("store a2");
    provider
        .store(
            "ns_b",
            "b1",
            "first in b",
            MemoryCategory::Daily,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store b1");
}

#[tokio::test]
async fn export_of_an_empty_store_terminates_immediately() {
    let (_tmp, provider) = fresh_driver();
    let page = provider.export_page(None, 10).await.expect("export");
    assert!(page.records.is_empty());
    assert!(page.next_cursor.is_none());
}

#[tokio::test]
async fn export_paginates_across_namespaces_and_terminates_on_a_null_cursor() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider).await;

    // A page size of 1 forces both intra-namespace and cross-namespace cursor
    // advances.
    let records = export_all(&provider, 1).await;
    assert_eq!(records.len(), 3, "every seeded entry must be exported");

    let mut namespaces: Vec<&str> = records
        .iter()
        .filter_map(|r| r.namespace.as_deref())
        .collect();
    namespaces.sort_unstable();
    namespaces.dedup();
    assert_eq!(namespaces, vec!["ns_a", "ns_b"]);
    assert!(records.iter().all(|r| r.kind == ENTRY_KIND));
}

#[tokio::test]
async fn export_import_round_trips_content_category_session_and_taint() {
    let (_source_tmp, source) = fresh_driver();
    seed(&source).await;
    let records = export_all(&source, 2).await;

    // Import into a *second*, independent workspace.
    let target_tmp = TempDir::new().unwrap();
    let target = EmbeddedMemoryProvider::new(
        target_tmp.path().join("ws"),
        crate::openhuman::config::schema::MemoryHooksConfig::default(),
    );
    let outcome = target.import_records(records).await.expect("import");
    assert_eq!(outcome.imported, 3);
    assert_eq!(outcome.failed, 0);
    assert!(outcome.errors.is_empty());

    let a1 = target
        .get("ns_a", "a1")
        .await
        .expect("get")
        .expect("a1 imported");
    assert_eq!(a1.content, "first in a");
    assert_eq!(a1.category, MemoryCategory::Core);
    assert_eq!(a1.taint, MemoryTaint::Internal);

    let a2 = target
        .get("ns_a", "a2")
        .await
        .expect("get")
        .expect("a2 imported");
    assert_eq!(a2.content, "second in a");
    assert_eq!(a2.category, MemoryCategory::Custom("notes".into()));
    assert_eq!(a2.taint, MemoryTaint::ExternalSync);

    let b1 = target
        .get("ns_b", "b1")
        .await
        .expect("get")
        .expect("b1 imported");
    assert_eq!(b1.category, MemoryCategory::Daily);

    // And the export of the target reproduces the same set.
    let reexported = export_all(&target, 10).await;
    assert_eq!(reexported.len(), 3);
}

/// SECURITY: an importing driver persists the taint it is given.
#[tokio::test]
async fn import_does_not_restamp_provenance() {
    let (_tmp, provider) = fresh_driver();
    let record = ExportRecord {
        kind: ENTRY_KIND.to_string(),
        id: "doc-1".into(),
        namespace: Some("ns_a".into()),
        taint: MemoryTaint::ExternalSync,
        payload: json!({
            "key": "synced",
            "content": "from elsewhere",
            "category": "core",
            "session_id": serde_json::Value::Null,
            "timestamp": "2026-01-01T00:00:00Z",
        }),
    };

    let outcome = provider.import_records(vec![record]).await.expect("import");
    assert_eq!(outcome.imported, 1);

    let got = provider
        .get("ns_a", "synced")
        .await
        .expect("get")
        .expect("imported");
    assert_eq!(got.taint, MemoryTaint::ExternalSync);
}

#[tokio::test]
async fn import_reports_bad_records_as_failed_with_a_reason_and_does_not_abort_the_batch() {
    let (_tmp, provider) = fresh_driver();
    let good = ExportRecord {
        kind: ENTRY_KIND.to_string(),
        id: "doc-good".into(),
        namespace: Some("ns_a".into()),
        taint: MemoryTaint::Internal,
        payload: json!({
            "key": "kept",
            "content": "SECRET-CONTENT-MARKER",
            "category": "core",
        }),
    };
    let unknown_kind = ExportRecord {
        kind: "chunk".into(),
        id: "doc-chunk".into(),
        namespace: Some("ns_a".into()),
        taint: MemoryTaint::Internal,
        payload: json!({ "content": "SECRET-CONTENT-MARKER" }),
    };
    let malformed = ExportRecord {
        kind: ENTRY_KIND.to_string(),
        id: "doc-bad".into(),
        namespace: Some("ns_a".into()),
        taint: MemoryTaint::Internal,
        payload: json!({ "content": "SECRET-CONTENT-MARKER" }),
    };

    let outcome = provider
        .import_records(vec![unknown_kind, good, malformed])
        .await
        .expect("a malformed record must not fail the batch");

    assert_eq!(outcome.imported, 1);
    assert_eq!(outcome.failed, 2);
    assert_eq!(outcome.errors.len(), 2);
    for error in &outcome.errors {
        assert!(
            !error.contains("SECRET-CONTENT-MARKER"),
            "import errors are logged and must not carry record content: {error}"
        );
    }
    assert!(provider.get("ns_a", "kept").await.expect("get").is_some());
}

#[tokio::test]
async fn export_rejects_a_cursor_this_driver_did_not_issue() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider).await;

    for bad in ["nonsense", "1", "a:b", "99:0", "0:9999"] {
        match provider.export_page(Some(bad), 10).await {
            Err(MemoryError::Invalid(_)) => {}
            Err(other) => panic!("cursor '{bad}' produced the wrong variant: {other:?}"),
            Ok(page) => panic!(
                "cursor '{bad}' must be rejected, got {} record(s)",
                page.records.len()
            ),
        }
    }
}
