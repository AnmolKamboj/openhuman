//! [`MemoryGraph`] for the embedded driver — the key/value and relation tier.
//!
//! ## Which host method backs which contract method
//!
//! The obvious `MemoryClient` methods are the wrong ones here, and it is worth
//! saying why rather than re-deriving it later:
//!
//! - `kv_get` returns a bare `serde_json::Value`; the contract wants a
//!   [`MemoryKvRecord`], which also carries `updated_at`. That timestamp is
//!   simply not on the value path — `KvStore::get_*` drops it too.
//! - `kv_list_namespace` returns `Vec<serde_json::Value>`, takes `&str` rather
//!   than `Option<&str>` (so it cannot address the global slice), and has no
//!   prefix or limit.
//! - `graph_query` returns camelCase JSON (`"updatedAt"`, `"evidenceCount"`,
//!   `"documentIds"`), which would need a hand-written camel→snake reader to
//!   become a [`GraphRelationRecord`] again — that is new logic, and lossy.
//!
//! So kv reads and relation reads go through `MemoryClient::kv_records` /
//! `graph_relations`, thin `pub(crate)` forwarders onto the storage layer's
//! already-typed `kv_records_*` / `graph_relations_*`. Writes go through the
//! public `kv_set` / `graph_upsert`.
//!
//! ## `kv_get` is O(slice), knowingly
//!
//! There is no single-record getter anywhere below this line: the vendored
//! `KvStore` exposes `records_namespace` / `records_global` and nothing
//! narrower. Rather than add a fourth key-transform path and a new SELECT here,
//! this reads the slice and picks the key. The right fix is a
//! `record_namespace(ns, key)` / `record_global(key)` pair **upstream in the
//! vendored `KvStore`**, not a re-implementation in the driver.
//!
//! ## Two behaviours inherited from the storage layer, not introduced here
//!
//! - **Entities and predicates are upper-cased on write** by
//!   `normalize_graph_entity` / `normalize_graph_predicate`, so a
//!   `put_relation("Alice", "owns", "Phoenix")` reads back as
//!   `("ALICE", "OWNS", "PHOENIX")`. Pinned by the storage layer's own tests;
//!   the driver must not "fix" it.
//! - **`relations` cannot return more than 300 rows per underlying statement** —
//!   every `graph_relations_*` SQL statement carries a hard-coded `LIMIT 300`.
//!   A contract `limit` above that is silently unreachable. `limit` truncates
//!   downward only.
//!
//! ## `updated_at` is not forwarded on write
//!
//! `graph_upsert_internal` stamps its own `now_ts()`. A driver must not let a
//! caller backdate a write, so [`GraphRelationRecord::updated_at`] is dropped
//! on the way in and re-read on the way out.

use async_trait::async_trait;
use serde_json::{json, Value};
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::MemoryGraph;
use tinycortex_api::types::{GraphRelationRecord, MemoryKvRecord};

use super::{host_error, EmbeddedMemoryProvider};

/// The per-statement row ceiling every `graph_relations_*` query carries.
pub(super) const RELATION_ROW_CEILING: usize = 300;

/// Rebuilds the attrs object the storage layer's `merge_graph_attrs` reads.
///
/// The record's structured fields (`evidence_count`, `document_ids`,
/// `chunk_ids`, `order_index`) live *inside* `attrs` by the host's merge
/// convention, so dropping them would silently lose the caller's evidence.
fn attrs_for_upsert(relation: &GraphRelationRecord) -> Value {
    let mut attrs = relation.attrs.as_object().cloned().unwrap_or_default();
    attrs.insert("evidence_count".to_string(), json!(relation.evidence_count));
    if !relation.document_ids.is_empty() {
        attrs.insert("document_ids".to_string(), json!(relation.document_ids));
    }
    if !relation.chunk_ids.is_empty() {
        attrs.insert("chunk_ids".to_string(), json!(relation.chunk_ids));
    }
    if let Some(order_index) = relation.order_index {
        attrs.insert("order_index".to_string(), json!(order_index));
    }
    Value::Object(attrs)
}

#[async_trait]
impl MemoryGraph for EmbeddedMemoryProvider {
    async fn kv_get(
        &self,
        namespace: Option<&str>,
        key: &str,
    ) -> Result<Option<MemoryKvRecord>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] kv_get namespace={} key_chars={}",
            namespace.unwrap_or("-"),
            key.chars().count()
        );
        // The stored key is canonicalized on write, so compare against the
        // same transform rather than the raw argument.
        let wanted = crate::openhuman::memory::store::safety::canonical_identifier(key);
        let records = self
            .client()
            .await?
            .kv_records(namespace)
            .await
            .map_err(|error| host_error("kv_get", error))?;
        Ok(records.into_iter().find(|record| record.key == wanted))
    }

    async fn kv_put(
        &self,
        namespace: Option<&str>,
        key: &str,
        value: Value,
    ) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] kv_put namespace={} key_chars={}",
            namespace.unwrap_or("-"),
            key.chars().count()
        );
        self.client()
            .await?
            .kv_set(namespace, key, &value)
            .await
            .map_err(|error| host_error("kv_put", error))
    }

    async fn kv_list(
        &self,
        namespace: Option<&str>,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryKvRecord>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] kv_list namespace={} prefix={} limit={limit}",
            namespace.unwrap_or("-"),
            prefix.unwrap_or("-")
        );
        let mut records = self
            .client()
            .await?
            .kv_records(namespace)
            .await
            .map_err(|error| host_error("kv_list", error))?;
        if let Some(prefix) = prefix {
            // Canonicalized for the same reason as `kv_get`: stored keys have
            // already been through the transform.
            let prefix = crate::openhuman::memory::store::safety::canonical_identifier(prefix);
            records.retain(|record| record.key.starts_with(&prefix));
        }
        records.truncate(limit);
        Ok(records)
    }

    async fn relations(
        &self,
        namespace: Option<&str>,
        subject: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<GraphRelationRecord>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] relations namespace={} subject={} predicate={} limit={limit}",
            namespace.unwrap_or("-"),
            subject.unwrap_or("-"),
            predicate.unwrap_or("-")
        );
        if limit > RELATION_ROW_CEILING {
            log::debug!(
                "[memory:driver:embedded] relations limit={limit} exceeds the storage ceiling \
                 {RELATION_ROW_CEILING}; the query cannot return more"
            );
        }
        let mut rows = self
            .client()
            .await?
            .graph_relations(namespace, subject, predicate)
            .await
            .map_err(|error| host_error("relations", error))?;
        rows.truncate(limit);
        Ok(rows)
    }

    async fn put_relation(&self, relation: GraphRelationRecord) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] put_relation namespace={} predicate={}",
            relation.namespace.as_deref().unwrap_or("-"),
            relation.predicate
        );
        let attrs = attrs_for_upsert(&relation);
        self.client()
            .await?
            .graph_upsert(
                relation.namespace.as_deref(),
                &relation.subject,
                &relation.predicate,
                &relation.object,
                &attrs,
            )
            .await
            .map_err(|error| host_error("put_relation", error))
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
