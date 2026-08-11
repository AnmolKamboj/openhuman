//! Host layer over [`tinymemory_core::goals`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::goals::…` path resolving.

pub use tinymemory_core::goals::*;

pub mod ops;
pub mod schemas;

pub use schemas::{all_memory_goals_controller_schemas, all_memory_goals_registered_controllers};

/// The reflection agent that maintains the goals list over several turns. It
/// drives an `Agent` through the harness registry, so it stays host-side.
pub mod enrich;

pub use enrich::{enrich_goals, spawn_enrich_goals, GOALS_AGENT_ID};

// The controller aggregators this domain's RPC surface defines.
pub use schemas::*;
