//! Host layer over [`tinymemory_core::sync::composio`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::sync::composio::…` path resolving.
//!
//! # This shim outlives its usefulness the day the in-process engine goes
//!
//! The glob below is what makes `integrations::composio::start_periodic_sync`
//! resolve, and therefore what lets `core/runtime/services.rs` start an engine
//! sync loop inside the host process. openhuman#5560 deletes the second,
//! in-process engine and leaves the loaded TinyMemory module as the only one;
//! after that this re-export names a crate the host no longer has a live engine
//! for, and the periodic loop it exposes has nothing to run against.
//!
//! It must not be deleted first, and the ordering is the point. Composio sync
//! degrades **quietly** — an unwired `ComposioHost` makes `is_available` read
//! `false` and a sync run report zero connections, which nobody can tell apart
//! from a user with none. So a half-removed loop looks like a working one for
//! as long as it takes someone to notice their mail stopped being indexed.
//!
//! The full ordered list lives next to the call site it protects, in
//! `core/runtime/services.rs`'s `start_bootstrap_jobs`. In short: the host
//! serves `ComposioHost` on the module bus (done — `modules/memory_host.rs`),
//! the module installs a bus-backed seam for it and starts the loop itself, a
//! `tinymemory` release carries both and `modules::registry` is re-pinned to
//! its digest — **then** this file and that call go, together with the
//! in-process engine.

pub use tinymemory_core::sync::composio::*;

pub mod providers;

pub mod bus;

pub use bus::{
    register_composio_trigger_subscriber, ComposioConfigChangedSubscriber,
    ComposioTriggerSubscriber,
};
