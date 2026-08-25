//! Host layer over [`tinymemory_core::tree`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::tree::…` path resolving.
//!
//! # What the glob still carries, checked rather than assumed (#5560)
//!
//! Four of the nine module names it supplies (`health`, `retrieval`, `tree`,
//! `tree_runtime`) are shadowed by the `pub mod` declarations below — an
//! explicit item beats a glob import — so its live contribution is narrower
//! than it looks, and all of it is engine-owned:
//!
//! - `score`, `summarise`, `nlp`, `ingest` — real code in `tinymemory-core`,
//!   not thin aliases: `score::embed::{Embedder, build_embedder_from_config,
//!   effective_embedder_slug, EMBEDDING_DIM}`, `score::extract::EntityKind`,
//!   `score::store`, `score::resolver`, `summarise::{summarise,
//!   fallback_summary, SummaryContext, SummaryInput}` and
//!   `nlp::extract_query_entities` are reached from the archivist, the
//!   sub-agent runner, `read_rpc::entities`, `inference::embeddings::rpc` and
//!   `retrieval/rpc.rs` in this directory.
//! - the seven `Tree{LabelStrategy, LeafPayload, ReadHit, ReadRequest,
//!   ReadResult, WriteOutcome, WriteRequest}` I/O types, which the crate
//!   re-exports out of `engine::backend::tree`. Nothing in `src/` names them —
//!   `tests/raw_coverage/memory_threads_raw_coverage_e2e.rs` names all seven,
//!   which is why narrowing this glob to what `src/` uses would compile here
//!   and break the integration lane.
//!
//! (`graph` is the fifth unshadowed name and has no caller under this path at
//! all — it is carried only because the glob is all-or-nothing.)
//!
//! None of it has a contract equivalent: `tinymemory_api::tree` is the summary
//! node vocabulary, not an embedder or an entity extractor. So this shim is
//! pinned by the engine's *scoring and summarisation* internals, and it goes
//! when those move behind the bus — not before.

pub use tinymemory_core::tree::*;

pub mod health;
pub mod retrieval;
// `tree::tree` mirrors `tinymemory_core::tree::tree` — the wrapper has to keep
// the extracted crate's path shape so every historical `memory::tree::tree::…`
// reference still resolves. Renaming it here would break that for a lint.
#[allow(clippy::module_inception)]
pub mod tree;
pub mod tree_runtime;

// Controller registries. These aggregate the RPC surface that stayed here, so
// they cannot live in the extracted crate alongside the rest of `tree`.
pub use crate::openhuman::memory::schema::{
    all_controller_schemas as all_memory_tree_controller_schemas,
    all_registered_controllers as all_memory_tree_registered_controllers,
};
pub use retrieval::{all_retrieval_controller_schemas, all_retrieval_registered_controllers};
pub use tree_runtime::{
    all_tree_summarizer_controller_schemas, all_tree_summarizer_registered_controllers,
};
