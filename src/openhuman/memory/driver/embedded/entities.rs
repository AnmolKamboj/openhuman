//! [`MemoryEntities`] for the embedded driver — the entity index and its
//! hotness counters.
//!
//! ## Two rankers behind one method
//!
//! The contract's [`MemoryEntities::entities`] ranks "by hotness when `query`
//! is `None` and by match quality otherwise". Those are two different host
//! surfaces, not one with a flag:
//!
//! - `query = Some` → `tree::retrieval::search_entities`, the engine's ranked
//!   surface-form search, returning `EntityMatch`.
//! - `query = None` → `read_rpc::top_entities_rpc`, a `GROUP BY` over
//!   `mem_tree_entity_index` ordered by mention count then recency, returning
//!   `read_rpc::types::EntityRef`.
//!
//! ## Where `hotness` comes from
//!
//! Neither ranker computes it — both are pure SQL over the index. The hotness
//! signal lives in a separate table (`mem_tree_entity_hotness`, read through
//! `store::trees::hotness::get`) and is turned into a scalar by
//! `TreePolicy::topic_hotness`, the host's existing formula. This file calls
//! that formula; it does not define one. An entity with no hotness row scores
//! `0.0`, which is what `topic_hotness` returns for zero signal anyway.
//!
//! That costs one extra read per returned row. It is bounded by the `limit`
//! already applied by the ranker, and there is no batch getter below this line
//! to use instead.
//!
//! ## `entity_edges` is a projection of the co-occurrence table — read this
//!
//! There is **no host function that returns a `GraphRelationRecord` for an
//! entity-index id.** Two things look like one and are not:
//!
//! - `memory::store::namespace_store::graph::graph_relations_namespace` is the
//!   *namespace-document* graph (subject/predicate/object extracted from
//!   documents). Its subjects are document entity strings, not
//!   `mem_tree_entity_index` canonical ids, so joining the two would silently
//!   mix id spaces. That surface is already exposed properly, as
//!   [`MemoryGraph::relations`](tinycortex_api::provider::MemoryGraph::relations).
//! - `memory::tree::graph::store::neighbors` is the *entity-index*
//!   co-occurrence table, keyed by exactly the right id — but it is undirected
//!   and carries only a weight.
//!
//! `neighbors` is the honest backing, so this method projects it into the
//! contract shape with a **single fixed predicate**, `"co_occurs_with"`.
//! `evidence_count` is the real co-occurrence count; `attrs` is `null`,
//! `updated_at` is `0.0`, and `document_ids` / `chunk_ids` are empty because
//! the table stores none of them. A reader who sees `GraphRelationRecord` here
//! must not assume the richer graph tier — that is what this paragraph is for.

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::{EntityHit, EntityRef};
use tinycortex_api::provider::MemoryEntities;
use tinycortex_api::types::GraphRelationRecord;

use crate::openhuman::config::Config;
use crate::openhuman::memory::read_rpc;
use crate::openhuman::memory::store::trees::hotness;
use crate::openhuman::memory::tree::graph::store as graph_store;
use crate::openhuman::memory::tree::retrieval::search_entities;
use crate::openhuman::memory::tree_policy::TreePolicy;

use super::{host_error, EmbeddedMemoryProvider};

/// The predicate materialised for every projected co-occurrence edge.
///
/// A constant rather than an inline literal so a caller can match on it and a
/// test can assert it without duplicating the string.
pub(super) const CO_OCCURRENCE_PREDICATE: &str = "co_occurs_with";

/// The host's hotness scalar for one entity, or `0.0` when it has no counters.
///
/// Blocking (SQLite) — call from the blocking pool.
fn hotness_for(config: &Config, entity_id: &str) -> f64 {
    match hotness::get(config, entity_id) {
        Ok(Some(counters)) => TreePolicy::topic().topic_hotness(
            entity_id,
            &counters.stats(),
            Utc::now().timestamp_millis(),
        ) as f64,
        Ok(None) => 0.0,
        Err(error) => {
            // A missing hotness row is not an error, and neither is a failed
            // read: ranking degrades, the entity list does not disappear.
            log::warn!("[memory:driver:embedded] hotness read failed: {error:#}");
            0.0
        }
    }
}

