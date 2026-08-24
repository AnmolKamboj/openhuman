//! Host layer over [`tinymemory_core::tree::tree_runtime`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::tree::tree_runtime::…` path resolving.

pub use tinymemory_core::tree::tree_runtime::*;

// The summary-tree node model is **contract** vocabulary, not engine
// vocabulary: it is defined in `tinymemory-bus` and the engine crate re-exports
// the same items, so these names and the ones the glob above would supply are
// one set of types, not two. Naming the contract explicitly (an explicit `use`
// shadows a glob, and here it shadows it with itself) records which half of
// this shim survives the engine leaving the build — and means the call sites on
// `memory::tree::tree_runtime::estimate_tokens` and friends need no edit when
// it does.
pub use crate::openhuman::memory::api::tree as types;
pub use crate::openhuman::memory::api::tree::{
    derive_node_ids, derive_parent_id, estimate_tokens, level_from_node_id, node_id_to_path,
    IngestRequest, NodeLevel, QueryResult, TreeNode, TreeStatus,
};

pub mod ops;
pub mod schemas;

pub use ops as rpc;

pub mod bus;

/// The `openhuman memory tree` CLI subcommands, which drive the RPC handlers.
pub mod cli;

// The controller aggregators this domain's RPC surface defines. Aliased
// exactly as the pre-extraction module exported them.
pub use schemas::{
    all_controller_schemas as all_tree_summarizer_controller_schemas,
    all_registered_controllers as all_tree_summarizer_registered_controllers,
};
