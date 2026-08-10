//! The `memory-git`-disabled surface of `memory::diff`.
//!
//! Mirrors **functions only**. The wire types stay in [`super::types`] and are
//! compiled in both directions, so — unlike the `voice` stub, which had to
//! re-declare types living inside its gated tree — there is zero type
//! duplication here and nothing that can drift.
//!
//! Only the three entry points that always-on code reaches are mirrored:
//!
//! | Caller | Function |
//! | --- | --- |
//! | `memory::sources::sync` | `auto_snapshot_after_sync` |
//! | `subconscious::profiles::memory` | `diff_since_checkpoint`, `create_checkpoint` |
//!
//! Everything else in the real `ops` is reached only from inside this module's
//! own gated files, so it needs no mirror. If you add a cross-domain caller,
//! add its function here rather than `#[cfg]`-ing the call site — keeping
//! feature awareness out of always-on domains is the whole point of the stub.
//!
//! **These return `Err`, not `Ok`-with-empty.** An empty `CrossSourceDiff`
//! would say "your world did not change", which the subconscious profile would
//! faithfully act on; an error says "this build cannot tell you", which it
//! already knows how to log and skip. Failing closed matters more than being
//! quiet: the caller in `profiles/memory.rs` logs and moves on.

use crate::openhuman::config::Config;
use crate::openhuman::memory::sources::types::MemorySourceEntry;

use super::types::{Checkpoint, CrossSourceDiff, Snapshot};

/// The message every disabled entry point returns.
///
/// Names the feature, because the reader is a developer looking at a log line
/// from a slim build and the actionable fact is which gate to turn on.
const DISABLED: &str = "memory diff is disabled at compile time (built without the `memory-git` \
                        feature); rebuild with `--features memory-git` for git-backed snapshots, \
                        checkpoints and diffs";

/// Function mirrors of the real [`super::ops`].
pub mod ops {
    use super::*;

    /// See [`super::super::ops::auto_snapshot_after_sync`].
    pub async fn auto_snapshot_after_sync(
        _source: &MemorySourceEntry,
        _config: &Config,
    ) -> Result<Snapshot, String> {
        Err(DISABLED.to_string())
    }

    /// See [`super::super::ops::create_checkpoint`].
    pub async fn create_checkpoint(_label: &str, _config: &Config) -> Result<Checkpoint, String> {
        Err(DISABLED.to_string())
    }

    /// See [`super::super::ops::diff_since_checkpoint`].
    pub async fn diff_since_checkpoint(
        _checkpoint_id: &str,
        _config: &Config,
        _include_text_diff: bool,
    ) -> Result<CrossSourceDiff, String> {
        Err(DISABLED.to_string())
    }
}

/// No controllers: the `memory_diff` namespace answers unknown-method.
///
/// Empty rather than a set of always-erroring handlers, so `/schema` does not
/// advertise a surface this build cannot serve.
pub fn all_memory_diff_controller_schemas() -> Vec<crate::core::ControllerSchema> {
    Vec::new()
}

/// No controllers to register. See [`all_memory_diff_controller_schemas`].
pub fn all_memory_diff_registered_controllers() -> Vec<crate::core::all::RegisteredController> {
    Vec::new()
}
