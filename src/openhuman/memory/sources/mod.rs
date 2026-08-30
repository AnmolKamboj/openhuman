//! Memory sources: the registry of connectors this workspace ingests from, the
//! readers that pull items out of them, and the JSON-RPC surface over both.
//!
//! # This module no longer globs the engine (#5560)
//!
//! It used to be `pub use tinymemory_core::sources::*;`, which made every
//! source read in the host a compile-time link to the memory engine. The
//! previous revision of these docs listed three things standing in the way, and
//! two of them are now done:
//!
//! 1. **The types had a home this crate did not depend on.** They are
//!    `tinymemory-sources`', an engine-neutral crate. It is a direct dependency
//!    now — it costs no crate this manifest did not already have (no
//!    `rusqlite`, no `tinycortex`), so the unlock really was the one
//!    `Cargo.toml` line the note predicted.
//! 2. **Two of the seven readers had no upstream twin.** `composio` and
//!    `twitter` were implemented in the engine crate but named nothing
//!    engine-shaped, so they came home unchanged. See [`readers`].
//! 3. **`sync`, `status` and `reconcile` are wired into pieces that have not
//!    moved.** This one still stands, and is why those three are still reached
//!    from the engine by name below rather than through a glob.
//!
//! ## What is still the engine's, and why each one
//!
//! - [`sync`] reaches `engine::run_source_pipeline`, `engine::{needs_rebuild,
//!   rebuild_tree_from_raw}`, `queue::store::retry_all_failed`,
//!   `sync::composio` and `sync::audit` — the ingest pipeline, the re-embed
//!   queue and the Composio sync half.
//! - [`status`] reaches `store::chunks::store::with_connection`, the raw SQLite
//!   chunk door. **`MemoryChunks::source_totals` is not a substitute**, and
//!   this is worth stating because it looks like one: `SourceTotal` carries
//!   `chunk_count` and `most_recent_ms` but no `chunks_pending`, and pending —
//!   "has no embedding, was not dropped, was not skipped for re-embed" — is the
//!   whole point of a sync-status row. `source_totals` also omits a source with
//!   zero chunks entirely, where `status_list` returns a row per *configured*
//!   source. Migrating onto it would compile and quietly report a healthy
//!   store. The ask upstream is a pending count.
//! - [`reconcile`] is Composio source reconciliation, so it moves with
//!   `sync::composio`.
//!
//! Porting those as they stand would move the ingest pipeline and the raw
//! chunk door *into* the host rather than behind the bus, which is the opposite
//! of what #5560 is for: a second unpoliced door spelled differently is still a
//! second unpoliced door.
//!
//! `MemorySourceSink` is not the answer for the registry either — it is
//! `accept_source_items` + `forget_source` + `forget_matching`, an *ingest*
//! door with no listing or CRUD member for a configured connector, and it is
//! the whole of `Capability::Sources`. A listing member would be an upstream
//! ask; the registry did not need one, because the file it reads is this
//! host's own.

pub mod readers;
pub mod registry;
pub mod rpc;
pub mod schemas;

/// The source vocabulary, from the crate that defines it.
pub mod types {
    pub use tinymemory_sources::types::{
        ContentType, MemorySourceEntry, SourceContent, SourceItem, SourceKind,
    };
}

pub use registry::{
    apply_kind_defaults, list_sources, memory_sync_defaults_for_toolkit, upsert_composio_source,
    ComposioUpsertTarget, MemorySourcePatch,
};
pub use types::{ContentType, MemorySourceEntry, SourceContent, SourceItem, SourceKind};

// ── Still the engine's ──────────────────────────────────────────────────────
//
// Named rather than globbed, so `grep tinymemory_core` in this domain is an
// honest inventory of what is left. See the module docs for what blocks each.
pub use tinymemory_core::sources::{reconcile, status, sync};

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_memory_sources_controller_schemas,
    all_registered_controllers as all_memory_sources_registered_controllers,
};
