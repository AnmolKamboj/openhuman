//! Product `Config` adapter for the tinycortex GitHub repo reader.
//!
//! The reader itself — commit/issue/PR fetching over `gh`, `git`, and the
//! public REST API — lives in the engine. This module keeps the host-side
//! `SourceReader` shape (`&Config`, `Result<_, String>`) that the sources RPC
//! surface and the sync runner are written against, and re-exports the two
//! coordinate helpers `sources::sync` derives its scopes from.

use async_trait::async_trait;

use crate::openhuman::config::Config;
use crate::openhuman::memory::sources::readers::SourceReader;
use crate::openhuman::memory::sources::types::{
    MemorySourceEntry, SourceContent, SourceItem, SourceKind,
};

pub use tinycortex::memory::sources::readers::github::{repo_archive_source_id, repo_chunk_scope};

pub struct GithubReader;

#[async_trait]
impl SourceReader for GithubReader {
    fn kind(&self) -> SourceKind {
        SourceKind::GithubRepo
    }

    async fn list_items(
        &self,
        source: &MemorySourceEntry,
        config: &Config,
    ) -> Result<Vec<SourceItem>, String> {
        tinycortex::memory::sources::SourceReader::list_items(
            &tinycortex::memory::sources::readers::github::GithubReader,
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
            &tinycortex::memory::sources::readers::github::GithubReader,
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
