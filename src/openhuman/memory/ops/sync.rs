//! Memory-sync RPC handlers and ingestion-status reporting.
//!
//! Sync RPCs publish `DomainEvent::MemorySyncRequested` on the global event
//! bus — they are fire-and-forget hooks for future ingestion subscribers.
//!
//! # Both engine calls here are openhuman#5560 blockers, and both degrade
//!
//! Neither of the two `tinymemory_core::` calls below could be routed through
//! the contract, and both change behaviour the moment the host's in-process
//! engine is deleted. Written down here because both failures are quiet, and
//! the quiet one is the one that gets shipped.
//!
//! - **`spawn_manual_sync` → `tinycortex::run_composio_connection`.** The
//!   engine's own `run_composio_connection_with_caps` opens with
//!   `global::client_if_ready().ok_or(… "memory client is not ready")`, so with
//!   the in-process engine gone every target fails and this handler emits
//!   `MemorySyncStage::Failed` per connection. Loud, at least, but wrong. There
//!   is no contract member to move to: the whole pipeline is `tinycortex`-shaped
//!   (a `SyncPipeline` over provider-specific fetchers), and the loaded module
//!   does not run it either — `tinymemory` v1.5.0's module carries a section
//!   headed "The periodic sync loops are deliberately NOT started here" with
//!   three named reasons. Manual sync and the periodic loop share this call, so
//!   they move together or not at all.
//! - **[`memory_ingestion_status`] → `global::client_if_ready()`.** Reads the
//!   engine's live `IngestionState` — running flag, in-flight document, queue
//!   depth. The contract has no member for it (`memory::bypass_allowlist_tests`
//!   already records it as "queue telemetry, absent from the contract"), and
//!   the state now lives inside the module's own address space. **This is the
//!   quiet one**: the `None` arm already answers "idle, queue empty", which is
//!   indistinguishable from a healthy store with nothing to do — so once the
//!   second engine goes this RPC reports permanent idle rather than failing,
//!   and the Memory panel's ingestion indicator simply never lights up again.
//!
//!   The honest fix is not a new bus member. `tinymemory_api::host::MemoryEvent`
//!   already carries `DocumentIngestStarted`/`Completed` **with `queue_depth`**,
//!   and the host already serves the event sink over the bus
//!   (`modules/memory_host.rs`), so the module's ingestion events reach this
//!   process today. The status snapshot should be folded up from that stream
//!   host-side, which is a host change with no release coupling — but it needs
//!   a home next to the sink in `memory/host.rs`, not here.

use crate::openhuman::config::rpc as config_rpc;
use crate::openhuman::memory::sync::composio;
use crate::rpc::RpcOutcome;
use tinymemory_api::sync_events::{emit_sync_stage, MemorySyncStage, MemorySyncTrigger};

/// Parameters for `memory_sync_channel`.
#[derive(Debug, serde::Deserialize)]
pub struct SyncChannelParams {
    pub channel_id: String,
}

/// Result returned by `memory_sync_channel`.
#[derive(Debug, serde::Serialize)]
pub struct SyncChannelResult {
    pub requested: bool,
    pub channel_id: String,
}

/// Result returned by `memory_sync_all`.
#[derive(Debug, serde::Serialize)]
pub struct SyncAllResult {
    pub requested: bool,
}

/// Result returned by `memory_ingestion_status`. Mirrors
/// [`crate::openhuman::memory::IngestionStatusSnapshot`] but is the public RPC
/// shape — the indirection keeps internal renames from breaking the wire
/// contract.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct IngestionStatusResult {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_namespace: Option<String>,
    pub queue_depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_completed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success: Option<bool>,
}

/// Request a memory sync for a specific channel.
///
/// Ingestion in OpenHuman is listener/webhook-driven — there is no per-provider
/// pull mechanism yet. This RPC publishes `DomainEvent::MemorySyncRequested` so
/// that future ingestion subscribers can react to an explicit pull request.
/// The event is fire-and-forget; the caller receives confirmation that the
/// request was published, not that ingestion ran.
pub async fn memory_sync_channel(
    params: SyncChannelParams,
) -> Result<RpcOutcome<SyncChannelResult>, String> {
    // `channel_id` is a user/context identifier — keep it out of normal logs.
    tracing::info!("[memory.sync] memory_sync_channel: entry");
    crate::core::bus::BUS.publish(crate::core::events::DomainEvent::MemorySyncRequested {
        channel_id: Some(params.channel_id.clone()),
    });
    emit_sync_stage(
        MemorySyncTrigger::Manual,
        MemorySyncStage::Requested,
        None,
        Some(&params.channel_id),
        Some("channel-targeted sync requested".to_string()),
        None, // channel-level sync — not a memory-source row
    );
    let channel_id_for_spawn = params.channel_id.clone();
    tokio::spawn(async move {
        if let Err(e) = spawn_manual_sync(Some(channel_id_for_spawn)).await {
            tracing::warn!(error = %e, "[memory.sync] background channel sync failed");
        }
    });
    tracing::debug!("[memory.sync] memory_sync_channel: MemorySyncRequested published");
    Ok(RpcOutcome::new(
        SyncChannelResult {
            requested: true,
            channel_id: params.channel_id,
        },
        vec![],
    ))
}

