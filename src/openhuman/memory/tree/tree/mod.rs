//! Host layer over [`tinymemory_core::tree::tree`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::tree::tree::…` path resolving.

// Engine-owned persistence types (`Tree`, `SummaryNode`, `Buffer`, `TreeKind`,
// the seal/flush/registry mechanics). None of them is a contract item, and the
// `TreeStatus` re-exported here is the engine's `store::trees` one — a
// different type from the contract's `TreeStatus`, which the sibling
// `tree_runtime` module exports. Two same-named types one module apart, so
// check which one a call site means before moving it.
pub use tinymemory_core::tree::tree::*;

pub mod rpc;
