//! Product artifact hooks around tinycortex-owned direct summary ingestion.

#[cfg(feature = "memory-git")]
use anyhow::Context;
use anyhow::Result;

use crate::openhuman::config::Config;
#[cfg(feature = "memory-git")]
use crate::openhuman::memory::store::content::wiki_git::{SummaryCommitBatch, SummaryCommitEntry};
use crate::openhuman::memory::store::trees::types::Tree;
use crate::openhuman::memory::tinycortex::{memory_config_from, HostSummariser};

pub use tinycortex::memory::tree::{SummaryIngestInput, SummaryIngestOutcome};

pub async fn ingest_summary(
    config: &Config,
    tree: &Tree,
    input: SummaryIngestInput,
) -> Result<SummaryIngestOutcome> {
    log::debug!(
        "[memory_tree::ingest] tinycortex enter tree_kind={} children={}",
        tree.kind.as_str(),
        input.child_labels.len()
    );
    let content_root = config.memory_tree_content_root();
    if let Err(error) =
        crate::openhuman::memory::store::content::obsidian::ensure_obsidian_defaults(&content_root)
    {
        log::warn!("[memory_tree::ingest] obsidian defaults failed: {error:#}");
    }

    let outcome = tinycortex::memory::tree::ingest_summary(
        &memory_config_from(config, config.workspace_dir.clone()),
        tree,
        input.clone(),
        &HostSummariser::new(config.clone()),
    )
    .await?;

    // The git wiki mirror is a DERIVED view: `ingest_summary` above has already
    // written the summary to disk, and this only records it in the git-backed
    // mirror. Skipping it when `memory-git` is off loses the mirror, not the
    // summary — so the call site is gated rather than stubbed.
    #[cfg(feature = "memory-git")]
    crate::openhuman::memory::store::content::wiki_git::commit_summaries(
        &content_root,
        &SummaryCommitBatch {
            reason: "summary_ingest".to_string(),
            tree_id: tree.id.clone(),
            tree_scope: tree.scope.clone(),
            entries: vec![SummaryCommitEntry {
                summary_id: outcome.summary_id.clone(),
                content_path: outcome.content_path.clone(),
                level: 1,
                child_count: input.child_labels.len(),
                token_count: input.token_count,
                time_range_start: input.time_range_start,
                time_range_end: input.time_range_end,
            }],
        },
    )
    .with_context(|| format!("commit ingested summary {}", outcome.summary_id))?;

    log::debug!(
        "[memory_tree::ingest] tinycortex complete sealed={}",
        outcome.sealed_ids.len()
    );
    Ok(outcome)
}
