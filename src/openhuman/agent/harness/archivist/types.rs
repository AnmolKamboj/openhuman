//! `ArchivistHook` struct definition.

use super::boundary::BoundaryConfig;
use crate::openhuman::config::Config;
use crate::openhuman::memory::api::provider::MemoryProvider;
use crate::openhuman::memory::tree::score::embed::Embedder;
use std::sync::Arc;
use tinymemory_core::chat::ChatProvider;

/// Post-turn hook that indexes conversation turns and manages segments.
pub struct ArchivistHook {
    /// The bound memory driver this archivist writes through.
    ///
    /// This used to be the raw SQLite connection shared with `UnifiedMemory`,
    /// which is precisely the handle no remote or module driver can supply —
    /// the `:290` blocker the #5378 correction documented. Every episodic and
    /// profile write now goes through the provider's capability families, so
    /// the archivist works against whatever driver the workspace bound.
    pub(super) provider: Option<Arc<dyn MemoryProvider>>,
    /// Whether the archivist is enabled.
    pub(super) enabled: bool,
    /// Boundary detection configuration.
    pub(super) boundary_config: BoundaryConfig,
    /// Optional runtime config — used to gate the tree-ingest path and to
    /// build the LLM chat provider + embedder.
    ///
    /// When `None`, the tree-ingest path is skipped. Set via
    /// [`ArchivistHook::with_config`] on the production path.
    pub(super) config: Option<Config>,
    /// Optional LLM provider for segment recap. When `None`, the
    /// fallback heuristic summary is used instead.
    pub(super) chat_provider: Option<Arc<dyn ChatProvider>>,
    /// Optional embedder for segment recap vectors. When `None`, embedding
    /// is skipped (segment is still summarised).
    pub(super) embedder: Option<Arc<dyn Embedder>>,
}
