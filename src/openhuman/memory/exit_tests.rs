//! Tests for the memory exit path.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::exit::shutdown_for_exit;

/// With no binding ever built, the exit path is the hook registry alone — and
/// the registry drains, so the hooks run once however many times exit is asked
/// for (the app-update restart path asks twice).
#[tokio::test]
async fn exit_runs_registered_hooks_once() {
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    crate::core::shutdown::register(move || {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    });

    shutdown_for_exit().await;
    shutdown_for_exit().await;

    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "the registry drains on the first exit; the second finds nothing"
    );
}
