//! Wall-clock scheduler that periodically enqueues a [`JobKind::FlushStale`]
//! so low-volume source-tree L0 buffers seal promptly.
//!
//! The daily global-digest loop was removed along with the global tree —
//! source trees plus the entity index are the substrate, so there is no
//! cross-source digest to enqueue. Only the stale-buffer flush remains.

use std::time::Duration;

use crate::openhuman::config::Config;

static STARTED: std::sync::Once = std::sync::Once::new();

/// Start the periodic flush_stale scheduler. Takes the full `Config` so the
/// enqueues match the same workspace + LLM settings the workers see — not
/// `Config::default()`.
pub fn start(config: Config) {
    STARTED.call_once(|| {
        // Periodic flush_stale loop (every 3 h) so L0 buffers seal
        // promptly even for low-volume sources.
        let cfg = config.clone();
        tokio::spawn(async move {
            // Fire once on startup so new installs & restarts don't wait
            // up to 3 h for the first seal window.
            retry_transient_failures(&cfg);
            enqueue_flush_stale(&cfg);
            loop {
                tokio::time::sleep(Duration::from_secs(3 * 60 * 60)).await;
                retry_transient_failures(&cfg);
                enqueue_flush_stale(&cfg);
            }
        });
    });
}

/// Self-heal the pipeline before each flush window: requeue jobs that
/// failed for transient reasons (network blips, timeouts, SQLITE_BUSY)
/// so chunks never sit unprocessed until the next manual sync.
/// Unrecoverable failures stay parked — see
/// [`store::requeue_transient_failed`].
fn retry_transient_failures(config: &Config) {
    let memory = crate::openhuman::memory::tinycortex::memory_config_from(
        config,
        config.workspace_dir.clone(),
    );
    match tinycortex::memory::queue::scheduler::self_heal(&memory) {
        Ok(0) => {}
        Ok(n) => {
            log::info!("[memory::jobs] periodic retry requeued {n} transient-failed job(s)");
            super::worker::wake_workers();
        }
        Err(err) => {
            log::warn!("[memory::jobs] periodic transient-failure retry failed: {err:#}");
        }
    }
}

/// Enqueue one stale-buffer flush job for the current 3-hour UTC window and
/// wake the workers, reporting whether a job was actually created.
///
/// `Ok(false)` means the window already had one — the enqueue is deduped on
/// `(date, hour_block)`, so this is the normal repeat-call outcome and not a
/// failure.
///
/// `pub(crate)` (it was private) so the embedded memory driver's
/// `MemoryMaintenance::consolidate` can drive the **same** path the periodic
/// scheduler drives. That matters: the flush job fans out into per-tree `Seal`
/// jobs whose label strategy is derived per tree by the queue worker
/// (`TreeFactory::from_tree(&tree).label_strategy(...)`). The alternative entry
/// point, `tree::tree::flush::flush_stale_buffers_default`, takes **one**
/// `LabelStrategy` for every tree, which no production caller uses and which
/// would apply one tree kind's labelling to all of them.
pub(crate) fn enqueue_flush_stale_job(config: &Config) -> Result<bool, String> {
    let memory = crate::openhuman::memory::tinycortex::memory_config_from(
        config,
        config.workspace_dir.clone(),
    );
    match tinycortex::memory::queue::scheduler::enqueue_flush_stale(&memory) {
        Ok(Some(_)) => {
            super::worker::wake_workers();
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(err) => Err(format!("{err:#}")),
    }
}

fn enqueue_flush_stale(config: &Config) {
    if let Err(err) = enqueue_flush_stale_job(config) {
        log::warn!("[memory::jobs] periodic flush_stale enqueue failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::memory::queue::store::{
        claim_next, count_by_status, DEFAULT_LOCK_DURATION_MS,
    };
    use crate::openhuman::memory::queue::types::{FlushStalePayload, JobKind, JobStatus};
    use tempfile::TempDir;

    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().unwrap();
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().to_path_buf();
        cfg.memory_tree.embedding_endpoint = None;
        cfg.memory_tree.embedding_model = None;
        cfg.memory_tree.embedding_strict = false;
        (tmp, cfg)
    }

    #[test]
    fn enqueue_flush_stale_enqueues_at_most_one_job_per_current_block() {
        let (_tmp, cfg) = test_config();
        enqueue_flush_stale(&cfg);
        enqueue_flush_stale(&cfg);

        assert_eq!(
            count_by_status(&cfg, JobStatus::Ready).unwrap(),
            1,
            "second enqueue in same 3h block should be dedupe-suppressed"
        );

        let claimed = claim_next(&cfg, DEFAULT_LOCK_DURATION_MS).unwrap().unwrap();
        assert_eq!(claimed.kind, JobKind::FlushStale);
        let payload: FlushStalePayload = serde_json::from_str(&claimed.payload_json).unwrap();
        assert_eq!(payload.max_age_secs, None);
    }
}
