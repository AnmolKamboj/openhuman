//! Host layer over [`tinymemory_core::sources`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::sources::…` path resolving.
//!
//! # Why this glob is still here (#5560), stated so nobody re-derives it
//!
//! On its face this looks like the easiest facade in the tree to bring home:
//! the registry it fronts is persisted in **this host's** `config.toml` under
//! `[[memory_sources]]`, and `tinymemory_core::sources` is a thin layer —
//! `types` is `pub use tinymemory_sources::…`, `registry` wraps
//! `tinymemory_sources::registry::SourceRegistry`, and the readers wrap
//! `tinymemory_sources::readers`. Two things block moving it, and both are
//! outside this file:
//!
//! 1. **The types have a home this crate does not depend on.** They are
//!    `tinymemory-sources`' — an engine-neutral crate the root manifest does
//!    not list. Bringing the domain here needs that dependency added, which is
//!    a `Cargo.toml` decision, not a code one.
//! 2. **`sources::sync` is wired into the pieces #5560 has not moved yet.**
//!    Its call graph reaches `engine::run_source_pipeline`,
//!    `queue::store::retry_all_failed`,
//!    `store::chunks::store::with_connection`, `sync::composio` and
//!    `sync::audit` — the ingest pipeline, the re-embed queue and the raw
//!    SQLite chunk door. Porting the domain as it stands would move those into
//!    the host rather than behind the bus, which is the opposite of what the
//!    issue is for: a second unpoliced door spelled differently is still a
//!    second unpoliced door.
//!
//! So this stays until the queue and the sync pipeline go behind the module.
//! `MemorySourceSink` is not the answer either — it is `accept_source_items` +
//! `forget_source` + `forget_matching`, an *ingest* door, with no listing or
//! CRUD member for a configured connector.

pub use tinymemory_core::sources::*;

pub mod rpc;
pub mod schemas;

pub use tinymemory_core::sources::apply_kind_defaults;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_memory_sources_controller_schemas,
    all_registered_controllers as all_memory_sources_registered_controllers,
};
