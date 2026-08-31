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
//! rather than by grepping for the paths, the production surface is now two
//! names in two files:
//!
//! - `score` — `read_rpc::entities` reaches `score::store` and
//!   `score::DEFAULT_DROP_THRESHOLD` for the chunk-score RPC.
//! - `summarise` — `agent::harness::archivist::recap` reaches
//!   `summarise::{summarise, SummaryContext, SummaryInput}`.
//!
//! Everything else the glob carries is now test-only or unreferenced.
//! `ingest` (`ingest_summary` / `SummaryIngestInput`) survives for
//! `tests/memory_sync_pipeline_e2e.rs`, `score::{embed, extract, resolver,
//! signals}` and `summarise::fallback_summary` for the raw-coverage suites,
//! and `nlp` and `graph` have no caller under this path at all — `nlp`
//! **used** to be reached from `retrieval/rpc.rs` and no longer is. A glob is
//! all-or-nothing, so the unused names ride along with the two that are left.
//!
//! Neither survivor has a contract equivalent: `tinymemory_api::tree` is the
//! summary-node vocabulary, not an embedder and not an entity extractor. So
//! this shim is pinned by the engine's *scoring and summarisation* internals
//! reached from two files outside this directory, and it goes when those two
//! move — not before.
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
// at. Two production files pin it; see the module docs above for which.
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
