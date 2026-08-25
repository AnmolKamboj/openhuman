//! Host layer over [`tinymemory_core::tree::retrieval`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::tree::retrieval::…` path resolving.

// Engine-owned, and the names are a trap worth naming: `RetrievalHit`,
// `QueryResponse`, `NodeKind` and `EntityMatch` here are **not** the contract's
// `RetrievalHit`, `RetrievalResponse`, `RetrievalNodeKind` and `EntityMatch`.
// The engine's hit carries a `tree_kind` the contract's has no field for, so
// repointing this shim at `memory::api::provider` is a wire change and not a
// path swap. Probe the item identity before assuming a carve-out is free.
//
// What keeps this line alive (#5560): `fast_retrieve` + `FastRetrieveOptions`
// (memory::agent::ops, memory::schema::handlers), `types::{NodeKind,
// QueryResponse, RetrievalHit}` (read_rpc::chunks, the sub-agent runner),
// `source::{SourceQuery, query_source_scoped}` (read_rpc::chunks) and
// `search::search_entities` — plus `{query_source, search_entities}` from the
// memory e2e suites. Every one names a shape the contract does not carry, so
// this is the same wire-change ask as the paragraph above, not a routing one.
pub use tinymemory_core::tree::retrieval::*;

pub mod rpc;
pub mod schemas;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_retrieval_controller_schemas,
    all_registered_controllers as all_retrieval_registered_controllers,
};
