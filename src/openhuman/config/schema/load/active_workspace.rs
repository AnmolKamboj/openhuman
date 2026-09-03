//! The active workspace, cached in memory (#5966).
//!
//! [`active_workspace_dir`](super::dirs::active_workspace_dir) resolves the
//! workspace through the loader's own path, which means reading a marker
//! file. That is the right cost for a *decision* — the notification bridge
//! pays it for the handful of workspace-bound events a supervisor tick
//! produces — and the wrong cost for a *stream*. The developer Event Log in
//! [`crate::core::jsonrpc`] has to stamp every domain event the process
//! publishes, and its `tokio_stream` `filter_map` closure is synchronous, so
//! it could not await a disk read even if the cost were acceptable.
//!
//! This module is the cheap answer: a process-global slot holding the last
//! workspace the loader resolved, readable synchronously and without I/O.
//!
//! # The disk stays the source of truth
//!
//! The cache is never authoritative. It is written *through* — every
//! successful resolution publishes its own answer here — and cleared
//! whenever one of the markers that decides the answer is rewritten, so the
//! next reader that can afford a resolve refills it. A stale value is
//! therefore not a thing this can hold: the marker writes and the resolves
//! are the only two ways the answer changes, and both touch this slot.
//!
//! `None` means "not resolved since the last change", not "no workspace".
//! Callers that cannot resolve must treat it as unknown rather than as a
//! mismatch — see the Event Log's handling in `core::jsonrpc`.
//!
//! # Why the env-injectable loader does not publish
//!
//! [`Config::load_or_init_with_env_lookup`](crate::openhuman::config::Config)
//! takes an [`EnvLookup`](super::env::EnvLookup) so tests can exercise the
//! `OPENHUMAN_WORKSPACE` branch without mutating the process environment.
//! Publishing from there would let one test's fixture directory become the
//! whole binary's idea of the active workspace. Only the two `ProcessEnv`
//! entry points publish: `Config::load_or_init` and `active_workspace_dir`.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use once_cell::sync::Lazy;

const LOG_PREFIX: &str = "[config:active-workspace]";

/// The state machine, separated from the process-global slot that holds it.
///
/// The decisions here — when a resolve is a *change*, what invalidation does
/// and does not clear — are the part worth asserting, and asserting them on
/// the global would be unsound: `Config::load_or_init` publishes into that
/// global, and the test binary runs thousands of tests in parallel, many of
/// which load a config. A test that pinned the global would pass alone and
/// fail whenever it happened to interleave with one of those.
#[derive(Default)]
struct ActiveWorkspace {
    /// The last resolved workspace, or `None` when a marker write has
    /// invalidated it and nothing has resolved since.
    current: Option<PathBuf>,
    /// The workspace last announced on the bus.
    ///
    /// Kept separately from `current`, and deliberately **not** cleared by
    /// invalidation. A marker write does not always change the answer —
    /// signing in as the user who is already active rewrites
    /// `active_user.toml` with the same id — and without this the re-resolve
    /// that follows would announce a switch that never happened, putting a
    /// phantom row in the Event Log and making a real switch harder to spot.
    announced: Option<PathBuf>,
}

impl ActiveWorkspace {
    /// Record `workspace_dir` as current. Returns `true` when this is a
    /// change the bus has not been told about yet.
    fn publish(&mut self, workspace_dir: &Path) -> bool {
        self.current = Some(workspace_dir.to_path_buf());
        if self.announced.as_deref() == Some(workspace_dir) {
            return false;
        }
        self.announced = Some(workspace_dir.to_path_buf());
        true
    }

    /// Forget the resolved answer because a marker that decides it was
    /// written. Leaves `announced` alone — see the field's own note.
    fn invalidate(&mut self) -> bool {
        self.current.take().is_some()
    }
}

static ACTIVE_WORKSPACE: Lazy<RwLock<ActiveWorkspace>> =
    Lazy::new(|| RwLock::new(ActiveWorkspace::default()));

/// Record `workspace_dir` as the workspace this process is serving.
///
/// Called after a resolution that used the real process environment.
/// Publishes [`DomainEvent::ActiveWorkspaceChanged`](crate::core::events::DomainEvent)
/// when the value actually changes, so consumers of a long-lived stream learn
/// about a switch without polling — and so the switch itself becomes a
/// visible row in the Event Log rather than an invisible reason its contents
/// changed.
pub(crate) fn publish_active_workspace(workspace_dir: &Path) {
    let announce = match ACTIVE_WORKSPACE.write() {
        Ok(mut guard) => guard.publish(workspace_dir),
        Err(error) => {
            log::warn!("{LOG_PREFIX} active workspace slot poisoned: {error}");
            return;
        }
    };

    if !announce {
        return;
    }

    log::info!(
        "{LOG_PREFIX} active workspace is now {}",
        workspace_dir.display()
    );
    // Published with no lock held: a subscriber is free to reach back into
    // the config layer, and holding the slot across the publish would make
    // that a deadlock.
    crate::core::bus::BUS.publish(crate::core::events::DomainEvent::ActiveWorkspaceChanged {
        workspace_dir: workspace_dir.to_path_buf(),
    });
}

/// The active workspace as last resolved, or `None` when a marker has been
/// rewritten since and nothing has resolved yet.
///
/// Synchronous and I/O-free — safe on a hot path. A caller that can afford
/// the disk read should use
/// [`active_workspace_dir`](super::dirs::active_workspace_dir) instead,
/// which is authoritative and refills this slot as a side effect.
pub fn active_workspace_dir_cached() -> Option<PathBuf> {
    match ACTIVE_WORKSPACE.read() {
        Ok(guard) => guard.current.clone(),
        Err(error) => {
            log::warn!("{LOG_PREFIX} active workspace slot poisoned: {error}");
            None
        }
    }
}

/// Drop the cached value because a marker that decides it was just written.
///
/// Clearing rather than overwriting is deliberate: a marker write says the
/// answer changed, not what it changed *to*. `active_user.toml` names a user
/// id, and turning that into a workspace is the resolver's job, including
/// the fallbacks that apply when the marker is absent or unreadable.
/// Guessing here would put a second, subtly different resolution rule in the
/// codebase — the failure mode #5334 came from.
pub(crate) fn invalidate_active_workspace() {
    match ACTIVE_WORKSPACE.write() {
        Ok(mut guard) => {
            if guard.invalidate() {
                log::debug!("{LOG_PREFIX} cleared after a workspace marker write");
            }
        }
        Err(error) => log::warn!("{LOG_PREFIX} active workspace slot poisoned: {error}"),
    }
}

#[cfg(test)]
#[path = "active_workspace_tests.rs"]
mod tests;