/// Request a memory sync for all channels.
///
/// Publishes `DomainEvent::MemorySyncRequested { channel_id: None }` on the
/// global event bus. No consumers exist yet — this is a hook for future
/// ingestion subscribers.
pub async fn memory_sync_all() -> Result<RpcOutcome<SyncAllResult>, String> {
    tracing::info!("[memory.sync] memory_sync_all: entry");
    crate::core::bus::BUS
        .publish(crate::core::events::DomainEvent::MemorySyncRequested { channel_id: None });
    emit_sync_stage(
        MemorySyncTrigger::Manual,
        MemorySyncStage::Requested,
        None,
        None,
        Some("global sync requested".to_string()),
        None, // global sync — not a memory-source row
    );
    tokio::spawn(async move {
        if let Err(e) = spawn_manual_sync(None).await {
            tracing::warn!(error = %e, "[memory.sync] background global sync failed");
        }
    });
    tracing::debug!("[memory.sync] memory_sync_all: MemorySyncRequested(all) published");
    Ok(RpcOutcome::new(SyncAllResult { requested: true }, vec![]))
}

async fn spawn_manual_sync(requested_connection: Option<String>) -> Result<(), String> {
    let config = config_rpc::load_config_with_timeout().await?;
    let targets = match composio::list_sync_targets(&config).await {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!(
                error = %e,
                "[memory.sync] no composio sync targets available — proceeding with empty list"
            );
            Vec::new()
        }
    };

    let targets: Vec<composio::SyncTarget> = match requested_connection.as_deref() {
        Some(requested) => targets
            .into_iter()
            .filter(|target| target.connection_id == requested || target.toolkit == requested)
            .collect(),
        None => targets,
    };

    if let Some(requested) = requested_connection.as_deref() {
        if targets.is_empty() {
            emit_sync_stage(
                MemorySyncTrigger::Manual,
                MemorySyncStage::Failed,
                None,
                Some(requested),
                Some("no active provider-backed sync target matched request".to_string()),
                None, // channel-level sync — not a memory-source row
            );
            return Err(format!(
                "memory sync: no active provider-backed target matched `{requested}`"
            ));
        }
    }

    tokio::spawn(async move {
        for target in targets {
            emit_sync_stage(
                MemorySyncTrigger::Manual,
                MemorySyncStage::Fetching,
                Some(&target.toolkit),
                Some(&target.connection_id),
                Some("provider sync started".to_string()),
                None, // provider-level composio sync — not a memory-source row
            );

            match tinymemory_core::tinycortex::run_composio_connection(
                &target.toolkit,
                &target.connection_id,
                &config,
            )
            .await
            {
                Ok(outcome) => {
                    emit_sync_stage(
                        MemorySyncTrigger::Manual,
                        MemorySyncStage::Completed,
                        Some(&target.toolkit),
                        Some(&target.connection_id),
                        Some(format!(
                            "provider sync completed items_ingested={}",
                            outcome.records_ingested
                        )),
                        None, // provider-level composio sync — not a memory-source row
                    );
                }
                Err(error) => {
                    emit_sync_stage(
                        MemorySyncTrigger::Manual,
                        MemorySyncStage::Failed,
                        Some(&target.toolkit),
                        Some(&target.connection_id),
                        Some(error.to_string()),
                        None, // provider-level composio sync — not a memory-source row
                    );
                    tracing::warn!(
                        toolkit = %target.toolkit,
                        connection_id = %target.connection_id,
                        error = %error,
                        "[memory.sync] provider sync failed"
                    );
                }
            }
        }
    });

    Ok(())
}

