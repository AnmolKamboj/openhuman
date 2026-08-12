//! [`MemoryTree`] for the embedded driver — the markdown time-summary tree.
//!
//! ## The family is `tree_runtime`, not `tree::retrieval`
//!
//! This is worth stating up front because the obvious reading of the method
//! names points at the wrong module. `tree::retrieval::{query_source,
//! drill_down}` are *retrieval* entry points: they return `QueryResponse<…>` /
//! `Vec<RetrievalHit>` over the hybrid ranker, and the `TreeStatus` in
//! `memory::store::trees::types` is a **different type with the same name** (an
//! enum with an `Active` variant, describing a sealed source tree).
//!
//! The contract's [`IngestRequest`], [`QueryResult`], [`TreeNode`] and
//! [`TreeStatus`] are the *runtime* tree's types — literally so:
//! `memory::tree::tree_runtime` re-exports
//! `tinycortex::memory::tree::runtime::*`, and that module in turn is
//! `pub use tinycortex_api::tree as types`. Contract type and host type are
//! **the same type**, so three of the five methods below convert nothing.
//!
//! ## The source-scope hazard does not arise here
//!
//! The M3c brief warned that threading `scope` into `retrieval::query_source`
//! would apply the allowlist twice — once as the explicit parameter and once
//! through the `current_source_scope()` task-local that callee reads
//! internally. That warning is real, and this file avoids it by not going
//! there: [`Self::query_source`] reads the **chunk store**
//! (`store::chunks::store::list_chunks`), whose `ListChunksQuery.source_scope`
//! is already an explicit parameter applied **in SQL before `LIMIT`**.
//!
//! That SQL predicate is predicate 3 of the three pinned by
//! `tree::retrieval::source_scope_tests`, and it is the one
//! [`SourceScope::allows_source_id`] was written against — equality or
//! `mem_src:{allowed}:` prefix, untagged content fails open, an empty allow
//! list keeps only untagged rows. So no predicate changes, no task-local is
//! read, and all 25 characterization tests stay untouched.
//!
//! ## `namespace` has no home on the chunk tier
//!
//! `mem_tree_chunks` has no namespace column — chunks are keyed by
//! `(source_kind, source_id)`. [`Self::query_source`] therefore *validates*
//! `namespace` (so a traversal attempt is still refused) and otherwise ignores
//! it. Said out loud rather than dropped silently.
//!
//! ## Sealing needs a summarisation model
//!
//! [`Self::seal`] and [`Self::cascade`] drive the LLM fold, so they resolve a
//! provider through `tree_runtime::ops::create_provider` — the same resolver
//! the `tree_summarizer.*` RPC path and the memory doctor use. When the host
//! has neither local AI nor `memory_tree.cloud_summarization_opt_in`, that
//! resolver fails and the call surfaces as [`MemoryError::Invalid`] carrying
//! the existing operator-facing message.
//!
//! Both short-circuit when there is nothing to do — an empty buffer for
//! `seal`, an empty tree for `cascade` — and return the current status without
//! resolving a provider at all. The contract requires both to be idempotent
//! no-ops in exactly those cases, and a no-op must not need a model.

use async_trait::async_trait;
use std::collections::HashSet;
use tinycortex_api::chunks::Chunk;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::SourceScope;
use tinycortex_api::provider::MemoryTree;
use tinycortex_api::tree::{IngestRequest, QueryResult, TreeStatus};

use crate::openhuman::config::Config;
use crate::openhuman::memory::store::chunks::store::{list_chunks, ListChunksQuery};
use crate::openhuman::memory::tree::tree_runtime::{engine, ops, store};

use super::{host_error, EmbeddedMemoryProvider};

