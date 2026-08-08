//! [`MemorySourceSink`] for the embedded driver — the write seam the host's
//! sync machinery pushes already-fetched items through.
//!
//! ## The host keeps the loop; this file is one step of it
//!
//! `memory::sources::sync::sync_source` stays exactly where it is. It owns the
//! per-source mutex, the `emit_sync_stage` progress events, Composio billing,
//! OAuth, rate limits, and dispatch by `SourceKind` — none of which belongs
//! behind a trait a third-party driver implements. This family is only the
//! "persist these items" step at the end of that loop.
//!
//! ## TAINT — why this family writes documents and `MemoryIngest` refuses
//!
//! `taint` is host-stamped provenance a driver "must persist … and never assign
//! itself". The two write tiers differ on whether they can:
//!
//! - The **chunk** tier (`ingest_pipeline::ingest_document_with_scope`, what
//!   `run_source_pipeline` writes through) has **no taint column** anywhere
//!   along its path — not on `DocumentInput`, not on `CanonicalisedSource`, not
//!   on `Chunk::metadata`. `MemoryIngest` (M3c) therefore *refuses* a non-default
//!   taint rather than dropping it.
//! - The **namespace-document** tier (`MemoryClient::put_doc` →
//!   `NamespaceDocumentInput.taint`) carries it as a real column.
//!
//! Refusal is the right answer for `MemoryIngest`, whose callers pass the
//! default. It is the *wrong* answer here: the contract says sync paths pass
//! [`MemoryTaint::ExternalSync`], so a sink that refuses non-default taint would
//! refuse its only intended caller. So this family writes through the tier that
//! can honour the argument. The alternative — call the chunk path and let the
//! `taint` parameter fall on the floor — is the exact failure the rule exists to
//! prevent: externally-synced content landing indistinguishable from
//! user-authored content across a prompt-injection trust boundary.
//!
//! **The behaviour difference this buys is real and is not hidden.** Items
//! accepted here land as namespace documents, so they are readable through
//! `MemoryDocuments` / `MemoryCore` and are queryable, but they do **not** flow
//! through canonicalisation, chunking and the summary-tree ingest the way
//! `run_source_pipeline` output does — so they do not appear in a source's
//! summary tree. Closing that gap needs a taint column on the chunk tier, not a
//! different call here.
//!
//! ## `skipped` is always 0, deliberately
//!
//! `put_doc` upserts and returns a document id whether the key was new or
//! already present; there is no already-present signal to read. Reporting a
//! guess would corrupt the one number the contract gives a caller for detecting
//! a silently-dropping driver, so the honest value is 0 and the count lands in
//! `written`.
//!
//! ## Naming hazard
//!
//! `tinycortex::memory::sources::SourceItem` also exists and is a **different
//! type** (an engine-side sync item). Everything here is
//! [`tinycortex_api::provider::types::SourceItem`]; nothing imports the other
//! one.

use async_trait::async_trait;
use serde_json::json;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::{IngestOutcome, SourceItem};
use tinycortex_api::provider::MemorySourceSink;
use tinycortex_api::types::{MemoryTaint, NamespaceDocumentInput};

use crate::openhuman::memory::store::chunks::store as chunks;
use crate::openhuman::memory::store::chunks::types::SourceKind;

use super::{host_error, EmbeddedMemoryProvider};

/// The namespace one logical source's accepted items live in.
///
/// Keyed on `source_id` alone — **not** on `(source_kind, source_id)` — so
/// [`MemorySourceSink::forget_source`], which is given only the id, can address
/// exactly what [`MemorySourceSink::accept_source_items`] wrote. The kind is
/// carried on each document instead (`source_type` + metadata), where losing it
/// costs nothing.
fn namespace_for(source_id: &str) -> String {
    format!("source:{source_id}")
}

