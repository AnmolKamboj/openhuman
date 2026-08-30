//! JSON-RPC handler functions for the Composio-backed Slack provider.
//!
//! Moved from `memory::slack_ingestion::rpc` into this module so the
//! entire Slack integration lives under `composio::providers::slack`.
//!
//! Public JSON-RPC surface:
//! - `openhuman.slack_memory_sync_trigger` — run `SlackProvider::sync()`
//!   once for each active Slack connection (or just one, if
//!   `connection_id` is supplied).
//! - `openhuman.slack_memory_sync_status` — list the per-connection
//!   sync cursors + last-synced timestamps.

use serde::{Deserialize, Serialize};

use crate::openhuman::config::Config;
use crate::openhuman::integrations::composio::client::{
    create_composio_client, direct_list_connections, ComposioClientKind,
};
use crate::openhuman::integrations::composio::types::ComposioConnectionsResponse;
use crate::openhuman::memory::sync::composio::providers::SyncOutcome;
use crate::rpc::RpcOutcome;

/// Optional connection-id override for the trigger. When absent, all
/// active Slack connections are synced (serially, one-by-one).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyncTriggerRequest {
    #[serde(default)]
    pub connection_id: Option<String>,
}

/// Result of `slack_memory_sync_trigger` — per-connection [`SyncOutcome`]s
/// plus aggregate counters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncTriggerResponse {
    pub outcomes: Vec<SyncOutcome>,
    pub connections_considered: usize,
    pub connections_synced: usize,
}

/// Mode-aware connection listing shared by `sync_trigger_rpc` and
/// `sync_status_rpc`. Returns the raw `ComposioConnectionsResponse`
/// (all toolkits, all statuses) — callers filter for slack + active
/// downstream so each RPC owns its own filter semantics.
///
/// Mirrors `composio::ops::composio_list_connections` (#1710): both
/// the backend arm and the direct arm share the same downstream
/// filtering, identical error wrapping, distinct log prefixes for
/// debuggability.
async fn list_slack_connections(config: &Config) -> Result<ComposioConnectionsResponse, String> {
    let kind = create_composio_client(config)
        .map_err(|e| format!("[slack_ingest] list_connections: {e}"))?;
    match kind {
        ComposioClientKind::Backend(client) => client
            .list_connections()
            .await
            .map_err(|e| format!("[slack_ingest] list_connections (backend) failed: {e:#}")),
        ComposioClientKind::Direct(direct) => direct_list_connections(&direct)
            .await
            .map_err(|e| format!("[slack_ingest] list_connections (direct) failed: {e:#}")),
    }
}

/// Run `SlackProvider::sync()` once for every active Slack connection
/// (or exactly one, if `connection_id` is provided). Fails if the
/// user is not signed in (no Composio JWT available).
pub async fn sync_trigger_rpc(
    config: &Config,
    req: SyncTriggerRequest,
) -> Result<RpcOutcome<SyncTriggerResponse>, String> {
    // Route through the mode-aware factory so direct-mode users
    // discover slack connections from THEIR personal Composio tenant —
    // not the tinyhumans backend tenant. Mirrors `composio::ops`
    // (#1710).
    let connections = list_slack_connections(config).await?;

    let mut candidates: Vec<_> = connections
        .connections
        .into_iter()
        .filter(|c| c.normalized_toolkit() == "slack" && c.is_active())
        .collect();

    if let Some(ref wanted) = req.connection_id {
        candidates.retain(|c| &c.id == wanted);
        if candidates.is_empty() {
            return Err(format!(
                "[slack_ingest] no active Slack connection with id={wanted}"
            ));
        }
    }

    let considered = candidates.len();
    let mut outcomes: Vec<SyncOutcome> = Vec::with_capacity(considered);

    // Resolved once, outside the loop: the binding is cached per workspace, but
    // a driver that serves no sync should refuse the whole request rather than
    // once per connection.
    let binding = crate::openhuman::memory::binding::for_config(config)?;
    let sync = binding.provider().as_source_sync().ok_or_else(|| {
        format!(
            "the bound memory driver '{}' does not serve source sync",
            binding.driver_id()
        )
    })?;

    for conn in candidates {
        let started_at_ms = now_ms();
        match sync
            .run_connection_sync("slack", &conn.id)
            .await
            .map_err(|error| error.to_string())
        {
            Ok(outcome) => outcomes.push(SyncOutcome {
                toolkit: "slack".to_string(),
                connection_id: Some(conn.id.clone()),
                reason: "manual".to_string(),
                items_ingested: outcome.records_ingested as usize,
                started_at_ms,
                finished_at_ms: now_ms(),
                summary: outcome
                    .note
                    .unwrap_or_else(|| "Slack sync completed".to_string()),
                details: serde_json::json!({
                    "more_pending": outcome.more_pending,
                    "actions_called": outcome.actions_called,
                    "provider_cost_usd": outcome.provider_cost_usd,
                }),
            }),
            Err(err) => {
                log::warn!(
                    "[slack_ingest] connection={} sync failed: {err:#} (continuing)",
                    conn.id
                );
            }
        }
    }

    let synced = outcomes.len();
    Ok(RpcOutcome::single_log(
        SyncTriggerResponse {
            outcomes,
            connections_considered: considered,
            connections_synced: synced,
        },
        format!("slack_ingest: trigger considered={considered} synced={synced}"),
    ))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Request body for `slack_memory_sync_status` — no parameters.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncStatusRequest {}

/// Response body for `slack_memory_sync_status` — one row per active
/// Slack Composio connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    pub connections: Vec<ConnectionStatus>,
}

