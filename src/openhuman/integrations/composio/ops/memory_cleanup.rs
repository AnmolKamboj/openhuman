//! Memory cleanup helpers used when deleting a Composio connection.

use std::sync::Arc;

use crate::openhuman::config::Config;
use tinymemory_api::chunks::SourceKind;
use tinymemory_core::store::chunks::store as memory_tree_store;
use tinymemory_core::store::MemoryClientRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCleanupTarget {
    Exact(SourceKind, String),
    Prefix(SourceKind, String),
    Owner(SourceKind, String),
}

impl MemoryCleanupTarget {
    pub(super) fn delete(&self, config: &Config) -> anyhow::Result<usize> {
        match self {
            Self::Exact(source_kind, source_id) => {
                memory_tree_store::delete_chunks_by_source(config, *source_kind, source_id)
            }
            Self::Prefix(source_kind, source_id_prefix) => {
                memory_tree_store::delete_chunks_by_source_prefix(
                    config,
                    *source_kind,
                    source_id_prefix,
                )
            }
            Self::Owner(source_kind, owner) => {
                memory_tree_store::delete_chunks_by_owner(config, *source_kind, owner)
            }
        }
    }

    pub(super) fn label(&self) -> String {
        match self {
            Self::Exact(source_kind, source_id) => {
                format!("{}:{source_id}", source_kind.as_str())
            }
            Self::Prefix(source_kind, source_id_prefix) => {
                format!("{}:{source_id_prefix}*", source_kind.as_str())
            }
            Self::Owner(source_kind, owner) => {
                format!("{}:owner:{owner}", source_kind.as_str())
            }
        }
    }
}

/// `memory` is the caller's handle on the live store — the RPC path resolves
/// the process client, tests pass one bound to their own workspace. Taken as a
/// parameter rather than constructed here: `MemoryClient::from_workspace_dir`
/// starts an ingestion worker at construction, so building one per
/// connection-delete put a second worker on the live store every time — the
/// exact hazard `memory::bypass_allowlist_tests` names for that constructor.
pub(crate) async fn composio_memory_targets_for_connection(
    memory: &MemoryClientRef,
    toolkit: Option<&str>,
    connection_id: &str,
) -> anyhow::Result<Vec<MemoryCleanupTarget>> {
    let Some(toolkit) = toolkit.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };

    let targets = match toolkit.to_ascii_lowercase().as_str() {
        "slack" => vec![MemoryCleanupTarget::Exact(
            SourceKind::Chat,
            format!("slack:{connection_id}"),
        )],
        "gmail" => gmail_memory_sources_for_connection(connection_id),
        "notion" => notion_memory_targets_for_connection(memory, connection_id).await?,
        "drive" | "googledrive" | "google_drive" => {
            drive_memory_targets_for_connection(connection_id)
        }
        _ => Vec::new(),
    };
    Ok(targets)
}

fn gmail_memory_sources_for_connection(connection_id: &str) -> Vec<MemoryCleanupTarget> {
    vec![
        MemoryCleanupTarget::Owner(SourceKind::Email, format!("gmail-sync:{connection_id}")),
        MemoryCleanupTarget::Exact(SourceKind::Email, format!("gmail:{connection_id}")),
        MemoryCleanupTarget::Prefix(SourceKind::Email, format!("gmail:{connection_id}:")),
        MemoryCleanupTarget::Prefix(SourceKind::Email, format!("gmail:{connection_id}/")),
    ]
}

async fn notion_memory_targets_for_connection(
    memory: &MemoryClientRef,
    connection_id: &str,
) -> anyhow::Result<Vec<MemoryCleanupTarget>> {
    let mut targets = connection_scoped_document_targets("notion", connection_id);

    let adapter = tinymemory_core::tinycortex::HostSyncAdapter::new(Arc::clone(memory));
    let state = tinycortex::memory::sync::SyncState::load(&adapter, "notion", connection_id)
        .await
        .map_err(|error| {
            anyhow::anyhow!("failed to load notion sync state for memory cleanup: {error}")
        })?;
    for raw_id in state.synced_ids {
        let Some(page_id) = notion_synced_page_id(&raw_id) else {
            continue;
        };
        targets.push(MemoryCleanupTarget::Exact(
            SourceKind::Document,
            format!("notion:{page_id}"),
        ));
        targets.push(MemoryCleanupTarget::Exact(
            SourceKind::Document,
            format!("composio-notion-page-{page_id}"),
        ));
    }

    Ok(dedupe_memory_targets(targets))
}

fn drive_memory_targets_for_connection(connection_id: &str) -> Vec<MemoryCleanupTarget> {
    ["drive", "googledrive", "google_drive"]
        .into_iter()
        .flat_map(|prefix| connection_scoped_document_targets(prefix, connection_id))
        .collect()
}

fn connection_scoped_document_targets(
    prefix: &str,
    connection_id: &str,
) -> Vec<MemoryCleanupTarget> {
    vec![
        MemoryCleanupTarget::Exact(SourceKind::Document, format!("{prefix}:{connection_id}")),
        MemoryCleanupTarget::Prefix(SourceKind::Document, format!("{prefix}:{connection_id}:")),
        MemoryCleanupTarget::Prefix(SourceKind::Document, format!("{prefix}:{connection_id}/")),
    ]
}

fn notion_synced_page_id(raw_id: &str) -> Option<String> {
    let page_id = raw_id.split_once('@').map_or(raw_id, |(id, _)| id).trim();
    (!page_id.is_empty()).then(|| page_id.to_string())
}

fn dedupe_memory_targets(targets: Vec<MemoryCleanupTarget>) -> Vec<MemoryCleanupTarget> {
    let mut unique = Vec::new();
    for target in targets {
        if !unique.contains(&target) {
            unique.push(target);
        }
    }
    unique
}
