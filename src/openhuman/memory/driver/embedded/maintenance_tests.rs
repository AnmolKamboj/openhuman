//! [`MemoryMaintenance`] tests.
//!
//! The point of most of these is *honesty*, not throughput: each operation must
//! either do the work it names or say in `findings` that it did not. A report
//! with `changed: 0` and an empty `findings` list reads as "ran, nothing to do"
//! and is exactly what these tests exist to prevent.

use super::super::test_support::fresh_driver;
use super::*;

#[tokio::test]
async fn doctor_never_reports_changed_and_names_itself() {
    let (_tmp, provider) = fresh_driver();
    let report = provider.doctor().await.expect("doctor");

    assert_eq!(report.operation, "doctor");
    assert_eq!(
        report.changed, 0,
        "the contract requires doctor to be read-only"
    );
}

#[tokio::test]
async fn doctor_is_repeatable_and_stays_read_only() {
    let (_tmp, provider) = fresh_driver();
    let first = provider.doctor().await.expect("first doctor");
    let second = provider.doctor().await.expect("second doctor");
    assert_eq!(first.changed, 0);
    assert_eq!(second.changed, 0);
    assert_eq!(first.examined, second.examined);
}

#[tokio::test]
async fn reembed_reports_an_enqueue_rather_than_a_run() {
    let (_tmp, provider) = fresh_driver();
    let report = provider.reembed().await.expect("reembed");

    assert_eq!(report.operation, "reembed");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.contains("asynchronously") && f.contains("queue")),
        "the report must not imply the re-embed already happened: {:?}",
        report.findings
    );
}

#[tokio::test]
async fn compact_states_that_no_vacuum_exists() {
    let (_tmp, provider) = fresh_driver();
    let report = provider.compact().await.expect("compact");

    assert_eq!(report.operation, "compact");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.contains("no index vacuum")),
        "compact must not silently under-deliver against the contract: {:?}",
        report.findings
    );
    assert!(
        !report.findings.is_empty(),
        "an empty findings list would read as 'ran, nothing to do'"
    );
}

#[tokio::test]
async fn consolidate_enqueues_once_per_window_and_says_so_the_second_time() {
    let (_tmp, provider) = fresh_driver();

    let first = provider.consolidate().await.expect("first consolidate");
    assert_eq!(first.operation, "consolidate");
    assert_eq!(first.changed, 1, "the first call enqueues a flush");
    assert!(first.findings.iter().any(|f| f.contains("enqueued")));

    // Deduped on (date, 3-hour block). A second call inside the window really
    // did nothing, and must not claim otherwise.
    let second = provider.consolidate().await.expect("second consolidate");
    assert_eq!(second.changed, 0);
    assert!(
        second.findings.iter().any(|f| f.contains("already queued")),
        "got: {:?}",
        second.findings
    );
}

#[tokio::test]
async fn every_operation_labels_itself_with_its_own_name() {
    let (_tmp, provider) = fresh_driver();
    // A copy-paste `operation` string is the kind of thing only an explicit
    // check catches — the reports are otherwise shaped identically.
    assert_eq!(
        provider.reembed().await.expect("reembed").operation,
        "reembed"
    );
    assert_eq!(
        provider.compact().await.expect("compact").operation,
        "compact"
    );
    assert_eq!(
        provider.consolidate().await.expect("consolidate").operation,
        "consolidate"
    );
    assert_eq!(provider.doctor().await.expect("doctor").operation, "doctor");
}