/// Per-connection sync state snapshot pulled from the Composio sync-state KV.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connection_id: String,
    /// JSON-encoded per-channel cursors (see
    /// `composio::providers::slack::sync::ChannelCursors`). Empty map
    /// when no channels have been flushed yet.
    pub per_channel_cursors: String,
    pub synced_ids_count: usize,
    pub requests_used_today: u32,
    pub daily_request_limit: u32,
}

/// Report one row per active Slack Composio connection, pulled from
/// the Composio sync-state KV store.
pub async fn sync_status_rpc(
    config: &Config,
    _req: SyncStatusRequest,
) -> Result<RpcOutcome<SyncStatusResponse>, String> {
    // Route through the mode-aware factory so direct-mode users see
    // status rows for THEIR slack connections, not the tinyhumans
    // backend tenant's (#1710).
    let connections = list_slack_connections(config).await?;

    // The state rows come from the driver now. Resolved once for the whole
    // report rather than per connection: a driver that serves no sync has
    // nothing to say about any of them.
    let binding = crate::openhuman::memory::binding::for_config(config)?;
    let sync = binding.provider().as_source_sync().ok_or_else(|| {
        format!(
            "the bound memory driver '{}' does not serve source sync",
            binding.driver_id()
        )
    })?;

    let mut rows = Vec::new();
    for conn in connections.connections {
        if conn.normalized_toolkit() != "slack" {
            continue;
        }
        if !conn.is_active() {
            continue;
        }
        let state = match sync.source_sync_state("slack", &conn.id).await {
            Ok(s) => s,
            Err(err) => {
                log::warn!(
                    "[slack_ingest] load_state connection={} failed: {err:#}",
                    conn.id
                );
                continue;
            }
        };
        // `None` is a connection that has never synced. The engine call this
        // replaced returned a freshly defaulted state for that case, so the row
        // it produced was all zeroes and an empty cursor — which is what
        // `unwrap_or_default` reproduces exactly. Skipping the row instead
        // would drop a connected source from the report the moment it was
        // connected and before its first sync.
        let state = state.unwrap_or_default();
        rows.push(ConnectionStatus {
            connection_id: conn.id.clone(),
            per_channel_cursors: state.cursor.clone().unwrap_or_else(|| "{}".to_string()),
            // The contract carries the count where the engine carried the set
            // itself. Same number, and the set was never read here for anything
            // but its length.
            synced_ids_count: usize::try_from(state.synced_item_count).unwrap_or(usize::MAX),
            requests_used_today: state.daily_requests_used,
            daily_request_limit: state.daily_request_limit,
        });
    }

    let count = rows.len();
    Ok(RpcOutcome::single_log(
        SyncStatusResponse { connections: rows },
        format!("slack_ingest: status connections={count}"),
    ))
}

// ── Tests ───────────────────────────────────────────────────────────
//
// `list_slack_connections` is the shared mode-aware connection-listing
// helper introduced when this RPC pair migrated from
// `build_composio_client` to the factory (#1710 Option C). The tests
// below cover the matrix the migration unlocks — backend mode without a
// session, direct mode without an api_key, and direct mode with an
// api_key (mode-resolution observed without going to the network).
//
// We deliberately avoid hitting `backend.composio.dev` from the test
// runner: the existing pattern across this module is to assert factory
// dispatch + error wrapping rather than mock the upstream HTTP. The
// network-touching paths are smoke-tested upstream in
// `composio::client_tests` / `composio::ops_tests` and the
// direct-mode-toggle test in `action_tool.rs`.

#[cfg(test)]
#[path = "rpc_tests.rs"]
mod tests;
