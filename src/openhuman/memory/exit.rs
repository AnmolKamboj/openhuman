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

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::time::Instant;

use crate::openhuman::memory::binding::MemoryBinding;

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

/// Rounds of "snapshot the cache, shut down what is new". Two is the honest
/// number: the first round covers everything built before exit began, the
/// second anything a background task built while the first round ran.
const SNAPSHOT_ROUNDS: usize = 2;

/// Ask every bound memory driver to shut down, then run the hooks the
/// in-process engine registered with the host.
///
/// Providers first, concurrently and on the shared deadline: a module drains
/// its own banked hook inside `Shutdown`, and the host's registry is where the
/// in-process engine registers instead. The binding cache is snapshotted, not
/// locked: a build that overlaps exit inserts after the snapshot, so a second
/// round picks up whatever appeared. Both halves are idempotent — a provider's
/// second `shutdown` is a no-op and the registry drains — so a signal landing
/// mid-teardown, or the app-update restart path calling this twice, repeats
/// nothing.
pub async fn shutdown_for_exit() {
    let deadline = Instant::now() + EXIT_BUDGET;
    // Addresses, not pointers: a raw pointer in the set would make this future
    // `!Send`, and the embedded server task that awaits it is spawned. The
    // `Arc` snapshot keeps every binding alive across the loop, so an address
    // cannot be reused for a different binding while it is in the set.
    let mut seen: HashSet<usize> = HashSet::new();
    for round in 1..=SNAPSHOT_ROUNDS {
        let pending: Vec<Arc<MemoryBinding>> = crate::openhuman::memory::binding::cached_bindings()
            .into_iter()
            .filter(|binding| seen.insert(Arc::as_ptr(binding) as usize))
            .collect();
        if pending.is_empty() {
            break;
        }
        let shutdowns = join_all(pending.iter().map(|binding| async move {
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
                "[memory:exit] driver shutdown exceeded the {EXIT_BUDGET:?} exit budget in \
                 round {round}; proceeding with exit"
            );
            break;
        }
    }

    let hooks_deadline = deadline.max(Instant::now() + HOOKS_FLOOR);
    if tokio::time::timeout_at(hooks_deadline, crate::core::shutdown::run_hooks_now())
        .await
        .is_err()
    {
        log::warn!("[memory:exit] shutdown hooks exceeded the exit budget; proceeding with exit");
    }
}
