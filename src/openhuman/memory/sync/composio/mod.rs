//! Host layer over [`tinymemory_core::sync::composio`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::sync::composio::…` path resolving.
//!
//! # This shim cannot go yet, and the reason is upstream, not here
//!
//! The glob below is what makes `integrations::composio::start_periodic_sync`
//! resolve, and therefore what lets `core/runtime/services.rs` start an engine
//! sync loop inside the host process. openhuman#5560 deletes the second,
//! in-process engine and leaves the loaded TinyMemory module as the only one,
//! so the obvious reading is that this file goes with it.
//!
//! **It does not, and this was checked against the pinned artifact rather than
//! assumed.** `modules::registry` pins `tinymemory` v1.5.0, whose module
//! carries a section headed *"The periodic sync loops are deliberately NOT
//! started here"* (`tinymemory-module/src/lib.rs`, right after
//! `start_queue_pool`). It names three reasons the loops cannot move yet, none
//! of which a host change can close:
//!
//! 1. The pipeline reads Composio credentials off `Config`, not off the
//!    `ComposioHost` seam — `composio_config` takes the direct-mode branch only
//!    when `config.composio().mode == "direct"` and otherwise wants
//!    `config.session_token()`, and the module's `EngineRuntimeConfig` answers
//!    neither. Routing it through `ComposioHost::execute` is a change to the
//!    *engine's* contract.
//! 2. `global::client_if_ready()` is `None` inside the module — it builds its
//!    store through `create_memory_client_with_local_ai`, which never touches
//!    the global slot.
//! 3. `EngineRuntimeConfig::memory_sync_interval_secs()` answers `Some(0)`,
//!    which the contract defines as *manual only*, so every tick would skip
//!    every source silently.
//!
//! The queue pool **did** move in v1.5.0, which is why `services.rs` no longer
//! calls `queue::start` — do not read that as precedent for these two.
//!
//! So the ordering stands, with one step still open. Composio sync degrades
//! **quietly** — an unwired `ComposioHost` makes `is_available` read `false`
//! and a sync run report zero connections, which nobody can tell apart from a
//! user with none. A half-removed loop looks like a working one for as long as
//! it takes someone to notice their mail stopped being indexed.
//!
//! The full ordered list lives next to the call site it protects, in
//! `core/runtime/services.rs`'s `start_bootstrap_jobs`. In short: the host
//! serves `ComposioHost` on the module bus (**done** — `modules/memory_host.rs`),
//! the module installs a bus-backed seam for it (**done** in v1.5.0 —
//! `composio::BusComposioHost`) **and starts the loop itself** (⛔ *not done*,
//! for the three reasons above), a `tinymemory` release carries all of it and
//! `modules::registry` is re-pinned to its digest — **then** this file and that
//! call go.

pub use tinymemory_core::sync::composio::*;

pub mod providers;

pub mod bus;

pub use bus::{
    register_composio_trigger_subscriber, ComposioConfigChangedSubscriber,
    ComposioTriggerSubscriber,
};
