//! [`MemoryMaintenance`] for the embedded driver — the four upkeep operations
//! the host's scheduler drives.
//!
//! No operation here installs a background task, and none of them is allowed to
//! return a success-shaped empty report for work that did not happen. Where the
//! embedded engine has no mechanism behind a contract operation, the report says
//! so in [`MaintenanceReport::findings`] rather than reading as "ran, nothing to
//! do".
//!
//! ## `reembed` and `consolidate` enqueue; they do not run
//!
//! Both go through the job queue, which is how the host itself drives them. The
//! reported `changed` is therefore *jobs enqueued*, and each report says
//! explicitly that the work runs asynchronously in the queue worker. That is
//! also why [`MemoryError::BudgetExceeded`] never appears here despite the
//! contract naming it for `reembed`: the embedding budget is exhausted inside
//! the worker, long after this call has returned.
//!
//! `consolidate` in particular must go through the queue rather than through
//! `tree::tree::flush::flush_stale_buffers_default`. The queue path fans out
//! into per-tree `Seal` jobs whose label strategy the worker derives per tree
//! (`TreeFactory::from_tree(&tree).label_strategy(...)`); the direct call takes
//! a single `LabelStrategy` for every tree, has no production caller, and would
//! apply one tree kind's labelling to all of them.
//!
//! ## `compact` under-delivers against the contract, and says so
//!
//! The contract asks for "vacuum indexes, drop tombstones, prune dead
//! references". **There is no `VACUUM` anywhere in this tree** — not in
//! `src/openhuman/memory/`, not in the vendored engine. The only thing the
//! embedded engine has that is genuinely in this family is
//! `queue::store::recover_stale_locks`, which drops lock rows referencing
//! workers that are gone: dead references, prunable. So that is what runs, and
//! the findings state plainly that no index vacuum exists.
//!
//! Two things deliberately do **not** run under `compact`:
//! `requeue_failed` / `requeue_transient_failed` are liveness self-heal, not
//! space reclamation, and the periodic scheduler already drives the transient
//! one every three hours — doing it again under a name that means something
//! else would make the operation lie in a second way. `diff::ops::cleanup` does
//! reclaim (checkpoint tags), but needs a retention window the contract does not
//! supply, and inventing one here would be policy.
//!
//! ## `doctor` is read-only and reports `note`, not `failure`
//!
//! `changed` is hard-coded `0`; the contract requires it. Findings come from
//! [`StageHealth::note`], documented as "short non-localized human note for logs
//! / CLI (never a secret)" — exactly the constraint `findings` imposes.
//! `PipelineFailure` carries an i18n remediation key meant for the UI, and
//! `DegradedState::cause` has no such never-a-secret guarantee, so neither is
//! used.

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::MaintenanceReport;
use tinycortex_api::provider::MemoryMaintenance;

use crate::openhuman::config::Config;
use crate::openhuman::memory::queue::store as queue_store;
use crate::openhuman::memory::queue::types::JobStatus;

use super::{host_error, EmbeddedMemoryProvider};

/// Total jobs plus ready jobs, both best-effort.
///
/// A counter read that errors degrades to 0 rather than failing the whole
/// operation, matching `tree::health::doctor`'s own rule for the same counters —
/// maintenance reporting is a convenience, not an audit.
fn queue_counts(config: &Config) -> (u64, u64) {
    let total = queue_store::count_total(config).unwrap_or(0);
    let ready = queue_store::count_by_status(config, JobStatus::Ready).unwrap_or(0);
    (total, ready)
}

#[async_trait]
impl MemoryMaintenance for EmbeddedMemoryProvider {
    async fn reembed(&self) -> Result<MaintenanceReport, MemoryError> {
        log::debug!("[memory:driver:embedded] reembed");
        let config = self.config().await?.clone();

        // `ensure_reembed_backfill` opens SQLite and is synchronous; it also
        // swallows its own errors by design (it must never fail a settings
        // save), so the observable effect is the queue depth delta below.
        let (examined, enqueued) = tokio::task::spawn_blocking(move || {
            let (total, ready_before) = queue_counts(&config);
            crate::openhuman::memory::queue::ensure_reembed_backfill(&config);
            let (_, ready_after) = queue_counts(&config);
            (total, ready_after.saturating_sub(ready_before))
        })
        .await
        .map_err(|error| host_error("reembed", format!("join: {error}")))?;

        Ok(MaintenanceReport {
            operation: "reembed".to_string(),
            examined,
            changed: enqueued,
            findings: vec![format!(
                "enqueued {enqueued} re-embed backfill job(s); the work itself runs \
                 asynchronously in the memory queue worker"
            )],
        })
    }

    async fn compact(&self) -> Result<MaintenanceReport, MemoryError> {
        log::debug!("[memory:driver:embedded] compact");
        let config = self.config().await?.clone();

        let (examined, recovered) = tokio::task::spawn_blocking(move || {
            let (total, _ready) = queue_counts(&config);
            let recovered = queue_store::recover_stale_locks(&config).unwrap_or(0);
            (total, recovered as u64)
        })
        .await
        .map_err(|error| host_error("compact", format!("join: {error}")))?;

        Ok(MaintenanceReport {
            operation: "compact".to_string(),
            examined,
            changed: recovered,
            findings: vec![
                format!("released {recovered} stale queue lock(s)"),
                "no index vacuum or tombstone pruning is implemented by the embedded engine; \
                 compact reclaims dead queue locks only"
                    .to_string(),
            ],
        })
    }

    async fn consolidate(&self) -> Result<MaintenanceReport, MemoryError> {
        log::debug!("[memory:driver:embedded] consolidate");
        let config = self.config().await?.clone();

        let (examined, enqueued) =
            tokio::task::spawn_blocking(move || -> Result<(u64, bool), String> {
                let (total, _ready) = queue_counts(&config);
                let enqueued =
                    crate::openhuman::memory::queue::scheduler::enqueue_flush_stale_job(&config)?;
                Ok((total, enqueued))
            })
            .await
            .map_err(|error| host_error("consolidate", format!("join: {error}")))?
            .map_err(|error| host_error("consolidate", error))?;

        let finding = if enqueued {
            "enqueued a stale-buffer flush; it fans out into per-tree seal jobs in the memory \
             queue worker"
        } else {
            // Not a failure: the enqueue is deduped on (date, 3-hour block), so
            // a second call inside the same window is a genuine no-op and must
            // not report work it did not do.
            "a stale-buffer flush is already queued for this window; nothing was enqueued"
        };
        Ok(MaintenanceReport {
            operation: "consolidate".to_string(),
            examined,
            changed: u64::from(enqueued),
            findings: vec![finding.to_string()],
        })
    }

    async fn doctor(&self) -> Result<MaintenanceReport, MemoryError> {
        log::debug!("[memory:driver:embedded] doctor");
        let config = self.config().await?;

        // Infallible and already `spawn_blocking`-wrapped host-side.
        let report = crate::openhuman::memory::tree::health::doctor::async_run_doctor(config).await;

        let findings = report
            .stages
            .iter()
            .filter(|stage| !stage.ok)
            .map(|stage| format!("{}: {}", stage.stage, stage.note))
            .collect::<Vec<_>>();

        Ok(MaintenanceReport {
            operation: "doctor".to_string(),
            examined: report.counters.total_chunks,
            // Read-only by contract. Not derived from anything — pinned.
            changed: 0,
            findings,
        })
    }
}

#[cfg(test)]
#[path = "maintenance_tests.rs"]
mod tests;