#[async_trait]
impl MemorySourceSink for EmbeddedMemoryProvider {
    async fn accept_source_items(
        &self,
        source_id: &str,
        source_kind: &str,
        items: Vec<SourceItem>,
        taint: MemoryTaint,
    ) -> Result<IngestOutcome, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] accept_source_items source_id={source_id} \
             source_kind={source_kind} items={} taint={}",
            items.len(),
            taint.as_db_str()
        );

        let namespace = namespace_for(source_id);
        let client = self.client().await?;
        let mut outcome = IngestOutcome::default();

        for item in items {
            if item.item_id.trim().is_empty() {
                // The item id is the upsert key. An empty one would collide
                // every such item onto a single document, silently losing all
                // but the last — refuse instead.
                return Err(MemoryError::Invalid(
                    "source item_id must not be empty: it is the dedupe key".to_string(),
                ));
            }

            let title = if item.title.trim().is_empty() {
                item.item_id.clone()
            } else {
                item.title.clone()
            };

            let input = NamespaceDocumentInput {
                namespace: namespace.clone(),
                key: item.item_id,
                title,
                content: item.content,
                source_type: source_kind.to_string(),
                priority: "medium".to_string(),
                tags: item.tags,
                metadata: json!({
                    "sourceId": source_id,
                    "sourceKind": source_kind,
                    // This is collection identity, deliberately separate
                    // from `item_id`, which is only the per-item dedupe key.
                    "path_scope": namespace,
                    "url": item.url,
                    "mime": item.mime,
                    "updatedAtMs": item.updated_at_ms,
                }),
                category: "core".to_string(),
                session_id: None,
                document_id: None,
                // The whole point of this family. Never substituted, never
                // defaulted.
                taint,
            };

            let id = client
                .put_doc(input)
                .await
                .map_err(|error| host_error("accept_source_items", error))?;
            outcome.written = outcome.written.saturating_add(1);
            outcome.ids.push(id);
        }

        Ok(outcome)
    }

    async fn forget_source(&self, source_id: &str) -> Result<u64, MemoryError> {
        log::debug!("[memory:driver:embedded] forget_source source_id={source_id}");
        let namespace = namespace_for(source_id);
        let client = self.client().await?;

        // Count before clearing: `clear_namespace` returns `()`, and the
        // contract wants the number of units removed.
        let listed = client
            .list_documents(Some(&namespace))
            .await
            .map_err(|error| host_error("forget_source", error))?;
        // `list_documents` answers `{"documents": [...]}`, not a bare array.
        let documents = listed
            .get("documents")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or(0) as u64;

        if documents > 0 {
            client
                .clear_namespace(&namespace)
                .await
                .map_err(|error| host_error("forget_source", error))?;
        }

        // Also drop chunk-tier content written for the same logical source
        // through `MemoryIngest` — the disconnect path must not leave half the
        // driver's copy of a source behind.
        //
        // **Exact match, never a prefix.** `sources::status::source_id_prefix`
        // expands a Composio source to `{toolkit}:%`, which would take out every
        // source sharing that toolkit. That over-deletion is tolerable in the
        // host's own connection-teardown path, which knows it is tearing down
        // the whole connection; behind a contract method that promises to drop
        // *one* logical source it would be a surprise. Teardown of a Composio
        // connection stays host-side in `integrations::composio::ops::
        // memory_cleanup`, which has the toolkit and connection id this family
        // deliberately never sees.
        let config = self.config().await?.clone();
        let id = source_id.to_string();
        let chunks_removed = tokio::task::spawn_blocking(move || -> anyhow::Result<usize> {
            let removed = chunks::delete_chunks_by_source(&config, SourceKind::Document, &id)?;
            // Finish off a tree left orphaned by an earlier partial delete;
            // a no-op when the chunk delete above already cascaded it.
            chunks::delete_orphaned_source_tree(&config, SourceKind::Document, &id)?;
            Ok(removed)
        })
        .await
        .map_err(|error| host_error("forget_source", format!("join: {error}")))?
        .map_err(|error| host_error("forget_source", format!("{error:#}")))?;

        log::debug!(
            "[memory:driver:embedded] forget_source source_id={source_id} documents={documents} \
             chunks={chunks_removed}"
        );
        Ok(documents + chunks_removed as u64)
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
