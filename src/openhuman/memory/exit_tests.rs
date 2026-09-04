//! Tests for the memory exit path.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::exit::{shutdown_for_exit, EXIT_BUDGET};

/// The hook registry is process-global, so the two tests below must not see
/// each other's hooks: each registers and drains under this lock.
static REGISTRY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// With no binding ever built, the exit path is the hook registry alone — and
/// the registry drains, so the hooks run once however many times exit is asked
/// for (the app-update restart path asks twice).
#[tokio::test]
async fn exit_runs_registered_hooks_once() {
    let _serial = REGISTRY.lock().await;
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

/// A hook that never answers must not hold the quit: exit returns within its
/// budget (plus the hooks' floor) and the process goes on without it.
#[tokio::test]
async fn exit_is_bounded_even_when_a_hook_hangs() {
    let _serial = REGISTRY.lock().await;
    crate::core::shutdown::register(|| async {
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let started = std::time::Instant::now();
    shutdown_for_exit().await;

    assert!(
        started.elapsed() < EXIT_BUDGET + Duration::from_secs(2),
        "exit took {:?}; the hanging hook held it past the budget",
        started.elapsed()
    );
}
