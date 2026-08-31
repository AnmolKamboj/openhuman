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
//! 3. **`sync` and `status` are wired into pieces that have not moved.** This
//!    one still stands, and is why those two are still reached from the engine
//!    by name below rather than through a glob. `reconcile` was on this line as
//!    well and has since come home — see below.
//!
//! ## What is still the engine's, and why each one
//!
//! - [`sync`] reaches `engine::run_source_pipeline`, `engine::{needs_rebuild,
//!   rebuild_tree_from_raw}`, `queue::store::retry_all_failed`,
//!   `sync::composio` and `sync::audit` — the ingest pipeline, the re-embed
//!   queue and the Composio sync half. Only `derive_scopes` is reached from
//!   production today (`rpc::reconcile_rpc`); `sync_source` itself has no
//!   caller left in `src/`, because the sync the product runs goes over the bus
//!   through `MemorySourceSync`. The module is kept whole rather than narrowed
//!   to the one live function: a re-export that hides which half is dead would
//!   make the next audit harder, not easier.
//! - [`status`] reaches `store::chunks::store::with_connection`, the raw SQLite
//!   chunk door, and is live in production behind
//!   `memory_sources.status_list`. **`MemoryChunks::source_totals` is not a
//!   substitute**, and this is worth stating because it looks like one:
//!   `SourceTotal` carries `chunk_count` and `most_recent_ms` but no
//!   `chunks_pending`, and pending — "has no embedding, was not dropped, was
//!   not skipped for re-embed" — is the whole point of a sync-status row.
//!   `source_totals` also omits a source with zero chunks entirely, where
//!   `status_list` returns a row per *configured* source. Migrating onto it
//!   would compile and quietly report a healthy store. The ask upstream is a
//!   pending count.
//!
//! Porting those two as they stand would move the ingest pipeline and the raw
//! chunk door *into* the host rather than behind the bus, which is the opposite
//! of what #5560 is for: a second unpoliced door spelled differently is still a
//! second unpoliced door.
//!
//! ## What came home, and why it was different
//!
//! [`reconcile`] was on the blocked list and no longer is. Both of its halves
//! read the `[[memory_sources]]` table in **this host's own config file** and
//! nothing below it: the scan came home when tinymemory v1.13.4 deleted the
//! in-process Composio pipeline it used to call, and
//! `apply_composio_source_caps_migration` followed in #5560. That is the line
//! between the two lists — a config rewrite is host work that happened to live
//! upstream, where an ingest pipeline and a SQLite cursor are not.
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
pub use tinymemory_core::sources::{status, sync};

// `reconcile` used to be entirely the engine's. tinymemory v1.13.4 deleted
// `ensure_composio_sources` along with the rest of the in-process Composio
// pipeline it scanned (`sync::composio::scan_active_sync_targets`), so this
// host carries its own — built on
// `memory::sync::composio::scan_active_sync_targets`, the tinyconnectors
// replacement. `apply_composio_source_caps_migration` followed it home in
// #5560: it never touched the deleted pipeline, only this host's config file,
// and reaching it through the engine bought a compile-time link to the engine
// for a `config.toml` rewrite.
pub mod reconcile;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_memory_sources_controller_schemas,
    all_registered_controllers as all_memory_sources_registered_controllers,
};
