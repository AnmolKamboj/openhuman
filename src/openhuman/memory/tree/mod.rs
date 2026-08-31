//! Host layer over [`tinymemory_core::tree`].
//!
//! The domain itself lives in the extracted crate; what stays here is its
//! JSON-RPC surface — handlers and controller schemas name OpenHuman's
//! `RpcOutcome` and `ControllerSchema`, which the engine crate cannot see.
//! The glob re-export keeps every historical `memory::tree::…` path resolving.
//!
//! # What the glob still carries, re-measured rather than assumed (#5560)
//!
//! Four of the nine module names it supplies (`health`, `retrieval`, `tree`,
//! `tree_runtime`) are shadowed by the `pub mod` declarations below — an
//! explicit item beats a glob import — so its live contribution is narrower
//! than it looks. **Measured by deleting the line and reading the compiler**,
//! rather than by grepping for the paths, the production surface is now
//! **empty**. The two names that pinned it are both gone:
//!
//! - `score` — `read_rpc::entities` reached `score::store` and
//!   `score::DEFAULT_DROP_THRESHOLD` for the chunk-score RPC. That is
//!   `MemoryChunks::chunk_score` and the contract's own
//!   `DEFAULT_DROP_THRESHOLD` since #5560.
//! - `summarise` — `agent::harness::archivist::recap` reached
//!   `summarise::{summarise, SummaryContext, SummaryInput}`. That is
//!   `MemoryTree::summarise` and the contract's owned DTOs now; only the
//!   recap's `#[cfg(test)]` arm still names the engine's, because the
//!   deterministic chat provider those tests install is a task-local *inside*
//!   the engine crate this binary links for tests.
//!
//! So everything the glob still carries is test-only. `ingest`
//! (`ingest_summary` / `SummaryIngestInput`) survives for
//! `tests/memory_sync_pipeline_e2e.rs`, `score::{embed, extract, resolver,
//! signals, store}` and `summarise::{summarise, fallback_summary}` for the
//! raw-coverage suites and this crate's own `*_tests.rs`, and `nlp` and
//! `graph` have no caller under this path at all — `nlp` **used** to be
//! reached from `retrieval/rpc.rs` and no longer is. A glob is all-or-nothing,
//! so the unused names ride along.
//!
//! Nothing left here has a contract equivalent, and none is wanted:
//! `tinymemory_api::tree` is the summary-node vocabulary, not an embedder and
//! not an entity extractor. This shim is now pinned purely by test targets, and
//! it goes when those stop asserting against an in-process engine — which is a
//! decision about how the raw-coverage suites are written, not a routing pass.
//!
//! The seven `Tree{LabelStrategy, LeafPayload, ReadHit, ReadRequest,
//! ReadResult, WriteOutcome, WriteRequest}` I/O types were re-exported here
//! explicitly until #5560's shed pass. **They are gone**: nothing in `src/`
//! named them, and the one consumer —
//! `tests/raw_coverage/memory_threads_raw_coverage_e2e.rs` — names
//! `tinycortex::memory::tree::…` directly now. A production `pub use` that
//! exists only to serve a test is exactly what keeps a crate in
//! `[dependencies]`, which is the thing this issue is removing.

// What is left of the glob is the scoring and summarisation half, and it is
// `tinymemory-core`'s own code rather than a re-export: `score` (embed /
// extract / store / resolver / signals), `summarise`, `nlp` and `ingest`, plus
// `graph`, which the glob carries because a glob is all-or-nothing.
// `tinymemory_api::tree` is the summary-node vocabulary — not an embedder and
// not an entity extractor — so there is nothing on the contract to route these
// at. **No production file pins it any more**; see the module docs above for
// what does.
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