/// Attaches hotness to a batch of `(id, kind, name, mentions)` tuples.
async fn with_hotness(
    config: &Config,
    rows: Vec<(String, String, String, u32)>,
) -> Result<Vec<EntityHit>, MemoryError> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || {
        rows.into_iter()
            .map(|(id, kind, name, mentions)| {
                let hotness = hotness_for(&config, &id);
                EntityHit {
                    entity: EntityRef { id, kind, name },
                    hotness,
                    mentions,
                }
            })
            .collect()
    })
    .await
    .map_err(|error| host_error("entities_hotness", format!("join error: {error}")))
}

#[async_trait]
impl MemoryEntities for EmbeddedMemoryProvider {
    async fn entities(
        &self,
        namespace: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EntityHit>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] entities namespace={namespace} has_query={} limit={limit}",
            query.is_some()
        );
        // `mem_tree_entity_index` has no namespace column — the index is
        // process-wide within a workspace. `namespace` is accepted for
        // contract shape and deliberately not used as a filter; inventing one
        // would be a new predicate, not a delegation.
        let _ = namespace;

        let config = self.config().await?;
        let rows = match query {
            Some(query) => search_entities(config, query, None, limit)
                .await
                .map_err(|error| host_error("entities_search", format!("{error:#}")))?
                .into_iter()
                .map(|hit| {
                    (
                        hit.canonical_id,
                        hit.kind.as_str().to_string(),
                        hit.surface,
                        u32::try_from(hit.mention_count).unwrap_or(u32::MAX),
                    )
                })
                .collect::<Vec<_>>(),
            None => {
                read_rpc::top_entities_rpc(config, None, u32::try_from(limit).unwrap_or(u32::MAX))
                    .await
                    .map_err(|error| host_error("entities_top", error))?
                    .value
                    .into_iter()
                    .map(|entity| (entity.entity_id, entity.kind, entity.surface, entity.count))
                    .collect::<Vec<_>>()
            }
        };

        with_hotness(config, rows).await
    }

    async fn entity_edges(
        &self,
        namespace: &str,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        log::debug!("[memory:driver:embedded] entity_edges namespace={namespace} limit={limit}");
        // Same reason as `entities`: the co-occurrence table is not
        // namespace-keyed.
        let _ = namespace;

        let config = self.config().await?.clone();
        let subject = entity_id.to_string();
        let neighbours = tokio::task::spawn_blocking(move || {
            graph_store::neighbors(&config, &subject).map(|mut rows| {
                // `neighbors` has no limit parameter, so the ceiling is applied
                // here. Rows already arrive weight-descending from the engine's
                // query; truncation therefore keeps the strongest edges.
                rows.truncate(limit);
                rows
            })
        })
        .await
        .map_err(|error| host_error("entity_edges", format!("join error: {error}")))?
        .map_err(|error| host_error("entity_edges", format!("{error:#}")))?;

        // An unknown entity yields no rows, which the contract says must be an
        // empty vector rather than `NotFound`.
        Ok(neighbours
            .into_iter()
            .map(|(object, weight)| GraphRelationRecord {
                namespace: None,
                subject: entity_id.to_string(),
                predicate: CO_OCCURRENCE_PREDICATE.to_string(),
                object,
                attrs: Value::Null,
                updated_at: 0.0,
                evidence_count: u32::try_from(weight.max(0)).unwrap_or(u32::MAX),
                order_index: None,
                document_ids: Vec::new(),
                chunk_ids: Vec::new(),
            })
            .collect())
    }

    async fn touch_entities(
        &self,
        namespace: &str,
        entity_ids: &[String],
    ) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] touch_entities namespace={namespace} n={}",
            entity_ids.len()
        );
        let _ = namespace;
        if entity_ids.is_empty() {
            return Ok(());
        }

        let config = self.config().await?.clone();
        let entity_ids = entity_ids.to_vec();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let now_ms = Utc::now().timestamp_millis();
            for entity_id in &entity_ids {
                // `get_or_fresh` creates a zeroed row for an id the index has
                // never seen, which is how "unknown ids are ignored, not
                // rejected" is satisfied without a pre-existence check.
                let mut counters = hotness::get_or_fresh(&config, entity_id)?;
                counters.mention_count_30d = counters.mention_count_30d.saturating_add(1);
                counters.last_seen_ms = Some(now_ms);
                counters.last_updated_ms = now_ms;
                hotness::upsert(&config, &counters)?;
            }
            Ok(())
        })
        .await
        .map_err(|error| host_error("touch_entities", format!("join error: {error}")))?
        .map_err(|error| host_error("touch_entities", format!("{error:#}")))
    }
}

#[cfg(test)]
#[path = "entities_tests.rs"]
mod tests;
