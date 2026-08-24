//! Test-only constructors for `ArchivistHook` that inject stub providers
//! directly, bypassing `with_config`'s provider-build logic.

use super::boundary::BoundaryConfig;
use super::types::ArchivistHook;
use crate::openhuman::config::Config;
use crate::openhuman::memory::api::provider::MemoryProvider;
use crate::openhuman::memory::tree::score::embed::Embedder;
use std::sync::Arc;
use tinymemory_core::chat::ChatProvider;

#[cfg(test)]
impl ArchivistHook {
    /// Test-only constructor that injects a stub `ChatProvider` and `Embedder`
    /// directly, bypassing `with_config`'s provider-build logic. Used by
    /// Phase 1 tests to verify LLM recap and embedding paths without hitting
    /// a real LLM or Ollama daemon. Exposed as `pub(crate)` so Phase 3
    /// STM recall integration tests can drive the full archivist path.
    pub(crate) fn new_with_stubs(
        provider: Arc<dyn MemoryProvider>,
        chat_provider: Arc<dyn ChatProvider>,
        embedder: Arc<dyn Embedder>,
    ) -> Self {
        Self {
            provider: Some(provider),
            enabled: true,
            boundary_config: BoundaryConfig::default(),
            config: Some(Config::default()),
            chat_provider: Some(chat_provider),
            embedder: Some(embedder),
        }
    }

    /// Test-only constructor that injects stub providers AND a `Config`, so the
    /// Phase 2 segment-tree ingest path (gated by
    /// `config.learning.chat_to_tree_enabled`) can be exercised hermetically.
    ///
    /// `config.learning.chat_to_tree_enabled` must be set to `true` by the caller
    /// for the tree ingest to fire; the hook does NOT force it on.
    pub(crate) fn new_with_stubs_and_config(
        provider: Arc<dyn MemoryProvider>,
        chat_provider: Arc<dyn ChatProvider>,
        embedder: Arc<dyn Embedder>,
        config: Config,
    ) -> Self {
        Self {
            provider: Some(provider),
            enabled: true,
            boundary_config: BoundaryConfig::default(),
            config: Some(config),
            chat_provider: Some(chat_provider),
            embedder: Some(embedder),
        }
    }
}
