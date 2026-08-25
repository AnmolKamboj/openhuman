//! Host layer over the engine's tool-memory domain.
//!
//! The rule store and its types are the engine's. What stays here is the pair
//! that plugs into the agent harness — the post-turn capture hook and the
//! system prompt section — both of which name harness traits (`PostTurnHook`,
//! `PromptContext`) that the engine crate cannot see.
//!
//! # Why this names `tinycortex` rather than `tinymemory_core` (#5560)
//!
//! This module used to be `pub use tinymemory_core::tool_memory::*;`, and that
//! glob resolved to three different places:
//!
//! | Name | Where it really lives |
//! | --- | --- |
//! | `ToolMemoryStore`, `TOOL_MEMORY_PROMPT_CAP` | `tinycortex::memory::tool_memory::store` |
//! | `tool_memory_namespace`, `ToolMemoryPriority`, `ToolMemoryRule`, `ToolMemorySource` | `tinycortex::memory::tool_memory::types` |
//! | `tool_memory_store` | a ten-line constructor in `tinymemory-core` |
//! | `test_helpers` | `tinymemory-core`, behind `cfg(any(test, feature = "test-support"))` |
//!
//! The first two rows are named at `tinycortex` directly now. They are the
//! **same items** either way — `tinymemory_core::tool_memory` re-exported them
//! out of `crate::engine::backend::tool_memory`, which is itself
//! `pub use tinycortex::memory::tool_memory` — so this changes no type, only
//! which crate alias holds it in the build. `tinycortex` stays a direct
//! dependency of this crate; `tinymemory-core` is the one #5560 removes.
//!
//! The third row came home: [`tool_memory_store`] below is the same one-line
//! `ToolMemoryStore::new` wrapper, and it was only in the engine crate because
//! that crate used to be this host's memory layer. Nothing in `tinymemory`
//! named it.
//!
//! The fourth row is **test-only** and stays on the engine crate deliberately:
//! `MockMemory` is a test fixture reached from three inline `#[cfg(test)]`
//! modules, and `cfg(test)` code links against the `tinymemory-core`
//! **dev-dependency** (declared with `features = ["test-support"]`), which
//! survives the shed. It is not a production reference and does not keep the
//! engine crate in the shipped binary.

use std::sync::Arc;

// The contract's storage trait, named at the contract rather than through
// `memory::Memory` — same item, one less alias to follow.
use tinymemory_api::traits::Memory;

pub use tinycortex::memory::tool_memory::{
    store::{ToolMemoryStore, TOOL_MEMORY_PROMPT_CAP},
    types::{tool_memory_namespace, ToolMemoryPriority, ToolMemoryRule, ToolMemorySource},
};

pub mod capture;
pub mod prompt;

/// Build the rule store over OpenHuman's shared memory object.
///
/// A named constructor rather than `ToolMemoryStore::new` at each call site:
/// every caller (`capture.rs`, the harness session builder, the raw-coverage
/// suite) passes the same `Arc<dyn Memory>` the host already holds, and the
/// indirection is what let the store's own construction move between crates
/// without touching them. Kept for exactly that reason.
pub fn tool_memory_store(memory: Arc<dyn Memory>) -> ToolMemoryStore {
    log::trace!("[memory::tool_memory] building ToolMemoryStore over the host memory object");
    ToolMemoryStore::new(memory)
}

/// The engine crate's `MockMemory` fixture, under its historical path.
///
/// Test-only in both directions: `tinymemory-core` compiles this module behind
/// `cfg(any(test, feature = "test-support"))`, and this re-export is
/// `#[cfg(test)]`, so it exists only in `cargo test --lib` builds where the
/// dev-dependency at `Cargo.toml`'s `[dev-dependencies]` supplies the crate.
/// Three inline `#[cfg(test)]` modules reach it —
/// `agent::experience::{capture, store}` and
/// `agent::tinyagents::host::experience_store`.
#[cfg(test)]
pub use tinymemory_core::tool_memory::test_helpers;
