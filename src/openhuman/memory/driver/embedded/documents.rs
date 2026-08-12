//! [`MemoryDocuments`] for the embedded driver — the namespace-document tier.
//!
//! ## The contract types *are* the host types
//!
//! `NamespaceDocumentInput`, `StoredMemoryDocument` and
//! `NamespaceRetrievalContext` in
//! [`crate::openhuman::memory::store::types`] are `pub use`d from
//! `tinycortex::memory`, which in turn re-exports `tinycortex_api::types`.
//! Same crate, same types — so there is no conversion in this file, only
//! signature shape (`usize` → `u32`, `Result<_, String>` →
//! [`MemoryError`]).
//!
//! ## `put_document` keeps the full pipeline
//!
//! It delegates to `MemoryClient::put_doc`, which persists and then enqueues a
//! background graph-extraction job — not `put_doc_light`, which skips vector
//! and graph indexing. The contract says nothing about extraction, so the
//! driver inherits the host's normal write behaviour rather than quietly
//! choosing the cheaper one.
//!
//! ## `get_document` had no host entry point
//!
//! There was no read-one-by-key path anywhere: `MemoryClient` has
//! `put_doc` / `list_documents` / `delete_document`, and `list_documents`'
//! SELECT carries no `content` column. `UnifiedMemory::get_document_by_key`
//! (added alongside this family) is `load_documents_for_scope`'s SELECT with a
//! `key` predicate — a filter on an existing query, not retrieval logic. It
//! canonicalizes the key through
//! [`canonical_document_key`](crate::openhuman::memory::store::safety::canonical_document_key),
//! the same transform the write path applies, so a PII-shaped key written by
//! `put_document` reads back rather than silently missing (#5164).

use async_trait::async_trait;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::MemoryDocuments;
use tinycortex_api::types::{
    NamespaceDocumentInput, NamespaceRetrievalContext, StoredMemoryDocument,
};

use super::{host_error, EmbeddedMemoryProvider};

#[async_trait]
impl MemoryDocuments for EmbeddedMemoryProvider {
    async fn put_document(&self, input: NamespaceDocumentInput) -> Result<String, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] put_document namespace={} key_chars={} content_chars={}",
            input.namespace,
            input.key.chars().count(),
            input.content.chars().count()
        );
        self.client()
            .await?
            .put_doc(input)
            .await
            .map_err(|error| host_error("put_document", error))
    }

    async fn get_document(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<StoredMemoryDocument>, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] get_document namespace={namespace} key_chars={}",
            key.chars().count()
        );
        self.client()
            .await?
            .get_document(namespace, key)
            .await
            .map_err(|error| host_error("get_document", error))
    }

    async fn query_documents(
        &self,
        namespace: &str,
        query: &str,
        limit: usize,
    ) -> Result<NamespaceRetrievalContext, MemoryError> {
        // The host's chunk budget is a `u32`; saturate rather than wrap.
        let max_chunks = u32::try_from(limit).unwrap_or(u32::MAX);
        log::debug!(
            "[memory:driver:embedded] query_documents namespace={namespace} query_len={} \
             max_chunks={max_chunks}",
            query.len()
        );
        self.client()
            .await?
            .query_namespace_context_data(namespace, query, max_chunks)
            .await
            .map_err(|error| host_error("query_documents", error))
    }
}

#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
