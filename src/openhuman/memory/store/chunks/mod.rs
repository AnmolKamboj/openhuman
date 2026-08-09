//! Chunks — the unit of memory_store persistence.
//!
//! One module for the full chunk lifecycle:
//!
//! - [`types`]    — `Chunk`, `Metadata`, `SourceKind`, `RawRef`,
//!                  `ListChunksQuery`. The persisted shape.
//! - [`store`]    — SQLite persistence (`chunks` table + connection cache).
//! - [`semantic`] — heading- and paragraph-aware chunker used by the
//!                  unified memory writer to split large documents into
//!                  LLM-context-sized pieces while preserving heading
//!                  context.
//!
//! The source-kind-dispatch chunker ([`chunk_markdown`], the default — chat /
//! email / document, with stable per-source sequence numbers and bounded
//! segments) is engine-owned and re-exported straight from `tinycortex`.
//! [`chunk_markdown`] and `semantic::chunk_markdown` both yield string-shaped
//! chunks; the store side decides what to do with them.

pub mod semantic;
pub mod store;
pub mod types;

pub use semantic::chunk_markdown as chunk_semantic;
pub use store::*;
pub use tinycortex::memory::chunks::{chunk_markdown, ChunkerInput, ChunkerOptions};
pub use types::*;
