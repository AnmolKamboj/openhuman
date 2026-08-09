//! [`MemoryDiff`] tests.
//!
//! Snapshots are seeded straight through the ledger (the same trick
//! `diff::ops`' own tests use) rather than through `capture_snapshot`, because
//! capturing needs a populated chunk store and the mapping under test is the
//! ledger→contract one.
//!
//! Two tests carry weight beyond shape:
//!
//! - `diff_rejects_a_snapshot_belonging_to_another_source` covers the hole
//!   `ops::compute_diff` leaves open — it never looks at `source_id`, so without
//!   the driver's check a caller can name source B while diffing source A's
//!   snapshots and get a plausible report.
//! - `snapshots_on_an_unknown_source_is_empty_not_an_error` pins the deliberate
//!   asymmetry with `capture_snapshot`'s `NotFound`.

use super::super::test_support::fresh_driver;
use super::*;

use tinycortex::memory::diff::{Ledger, SnapshotMeta};

use crate::openhuman::memory::driver::embedded::EmbeddedMemoryProvider;

/// Commit a snapshot directly into the ledger for `source_id`.
fn seed(
    provider: &EmbeddedMemoryProvider,
    source_id: &str,
    at_ms: i64,
    items: &[(&str, &str)],
) -> Snapshot {
    let ledger = Ledger::open(provider.workspace_dir()).expect("ledger opens");
    let items: Vec<(String, String)> = items
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    ledger
        .commit_snapshot(
            &SnapshotMeta {
                source_id: source_id.to_string(),
                source_kind: "folder".to_string(),
                label: "Docs".to_string(),
                trigger: SnapshotTrigger::Auto,
            },
            &items,
            at_ms,
        )
        .expect("commit snapshot")
}

#[tokio::test]
async fn capture_snapshot_on_an_unknown_source_is_not_found() {
    let (_tmp, provider) = fresh_driver();
    let error = provider
        .capture_snapshot("no-such-source")
        .await
        .expect_err("unknown source must not silently succeed");
    assert!(
        matches!(error, MemoryError::NotFound(ref id) if id == "no-such-source"),
        "got: {error:?}"
    );
}

#[tokio::test]
async fn snapshots_on_an_unknown_source_is_empty_not_an_error() {
    let (_tmp, provider) = fresh_driver();
    let snapshots = provider
        .snapshots("no-such-source", 10)
        .await
        .expect("an unknown source yields an empty list, per the contract");
    assert!(snapshots.is_empty());
}

#[tokio::test]
async fn snapshots_maps_ledger_entries_onto_the_contract_shape() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider, "src_a", 1_000, &[("a", "alpha")]);
    seed(&provider, "src_a", 2_000, &[("a", "alpha"), ("b", "beta")]);
    // A second source must not leak into the first source's listing.
    seed(&provider, "src_b", 3_000, &[("z", "zeta")]);

    let snapshots = provider.snapshots("src_a", 10).await.expect("snapshots");

    assert_eq!(snapshots.len(), 2);
    assert!(snapshots.iter().all(|s| s.source_id == "src_a"));
    // Newest first, per the trait doc.
    assert_eq!(snapshots[0].taken_at_ms, 2_000);
    assert_eq!(snapshots[0].item_count, 2);
    assert_eq!(snapshots[0].label, "Docs");
    assert!(!snapshots[0].id.is_empty());
}

#[tokio::test]
async fn snapshots_honours_the_limit() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider, "src_a", 1_000, &[("a", "alpha")]);
    seed(&provider, "src_a", 2_000, &[("a", "beta")]);

    let snapshots = provider.snapshots("src_a", 1).await.expect("snapshots");
    assert_eq!(snapshots.len(), 1);
}

#[tokio::test]
async fn diff_counts_and_per_item_kinds_match_the_ledger() {
    let (_tmp, provider) = fresh_driver();
    let from = seed(
        &provider,
        "src_a",
        1_000,
        &[("a", "alpha"), ("b", "beta"), ("c", "gamma")],
    );
    let to = seed(
        &provider,
        "src_a",
        2_000,
        &[("a", "alpha"), ("b", "beta v2"), ("d", "delta")],
    );

    let report = provider
        .diff("src_a", Some(&from.id), &to.id)
        .await
        .expect("diff");

    assert_eq!(report.source_id, "src_a");
    assert_eq!(report.from_snapshot_id.as_deref(), Some(from.id.as_str()));
    assert_eq!(report.to_snapshot_id, to.id);
    assert_eq!(report.added, 1);
    assert_eq!(report.removed, 1);
    assert_eq!(report.modified, 1);
    assert_eq!(report.unchanged, 1);

    let kind_of = |id: &str| {
        report
            .changes
            .iter()
            .find(|c| c.item_id == id)
            .map(|c| c.kind)
    };
    assert_eq!(kind_of("d"), Some(ChangeKind::Added));
    assert_eq!(kind_of("c"), Some(ChangeKind::Removed));
    assert_eq!(kind_of("b"), Some(ChangeKind::Modified));
    assert_eq!(kind_of("a"), None, "unchanged items are not listed");
}

#[tokio::test]
async fn diff_with_no_baseline_reports_everything_added() {
    let (_tmp, provider) = fresh_driver();
    let to = seed(&provider, "src_a", 1_000, &[("a", "alpha")]);

    let report = provider.diff("src_a", None, &to.id).await.expect("diff");
    assert_eq!(report.added, 1);
    assert_eq!(report.from_snapshot_id, None);
}

#[tokio::test]
async fn diff_rejects_a_snapshot_belonging_to_another_source() {
    let (_tmp, provider) = fresh_driver();
    let from = seed(&provider, "src_a", 1_000, &[("a", "alpha")]);
    let to = seed(&provider, "src_a", 2_000, &[("a", "beta")]);

    // Both snapshots really are src_a's, so the engine's own cross-source guard
    // is satisfied — only the driver's check catches the wrong source name.
    let error = provider
        .diff("src_b", Some(&from.id), &to.id)
        .await
        .expect_err("naming the wrong source must not produce a plausible report");
    assert!(
        matches!(error, MemoryError::Invalid(ref message) if message.contains("src_a")),
        "got: {error:?}"
    );
}
