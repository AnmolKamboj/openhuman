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
pub use tinymemory_core::tree::retrieval::*;

pub mod rpc;
pub mod schemas;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_retrieval_controller_schemas,
    all_registered_controllers as all_retrieval_registered_controllers,
};