/// Returns the current memory-ingestion status: whether a job is running, the
/// in-flight document, queue depth, and the most recent completion. Read-only,
/// safe to poll.
pub async fn memory_ingestion_status() -> Result<RpcOutcome<IngestionStatusResult>, String> {
    let snapshot = match tinymemory_core::global::client_if_ready() {
        Some(c) => c.ingestion_state().snapshot(),
        // Memory not yet initialised — report idle, no in-flight job.
        None => Default::default(),
    };
    Ok(RpcOutcome::new(
        IngestionStatusResult {
            running: snapshot.running,
            current_document_id: snapshot.current_document_id,
            current_title: snapshot.current_title,
            current_namespace: snapshot.current_namespace,
            queue_depth: snapshot.queue_depth,
            last_completed_at: snapshot.last_completed_at,
            last_document_id: snapshot.last_document_id,
            last_success: snapshot.last_success,
        },
        vec![],
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};

    use crate::core::bus::BUS;
    use crate::core::events::DomainEvent;
    use tinybus::EventHandler;

    fn test_mutex() -> &'static std::sync::Mutex<()> {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn ensure_memory_client() -> tinymemory_core::store::MemoryClientRef {
        crate::openhuman::memory::ops::ensure_shared_memory_client();
        tinymemory_core::global::client().expect("memory client")
    }

    struct ChannelCapture {
        tx: mpsc::UnboundedSender<Option<String>>,
    }

    #[async_trait]
    impl EventHandler<DomainEvent> for ChannelCapture {
        fn name(&self) -> &str {
            "memory::ops::sync::tests::capture"
        }

        fn domains(&self) -> Option<&[&str]> {
            Some(&["memory"])
        }

        async fn handle(&self, event: &DomainEvent) {
            if let DomainEvent::MemorySyncRequested { channel_id } = event {
                let _ = self.tx.send(channel_id.clone());
            }
        }
    }

    #[test]
    fn sync_channel_params_deserialize_channel_id() {
        let params: SyncChannelParams =
            serde_json::from_value(json!({"channel_id": "channel-1"})).unwrap();
        assert_eq!(params.channel_id, "channel-1");
    }

    #[test]
    fn ingestion_status_result_default_is_idle() {
        let status = IngestionStatusResult::default();
        assert!(!status.running);
        assert!(status.current_document_id.is_none());
        assert!(status.current_title.is_none());
        assert!(status.current_namespace.is_none());
        assert_eq!(status.queue_depth, 0);
        assert!(status.last_completed_at.is_none());
        assert!(status.last_document_id.is_none());
        assert!(status.last_success.is_none());
    }

    #[test]
    fn sync_result_structs_serialize_expected_fields() {
        let one = serde_json::to_value(SyncChannelResult {
            requested: true,
            channel_id: "abc".into(),
        })
        .unwrap();
        assert_eq!(one, json!({"requested": true, "channel_id": "abc"}));

        let all = serde_json::to_value(SyncAllResult { requested: true }).unwrap();
        assert_eq!(all, json!({"requested": true}));
    }

    #[tokio::test]
    async fn memory_sync_channel_publishes_targeted_event() {
        let _serial = crate::openhuman::memory::ops::GLOBAL_MEMORY_TEST_LOCK
            .lock()
            .await;
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = crate::core::bus::init().await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _subscription = BUS
            .subscribe(Arc::new(ChannelCapture { tx }))
            .expect("global bus should be initialized");

        let outcome = memory_sync_channel(SyncChannelParams {
            channel_id: "channel-123".into(),
        })
        .await
        .expect("memory_sync_channel");
        assert!(outcome.value.requested);
        assert_eq!(outcome.value.channel_id, "channel-123");

        let received = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("event should arrive before timeout")
            .expect("sender should still be connected");
        assert_eq!(received.as_deref(), Some("channel-123"));
    }

    #[tokio::test]
    async fn memory_sync_all_publishes_broadcast_event() {
        let _serial = crate::openhuman::memory::ops::GLOBAL_MEMORY_TEST_LOCK
            .lock()
            .await;
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = crate::core::bus::init().await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _subscription = BUS
            .subscribe(Arc::new(ChannelCapture { tx }))
            .expect("global bus should be initialized");

        let outcome = memory_sync_all().await.expect("memory_sync_all");
        assert!(outcome.value.requested);

        let received = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("event should arrive before timeout")
            .expect("sender should still be connected");
        assert!(
            received.is_none(),
            "sync-all should publish channel_id=None"
        );
    }

    #[tokio::test]
    async fn memory_ingestion_status_reflects_initialized_client_snapshot() {
        let _serial = crate::openhuman::memory::ops::GLOBAL_MEMORY_TEST_LOCK
            .lock()
            .await;
        let _guard = test_mutex()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let client = ensure_memory_client();
        let state = client.ingestion_state();

        // Reset any residue from background ingestion left by prior tests.
        state.reset_for_test();

        state.enqueue();
        state.mark_running("doc-sync", "Sync Title", "sync-test");

        let status = memory_ingestion_status()
            .await
            .expect("memory_ingestion_status")
            .value;

        assert!(status.running);
        assert_eq!(status.current_document_id.as_deref(), Some("doc-sync"));
        assert_eq!(status.current_title.as_deref(), Some("Sync Title"));
        assert_eq!(status.current_namespace.as_deref(), Some("sync-test"));
        assert_eq!(status.queue_depth, 1);

        state.dequeue();
        state.mark_completed("doc-sync", true, 12345);
    }
}
