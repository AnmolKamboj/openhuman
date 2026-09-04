//! What memory does on the way out of the process.
//!
//! The engine registers exactly one shutdown hook — its queue worker releasing
//! the leases on in-flight jobs, so the next launch re-claims that work instead
//! of waiting the leases out (tinymemory#133). In module mode the engine banks
//! that hook and drains it when the host calls `Shutdown` (tinymemory#137).
//! The host never did. The embedded server's graceful path is a cancellation
//! token, not SIGTERM, so [`crate::core::shutdown::signal`] never resolved
//! there, and nothing else ever called the bound provider's `shutdown`: every
//! normal quit left the leases held, and every next launch took the slow path.
//!
//! This is the other half. It runs from the server's post-drain block, and it
//! is bounded, because a quit that hangs on a wedged store is worse than a
//! lease that expires on its own.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::future::join_all;
use tokio::time::Instant;

/// The whole of memory's exit work — every bound driver's `shutdown`, then
/// the host's hook registry — must fit in this, together.
///
/// One deadline rather than one per driver. The Tauri shell gives the server
/// a bounded moment to drain before it aborts the task, and a per-driver
/// budget multiplied by the number of bindings could outrun that moment with
/// a hook still pending. Releasing a lease is one write per in-flight job;
/// anything slower is a store that is not going to answer, and its leases
/// expire by themselves. The shell sizes its drain budget from this constant
/// plus the ollama cleanup that follows it in `serve_http`.
pub const EXIT_BUDGET: Duration = Duration::from_secs(2);

/// The least the hook registry gets even when the drivers spent the budget.
/// In-process engines (dev runs, tests) release their leases through a hook,
/// not a driver, so the hooks are never skipped outright.
const HOOKS_FLOOR: Duration = Duration::from_millis(250);

/// Whether a real server has started in this process.
///
/// The exit gate below engages only then. Unit tests call
/// [`shutdown_for_exit`] without ever serving, and a process-wide refusal to
/// bind memory would reach every other test in the same process.
static SERVING: AtomicBool = AtomicBool::new(false);

/// Set when memory's exit work begins, cleared when a server starts.
///
/// Read by the binding cache *inside its write lock* before every insert. That
/// ordering is what makes the exit snapshot complete: the flag goes up, then
/// the snapshot is taken under the same lock, so a builder either inserted
/// before the snapshot (and is in it) or sees the flag and is refused. No
/// number of re-snapshots could say that.
static EXITING: AtomicBool = AtomicBool::new(false);

/// Called by the embedded server as it starts serving. Arms the exit gate for
/// the eventual exit and clears one left by a previous server in this process
/// (the shell can respawn the core task without restarting the process).
pub fn server_starting() {
    EXITING.store(false, Ordering::SeqCst);
    SERVING.store(true, Ordering::SeqCst);
}

/// Whether memory is on its way out: new bindings are refused.
pub(crate) fn exiting() -> bool {
    EXITING.load(Ordering::SeqCst)
}

/// Test seam: put both flags back so one test's exit cannot leak into another.
#[cfg(test)]
pub(crate) fn reset_gate_for_tests() {
    EXITING.store(false, Ordering::SeqCst);
    SERVING.store(false, Ordering::SeqCst);
}

/// Ask every bound memory driver to shut down, then run the hooks the
/// in-process engine registered with the host.
///
/// Providers first, concurrently and on the shared deadline: a module drains
/// its own banked hook inside `Shutdown`, and the host's registry is where the
/// in-process engine registers instead. The snapshot of the binding cache is
/// complete rather than merely recent: the exit gate goes up first, and the
/// cache checks it under its write lock before every insert, so nothing can
/// appear after the snapshot. Both halves are idempotent — a provider's second
/// `shutdown` is a no-op and the registry drains — so a signal landing
/// mid-teardown, or the app-update restart path calling this twice, repeats
/// nothing. The drivers it shut down leave the cache afterwards, so the
/// server the shell starts next in this same process binds anew instead of
/// reusing a driver whose workers are stopped.
pub async fn shutdown_for_exit() {
    if SERVING.load(Ordering::SeqCst) {
        EXITING.store(true, Ordering::SeqCst);
    }
    let deadline = Instant::now() + EXIT_BUDGET;

    let bindings = crate::openhuman::memory::binding::cached_bindings();
    if !bindings.is_empty() {
        let shutdowns = join_all(bindings.iter().map(|binding| async move {
            let driver = binding.driver_id().to_string();
            match binding.provider().shutdown().await {
                Ok(()) => log::debug!("[memory:exit] driver '{driver}' shut down"),
                Err(error) => {
                    log::warn!("[memory:exit] driver '{driver}' shutdown failed: {error}");
                }
            }
        }));
        if tokio::time::timeout_at(deadline, shutdowns).await.is_err() {
            log::warn!(
                "[memory:exit] driver shutdown exceeded the {EXIT_BUDGET:?} exit budget; \
                 proceeding with exit"
            );
        }
        // Out of the cache either way — answered or timed out, these drivers
        // have been told to stop. The shell restarts the embedded server in
        // place; a server starting again in this process must bind fresh
        // drivers, never be handed back ones whose workers exit already
        // stopped.
        crate::openhuman::memory::binding::evict_bindings(&bindings);
    }

    let hooks_deadline = deadline.max(Instant::now() + HOOKS_FLOOR);
    if tokio::time::timeout_at(hooks_deadline, crate::core::shutdown::run_hooks_now())
        .await
        .is_err()
    {
        log::warn!("[memory:exit] shutdown hooks exceeded the exit budget; proceeding with exit");
    }
}
