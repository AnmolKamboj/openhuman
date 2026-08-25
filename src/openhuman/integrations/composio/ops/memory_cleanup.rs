//! Memory cleanup helpers used when deleting a Composio connection.

use std::sync::Arc;

use crate::openhuman::config::Config;
use tinymemory_api::chunks::SourceKind;
use tinymemory_api::provider::ForgetSelector;
use tinymemory_core::store::MemoryClientRef;

/// One thing a connection delete has to remove from memory.
///
/// The three variants are the three [`ForgetSelector`] arms this domain needs,
/// named in this domain's own vocabulary: a Composio connection files content
/// under an exact source id, under a family of derived ids sharing a prefix,
/// and — for a mailbox shared with other connections — under an owner. They
/// stay a local enum rather than becoming `ForgetSelector` directly because
/// `label` is what a partial-failure message shows the user, and that wording
/// is this host's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryCleanupTarget {
    Exact(SourceKind, String),
    Prefix(SourceKind, String),
    Owner(SourceKind, String),
}

impl MemoryCleanupTarget {
    /// The contract selector this target names.
    ///
    /// `source_kind` crosses as a wire string because the set of source kinds
    /// belongs to the host's sync machinery and grows without a contract
    /// change; `SourceKind::as_str` is the same spelling `label` already
    /// shows.
    fn selector(&self) -> ForgetSelector {
        match self {
            Self::Exact(source_kind, source_id) => ForgetSelector::Source {
                source_kind: source_kind.as_str().to_string(),
                source_id: source_id.clone(),
            },
            Self::Prefix(source_kind, source_id_prefix) => ForgetSelector::SourcePrefix {
                source_kind: source_kind.as_str().to_string(),
                source_id_prefix: source_id_prefix.clone(),
            },
            Self::Owner(source_kind, owner) => ForgetSelector::Owner {
                source_kind: source_kind.as_str().to_string(),
                owner: owner.clone(),
            },
        }
    }

    /// Remove this target through the bound memory driver, returning the chunk
    /// count it took with it.
    ///
    /// A driver without the `Sources` family is **refused**, not degraded to
    /// zero: this is a delete, and its only empty answer — zero chunks
    /// removed — is byte-identical to a successful delete of nothing. The
    /// caller sums these into `memory_chunks_deleted` on the delete-connection
    /// reply, so a silent zero would tell the user a disconnected account's
    /// mail had left memory while it is still on disk. The caller already
    /// collects per-target failures and reports them beside the count, so a
    /// refusal here is surfaced rather than fatal.
    ///
    /// `ForgetOutcome::trees_cleaned` is dropped on purpose —
    /// `memory_chunks_deleted` has always been a chunk count, and this does
    /// not change the reply's shape.
    ///
    /// No `spawn_blocking`: the driver owns whether its own writes block, and
    /// the module's do not run on this thread at all.
    pub(super) async fn delete(&self, config: &Config) -> anyhow::Result<usize> {
        let binding = crate::openhuman::memory::binding::for_config(config)
            .map_err(|e| anyhow::anyhow!("forget_matching: {e}"))?;
        let Some(sources) = binding.provider().as_sources() else {
            return Err(anyhow::anyhow!(
                "forget_matching: driver '{}' does not serve Sources",
                binding.driver_id()
            ));
        };
        let selector = self.selector();
        let outcome = sources
            .forget_matching(&selector)
            .await
            .map_err(|e| anyhow::anyhow!("forget_matching: {e}"))?;
        log::debug!(
            "[composio][memory] forget_matching target={} removed chunks={} trees={} (driver='{}')",
            self.label(),
            outcome.chunks_removed,
            outcome.trees_cleaned,
            binding.driver_id()
        );
        Ok(usize::try_from(outcome.chunks_removed).unwrap_or(usize::MAX))
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
