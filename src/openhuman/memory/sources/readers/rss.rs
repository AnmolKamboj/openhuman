//! Product `Config` adapter for the tinycortex RSS/Atom feed reader.

use async_trait::async_trait;

use crate::openhuman::config::Config;
use crate::openhuman::memory::sources::readers::SourceReader;
use crate::openhuman::memory::sources::types::{
    MemorySourceEntry, SourceContent, SourceItem, SourceKind,
};

/// Product adapter retaining the engine reader for a complete sync pass.
///
/// The engine reader caches a freshly fetched feed between `list_items` and
/// `read_item`, so constructing it per trait call would turn one sync into
/// N+1 downloads.
pub struct RssReader {
    inner: tinycortex::memory::sources::readers::rss::RssReader,
}

impl RssReader {
    pub fn new() -> Self {
        Self {
            inner: tinycortex::memory::sources::readers::rss::RssReader::new(),
        }
    }
}

impl Default for RssReader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceReader for RssReader {
    fn kind(&self) -> SourceKind {
        SourceKind::RssFeed
    }

    async fn list_items(
        &self,
        source: &MemorySourceEntry,
        config: &Config,
    ) -> Result<Vec<SourceItem>, String> {
        tinycortex::memory::sources::SourceReader::list_items(
            &self.inner,
            source,
            &crate::openhuman::memory::tinycortex::memory_config_from(
                config,
                config.workspace_dir.clone(),
            ),
        )
        .await
        .map_err(|error| error.to_string())
    }

    async fn read_item(
        &self,
        source: &MemorySourceEntry,
        item_id: &str,
        config: &Config,
    ) -> Result<SourceContent, String> {
        tinycortex::memory::sources::SourceReader::read_item(
            &self.inner,
            source,
            item_id,
            &crate::openhuman::memory::tinycortex::memory_config_from(
                config,
                config.workspace_dir.clone(),
            ),
        )
        .await
        .map_err(|error| error.to_string())
    }
}
