//! Snapshot-based change tracking for memory sources.
//!
//! After each sync, this module captures what's in the chunk store for
//! that source, then diffs against previous snapshots to surface
//! additions, removals, and modifications — helping agents understand
//! how their world view has changed over time.
//!
//! Snapshots are built from already-ingested data in `mem_tree_chunks`
//! (not by re-calling source readers), making them free of API calls.
//!
//! Storage is a git repository at `<workspace>/memory_diff/repo` (the diff
//! *ledger*): snapshots are commits, checkpoints are tags, read markers are
//! refs, and diffs are git tree diffs. `mem_tree_chunks` stays authoritative;
//! the ledger is a derived view used purely for change tracking.
//!
//! W7: the snapshot/diff/checkpoint/ledger engine is now
//! `tinycortex::memory::diff::DiffEngine` (a byte-identical port over the same
//! `<workspace>/memory_diff/repo` git layout). This module is a thin host shim:
//! [`ops`] async-wraps the engine, [`source`] supplies the chunk-store item
//! seam (`DiffEngine`'s `SnapshotItemSource`), and [`rpc`]/[`schemas`]/[`tools`]
//! keep the RPC + agent surface. The wire types are the crate's, named directly
//! (`tinycortex::memory::diff::types`) rather than through a host re-export
//! module.
//!
//! Features:
//! - Per-source snapshots (auto after sync, or manual via RPC)
//! - Diff between any two snapshots
//! - Named checkpoints for cross-source "what changed since X" queries
//! - Agent tool for in-conversation diff queries

//! ## The `memory-git` gate
//!
//! All of the above needs a git ledger, and libgit2 is one of the two most
//! expensive native builds left in the graph — so the behaviour sits behind
//! `memory-git` (default-OFF, product-ON), which also carries `git2` and
//! tinycortex's `git-diff`/`wiki-git`. Off, it sheds `git2` + `libgit2-sys` +
//! `libz-sys`, taking the kernel profile from 5 native builds to 3.
//!
//! **`types` stays ungated**, mirroring the carve-out on the tinycortex side:
//! it re-exports `serde`-only wire types that always-on callers name. The
//! subconscious memory profile renders `CrossSourceDiff` and `ChangeKind` into
//! prompts, and duplicating those in a stub would be two definitions of one
//! serde shape, free to drift apart.
//!
//! The three `ops` entry points always-on code calls are stubbed rather than
//! `#[cfg]`'d at each call site, so `memory::sources::sync` and the
//! subconscious profile need no feature awareness — a diff simply never
//! materialises. Registration sites get the opposite treatment: the schema
//! aggregators return empty vecs (the controllers become unknown-method) and
//! `MemoryDiffTool` is `#[cfg]`'d out at its one registration site in
//! `tools/ops.rs`, because a registered tool that always errors is worse than
//! an absent one — the model would keep choosing it and reporting the failure.

#[cfg(feature = "memory-git")]
pub mod ops;
#[cfg(feature = "memory-git")]
pub mod rpc;
#[cfg(feature = "memory-git")]
pub mod schemas;
#[cfg(feature = "memory-git")]
pub mod source;
#[cfg(feature = "memory-git")]
pub mod tools;

#[cfg(not(feature = "memory-git"))]
mod stub;
#[cfg(not(feature = "memory-git"))]
pub use stub::{all_memory_diff_controller_schemas, all_memory_diff_registered_controllers, ops};

#[cfg(feature = "memory-git")]
pub use schemas::{
    all_controller_schemas as all_memory_diff_controller_schemas,
    all_registered_controllers as all_memory_diff_registered_controllers,
};
pub use tinycortex::memory::diff::types::{
    ChangeKind, Checkpoint, CrossSourceDiff, DiffResult, DiffSummary, ItemChange, Snapshot,
    SnapshotTrigger,
};
pub use tools::MemoryDiffTool;
