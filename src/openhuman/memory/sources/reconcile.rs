//! Startup/list-time reconciliation of Composio connections into the memory
//! sources registry.
//!
//! Ported from `tinymemory_core::sources::reconcile::ensure_composio_sources`,
//! which tinymemory v1.13.4 deleted along with the rest of the in-process
//! Composio pipeline it read (`sync::composio::scan_active_sync_targets`).
//! The behaviour is unchanged: scan every active Composio connection with a
//! native sync provider, upsert each into the registry, run the one-time caps
//! migration, and hand back the live active-connection set so
//! `rpc::list_rpc` can hide stale rows. Only the scan's source moved — it
//! reads through `memory::sync::composio::scan_active_sync_targets`, this
//! host's own replacement built on the `tinyconnectors` module, instead of
//! the deleted engine function of the same name.
//!
//! `apply_composio_source_caps_migration` itself is untouched and still comes
//! from the engine (`tinymemory_core::sources::reconcile`) — it never read
//! the deleted pipeline, only the registry.

use std::collections::HashSet;

use crate::openhuman::config::rpc as config_rpc;
use crate::openhuman::memory::sources::registry;

/// Reconcile active Composio connections into the memory sources registry and
/// return the live active-connection set scanned this call.
///
/// Returns `Some(connection_ids)` — the `connection_id`s of every active sync
/// target — when the live Composio scan **succeeded**, so callers (notably
/// `rpc::list_rpc`) can filter the listing down to connections that are still
/// active and dedupe identical rows. Returns `None` when the scan could not
/// run (config load / network / auth failure); callers must treat `None` as
/// "active set unavailable" and **not** hide any sources — an empty scan from
/// a transient blip must never be read as "everything is inactive".
pub async fn ensure_composio_sources() -> Option<HashSet<String>> {
    tracing::debug!("[memory_sources:reconcile] starting composio reconciliation");

    let config = match config_rpc::load_config_with_timeout().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "[memory_sources:reconcile] failed to load config; skipping"
            );
            return None;
        }
    };

    // Always hit Composio directly here — using list_sync_targets would
    // short-circuit through the registry and miss new connections.
    let targets =
        match crate::openhuman::memory::sync::composio::scan_active_sync_targets(&config).await {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "[memory_sources:reconcile] no composio sync targets available; skipping"
                );
                return None;
            }
        };

    // Build the upsert targets up front, then apply them with a single config
    // load + save via the batch path. The per-call upsert does its own
    // load-modify-save, so a per-target loop costs 2N config round-trips for N
    // connections; batching collapses that to 2.
    let upsert_targets = build_upsert_targets(&targets);
    let upserted = match registry::upsert_composio_sources_batch(&upsert_targets).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(
                targets = targets.len(),
                error = %e,
                "[memory_sources:reconcile] batch upsert failed"
            );
            0
        }
    };

    if !targets.is_empty() {
        tracing::info!(
            targets = targets.len(),
            upserted = upserted,
            "[memory_sources:reconcile] composio reconciliation complete"
        );
    }

    // Run the one-time caps migration after the reconcile loop so any
    // sources upserted just above are also considered. Still the engine's —
    // it only ever read the registry, never the deleted pipeline.
    if let Err(e) = tinymemory_core::sources::reconcile::apply_composio_source_caps_migration().await
    {
        tracing::warn!(
            error = %e,
            "[memory_sources:reconcile] caps migration failed (non-fatal, will retry next time)"
        );
    }

    // The scan succeeded — surface the live active-connection set so the list
    // path can hide rows for connections that are no longer active (re-auth /
    // token expiry mints a fresh connection_id, stranding the old row) and
    // collapse identical same-id duplicates.
    Some(targets.iter().map(|t| t.connection_id.clone()).collect())
}

/// Build the `(toolkit, connection_id, label)` upsert targets for a batch
/// reconcile from the scanned Composio sync targets.
///
/// The label is a title-cased toolkit name plus the truncated connection id so
/// distinct accounts of the same toolkit (e.g. two Gmail logins) don't all show
/// as "Gmail connection". Pure (no I/O) so it can be unit-tested directly.
fn build_upsert_targets(
    targets: &[crate::openhuman::memory::sync::composio::SyncTarget],
) -> Vec<registry::ComposioUpsertTarget> {
    targets
        .iter()
        .map(|target| {
            let label = format!(
                "{} · {}",
                title_case(&target.toolkit),
                short_id(&target.connection_id)
            );
            (target.toolkit.clone(), target.connection_id.clone(), label)
        })
        .collect()
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

fn short_id(id: &str) -> &str {
    // Show only the last 8 Unicode scalar values to keep labels compact.
    // Byte-slicing would panic if the cut point isn't a UTF-8 boundary.
    let n = id.chars().count();
    if n <= 8 {
        return id;
    }
    let skip = n - 8;
    let start = id.char_indices().nth(skip).map(|(idx, _)| idx).unwrap_or(0);
    &id[start..]
}

#[cfg(test)]
#[path = "reconcile_tests.rs"]
mod tests;
