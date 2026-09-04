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

use std::time::Duration;

/// What one driver's shutdown may take.
///
/// Releasing a lease is one write per in-flight job. Anything slower is a
/// store that is not going to answer, and its leases expire by themselves.
pub const DRIVER_EXIT_BUDGET: Duration = Duration::from_secs(2);

/// Ask every bound memory driver to shut down, then run the hooks the
/// in-process engine registered with the host.
///
/// Providers first: a module drains its own banked hook inside `Shutdown`, and
/// the host's registry is where the in-process engine (dev runs and tests)
/// registers instead. Both are idempotent — a provider's second `shutdown` is a
/// no-op, and the registry drains — so a signal landing mid-teardown, or the
/// app-update restart path calling this twice, repeats nothing.
pub async fn shutdown_for_exit() {
    for binding in crate::openhuman::memory::binding::cached_bindings() {
        let driver = binding.driver_id().to_string();
        match tokio::time::timeout(DRIVER_EXIT_BUDGET, binding.provider().shutdown()).await {
            Ok(Ok(())) => log::debug!("[memory:exit] driver '{driver}' shut down"),
            Ok(Err(error)) => {
                log::warn!("[memory:exit] driver '{driver}' shutdown failed: {error}");
            }
            Err(_) => log::warn!(
                "[memory:exit] driver '{driver}' shutdown exceeded {DRIVER_EXIT_BUDGET:?}; \
                 proceeding with exit"
            ),
        }
    }
    crate::core::shutdown::run_hooks_now().await;
}
