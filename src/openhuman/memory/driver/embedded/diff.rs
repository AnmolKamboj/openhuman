//! [`MemoryDiff`] for the embedded driver — snapshot capture and change
//! computation over synced sources.
//!
//! Every method delegates to [`memory::diff::ops`](crate::openhuman::memory::diff::ops),
//! never to `tinycortex::memory::diff::DiffEngine` or `Ledger` directly. That
//! matters for more than tidiness: `ops::take_snapshot` publishes
//! [`DomainEvent::MemoryDiffSnapshotTaken`](crate::core::event_bus::DomainEvent),
//! and reaching past it would make a snapshot captured through the contract
//! invisible to every subscriber that watches for one.
//!
//! ## Three contract methods, ten host functions — the other seven stay host-side
//!
//! [`MemoryDiff`] is exactly `capture_snapshot` / `snapshots` / `diff`. The host
//! additionally has `diff_since_last`, `diff_since_read`, `mark_read`,
//! `create_checkpoint`, `diff_since_checkpoint`, `cleanup` and
//! `auto_snapshot_after_sync`. Those are **not omissions**: read markers and
//! named checkpoints are product surface with no contract representation, and
//! `auto_snapshot_after_sync` is a hook on the host's sync scheduling. They keep
//! their RPC/tool entry points and are not reachable through the provider.
//!
//! ## Asymmetric `NotFound`, on purpose
//!
//! `capture_snapshot` on an unknown source is [`MemoryError::NotFound`]: there
//! is nothing to snapshot, and the contract names that case. `snapshots` on an
//! unknown source is an **empty vector**, which the contract also names — the
//! git ledger has no source registry to consult, so "no snapshots" and "no such
//! source" are the same observation there. Do not "fix" the asymmetry by adding
//! a registry lookup to `snapshots`; it would change a documented outcome.
//!
//! ## The source registry is read through the driver's own config
//!
//! [`registry::get_source_in`] rather than `registry::get_source`: the latter
//! resolves the config path from the process environment, which for a driver
//! bound to workspace B would consult workspace A's source list.
//!
//! ## `diff` checks the source it was told about
//!
//! `ops::compute_diff` takes only snapshot ids — they are globally unique commit
//! SHAs, so it ignores `source_id` entirely and the engine's own cross-source
//! guard only catches `from`/`to` disagreeing with *each other*. A caller can
//! therefore diff source A's two snapshots while naming source B and get a
//! report that looks right. The returned `DiffResult` carries the real
//! `source_id`, so this file compares and rejects the mismatch.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::{ChangeKind, DiffReport, SnapshotRef, SourceChange};
use tinycortex_api::provider::MemoryDiff;

use crate::openhuman::memory::diff::ops;
use crate::openhuman::memory::sources::registry;
use tinycortex::memory::diff::types::{
    ChangeKind as EngineChangeKind, DiffResult, ItemChange, Snapshot, SnapshotTrigger,
};

use super::{host_error, EmbeddedMemoryProvider};

/// Engine snapshot → contract identity.
///
/// `source_kind` and `trigger` have no home in [`SnapshotRef`]; the contract
/// exposes identity and counts only.
fn to_snapshot_ref(snapshot: Snapshot) -> SnapshotRef {
    SnapshotRef {
        id: snapshot.id,
        source_id: snapshot.source_id,
        label: snapshot.label,
        item_count: snapshot.item_count,
        taken_at_ms: snapshot.taken_at_ms,
    }
}

/// Two enums, identical wire strings, no shared type — so this is a `match`,
/// not a serde round-trip. The contract's own doc records the equivalence.
fn to_change_kind(kind: EngineChangeKind) -> ChangeKind {
    match kind {
        EngineChangeKind::Added => ChangeKind::Added,
        EngineChangeKind::Removed => ChangeKind::Removed,
        EngineChangeKind::Modified => ChangeKind::Modified,
    }
}

/// Engine item change → contract change. `text_diff` is dropped because
/// [`SourceChange`] has no field for it — which is also why this family always
/// asks the engine for `include_text_diff: false` rather than computing a diff
/// nobody can read.
fn to_source_change(change: ItemChange) -> SourceChange {
    SourceChange {
        item_id: change.item_id,
        title: change.title,
        kind: to_change_kind(change.kind),
        old_content_hash: change.old_content_hash,
        new_content_hash: change.new_content_hash,
    }
}

/// Engine diff → contract report. The engine's nested `summary` flattens into
/// the report's four counters; `source_kind` / `source_label` are dropped.
fn to_diff_report(result: DiffResult) -> DiffReport {
    DiffReport {
        source_id: result.source_id,
        from_snapshot_id: result.from_snapshot_id,
        to_snapshot_id: result.to_snapshot_id,
        added: result.summary.added,
        removed: result.summary.removed,
        modified: result.summary.modified,
        unchanged: result.summary.unchanged,
        changes: result.changes.into_iter().map(to_source_change).collect(),
    }
}

#[async_trait]
impl MemoryDiff for EmbeddedMemoryProvider {
    async fn capture_snapshot(&self, source_id: &str) -> Result<SnapshotRef, MemoryError> {
        log::debug!("[memory:driver:embedded] capture_snapshot source_id={source_id}");
        let config = self.config().await?;

        let Some(source) = registry::get_source_in(config, source_id)
            .map_err(|error| host_error("capture_snapshot", error))?
        else {
            return Err(MemoryError::NotFound(source_id.to_string()));
        };

        // `Manual` and not `Auto`: `Auto` is the trigger the host stamps from
        // `auto_snapshot_after_sync`, and it is rendered in the ledger trailer
        // and the domain event. A snapshot asked for through the contract was
        // asked for explicitly.
        ops::take_snapshot(&source, config, SnapshotTrigger::Manual)
            .await
            .map(to_snapshot_ref)
            .map_err(|error| host_error("capture_snapshot", error))
    }

    async fn snapshots(
        &self,
        source_id: &str,
        limit: usize,
    ) -> Result<Vec<SnapshotRef>, MemoryError> {
        // The ledger's limit is a `u32`; saturate rather than wrap.
        let limit = u32::try_from(limit).unwrap_or(u32::MAX);
        log::debug!("[memory:driver:embedded] snapshots source_id={source_id} limit={limit}");

        let config = self.config().await?;
        ops::list_snapshots(config, Some(source_id), limit)
            .await
            .map(|snapshots| snapshots.into_iter().map(to_snapshot_ref).collect())
            .map_err(|error| host_error("snapshots", error))
    }

    async fn diff(
        &self,
        source_id: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<DiffReport, MemoryError> {
        log::debug!("[memory:driver:embedded] diff source_id={source_id} from={from:?} to={to}");
        let config = self.config().await?;

        // `include_text_diff: false` — see `to_source_change`.
        let result = ops::compute_diff(config, from, to, false)
            .await
            // The host flattens the engine's error to a `String`, so an unknown
            // snapshot id cannot be distinguished from a corrupt ledger here.
            // The contract asks for `NotFound` in the first case; getting there
            // needs `diff::ops` to stop flattening, which is a host change
            // beyond this step. Matching on the message text instead would
            // silently reclassify the moment libgit2's wording changes.
            .map_err(|error| host_error("diff", error))?;

        if result.source_id != source_id {
            return Err(MemoryError::Invalid(format!(
                "snapshot '{to}' belongs to source '{}', not '{source_id}'",
                result.source_id
            )));
        }

        Ok(to_diff_report(result))
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