/// Runs a blocking store call on the blocking pool with an owned `Config`.
///
/// Every `tree_runtime::store` and `chunks::store` entry point is synchronous
/// and hits SQLite or the filesystem; calling one straight from an async
/// contract method would stall the reactor.
async fn blocking<T, F>(config: &Config, context: &'static str, run: F) -> Result<T, MemoryError>
where
    T: Send + 'static,
    F: FnOnce(&Config) -> anyhow::Result<T> + Send + 'static,
{
    let config = config.clone();
    tokio::task::spawn_blocking(move || run(&config))
        .await
        .map_err(|error| host_error(context, format!("join error: {error}")))?
        .map_err(|error| host_error(context, format!("{error:#}")))
}

/// `validate_namespace` / `validate_node_id` failures are caller errors, so
/// they become [`MemoryError::Invalid`] rather than the opaque `Other` the
/// host's `String` channel would otherwise collapse to.
fn invalid(reason: String) -> MemoryError {
    MemoryError::Invalid(reason)
}

#[async_trait]
impl MemoryTree for EmbeddedMemoryProvider {
    async fn append(&self, request: IngestRequest) -> Result<(), MemoryError> {
        log::debug!(
            "[memory:driver:embedded] tree_append namespace={} content_chars={} has_metadata={}",
            request.namespace,
            request.content.chars().count(),
            request.metadata.is_some()
        );
        store::validate_namespace(&request.namespace).map_err(invalid)?;
        if request.content.trim().is_empty() {
            return Err(invalid("content must not be empty".to_string()));
        }

        let config = self.config().await?;
        // Mirrors `ops::tree_summarizer_ingest` exactly: trimmed namespace,
        // ingest-time fallback for the timestamp. The returned buffer path is
        // an implementation detail and is dropped.
        let namespace = request.namespace.trim().to_string();
        let timestamp = request.timestamp.unwrap_or_else(chrono::Utc::now);
        let content = request.content;
        let metadata = request.metadata;
        blocking(config, "tree_append", move |config| {
            store::buffer_write(config, &namespace, &content, &timestamp, metadata.as_ref())
                .map(|_path| ())
        })
        .await
    }

    async fn query_source(
        &self,
        namespace: &str,
        source_id: &str,
        limit: usize,
        scope: Option<&SourceScope>,
    ) -> Result<Vec<Chunk>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] tree_query_source namespace={namespace} \
             source_id={source_id} limit={limit} scoped={}",
            scope.is_some()
        );
        // Validated but not used as a filter — see the module docs.
        store::validate_namespace(namespace).map_err(invalid)?;

        let query = ListChunksQuery {
            source_id: Some(source_id.to_string()),
            // The allowlist travels into SQL, applied before `LIMIT`. This is
            // the whole point of the contract taking `scope` as a parameter.
            source_scope: scope
                .map(|scope| scope.allow.iter().cloned().collect::<HashSet<String>>()),
            limit: Some(limit),
            exclude_dropped: true,
            ..ListChunksQuery::default()
        };

        let config = self.config().await?;
        // `ORDER BY timestamp_ms DESC` in `list_chunks` is the contract's
        // "newest first"; no re-sorting here.
        blocking(config, "tree_query_source", move |config| {
            list_chunks(config, &query)
        })
        .await
    }

    async fn drill_down(&self, namespace: &str, node_id: &str) -> Result<QueryResult, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] tree_drill_down namespace={namespace} node_id={node_id}"
        );
        store::validate_namespace(namespace).map_err(invalid)?;
        store::validate_node_id(node_id).map_err(invalid)?;

        let config = self.config().await?;
        let namespace = namespace.trim().to_string();
        let node_id = node_id.to_string();
        let found = {
            let namespace = namespace.clone();
            let node_id = node_id.clone();
            blocking(config, "tree_drill_down", move |config| {
                let Some(node) = store::read_node(config, &namespace, &node_id)? else {
                    return Ok(None);
                };
                let children = store::read_children(config, &namespace, &node_id)?;
                Ok(Some(QueryResult { node, children }))
            })
            .await?
        };

        // The contract mandates `NotFound` here. The RPC path returns a
        // `String` for the same case, which is why this is constructed in the
        // driver rather than mapped from below.
        found.ok_or_else(|| {
            MemoryError::NotFound(format!("tree node '{node_id}' not found in '{namespace}'"))
        })
    }

    async fn seal(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        log::debug!("[memory:driver:embedded] tree_seal namespace={namespace}");
        store::validate_namespace(namespace).map_err(invalid)?;
        let config = self.config().await?;
        let namespace = namespace.trim().to_string();

        // Nothing buffered ⇒ nothing to seal. Short-circuited *before* the
        // provider is resolved so a scheduler may call `seal` unconditionally
        // on a host with no summarisation model without seeing an error.
        let buffered = {
            let namespace = namespace.clone();
            blocking(config, "tree_seal_buffer_read", move |config| {
                store::buffer_read(config, &namespace)
            })
            .await?
        };
        if !buffered.is_empty() {
            let provider = ops::create_provider(config)
                .map_err(invalid)
                .map(|(provider, _model)| provider)?;
            // `Ok(None)` means the buffer emptied under us — still a success.
            engine::run_summarization(config, provider.as_ref(), &namespace, chrono::Utc::now())
                .await
                .map_err(|error| host_error("tree_seal", format!("{error:#}")))?;
        }

        blocking(config, "tree_seal_status", move |config| {
            store::get_tree_status(config, &namespace)
        })
        .await
    }

    async fn cascade(&self, namespace: &str) -> Result<TreeStatus, MemoryError> {
        log::debug!("[memory:driver:embedded] tree_cascade namespace={namespace}");
        store::validate_namespace(namespace).map_err(invalid)?;
        let config = self.config().await?;
        let namespace = namespace.trim().to_string();

        // An empty tree has no leaves to roll up. Same short-circuit rationale
        // as `seal`.
        let status = {
            let namespace = namespace.clone();
            blocking(config, "tree_cascade_status", move |config| {
                store::get_tree_status(config, &namespace)
            })
            .await?
        };
        if status.total_nodes == 0 {
            return Ok(status);
        }

        let provider = ops::create_provider(config)
            .map_err(invalid)
            .map(|(provider, _model)| provider)?;
        // NOTE: `rebuild_tree` recomputes every parent level from the hour
        // leaves rather than incrementally rolling up only what changed. Same
        // direction and same resulting state as the contract's "roll sealed
        // leaves up through the parent levels", and idempotent as required —
        // but more expensive than the word "cascade" suggests. There is no
        // incremental host entry point to delegate to, and writing one would
        // be engine logic.
        engine::rebuild_tree(config, provider.as_ref(), &namespace)
            .await
            .map_err(|error| host_error("tree_cascade", format!("{error:#}")))
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
