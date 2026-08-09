//! [`MemoryIngest`] for the embedded driver — bulk content into the chunk
//! tier.
//!
//! Both methods re-shape an [`IngestItem`] batch into the canonicaliser input
//! `memory::ingest_pipeline` already takes and hand it straight over. No
//! chunking, splitting, scoring, or dedupe decision happens in this file; all
//! of it is the engine's.
//!
//! ## TAINT — this driver REFUSES what it cannot persist
//!
//! [`IngestItem::taint`] is a host-stamped provenance marker that a driver
//! "must persist what it is given and must never assign or upgrade". The chunk
//! tier **cannot** carry it: `Chunk::metadata` has no taint field, and neither
//! `ingest_document_versioned` nor the engine's `ingest::pipeline` takes a
//! taint parameter. Taint lives on the *other* tier — `MemoryTaint` is a column
//! on the `UnifiedMemory` namespace-document path reached through
//! `Memory::store_with_taint`, which is `MemoryCore::store`'s (M3a) business,
//! not this family's.
//!
//! Three ways to handle that, and only one is defensible:
//!
//! 1. **Refuse** a non-default taint with [`MemoryError::Invalid`] — what this
//!    file does. A driver that must persist what it is given and cannot must
//!    not accept the call.
//! 2. Smuggle it into `tags` as a reserved label. That invents an on-disk
//!    convention, which this step is explicitly not allowed to do.
//! 3. Drop it silently. This is the failure mode the rule exists to prevent:
//!    externally-synced content would land indistinguishable from user-authored
//!    content, which is a prompt-injection trust boundary, not a formatting
//!    detail.
//!
//! `ingest_refuses_non_default_taint` is the security test that pins this. If a
//! future change gives the chunk tier a taint column, replace the refusal with
//! a real write — never with a drop.
//!
//! ## Fields with nowhere to go, stated rather than dropped
//!
//! - **`namespace`** — the chunk tier is keyed by `(source_kind, source_id)`
//!   and has no namespace column. Ignored.
//! - **`mime`** — the canonicaliser takes decoded text only. A text-ish MIME is
//!   accepted and dropped; anything else is [`MemoryError::Invalid`], which is
//!   the case the contract names.
//! - **`title`** — [`IngestItem`] has none, so `DocumentInput.title` is empty.
//!   `document::canonicalise` only bails when title *and* body are empty, so a
//!   body-only document still ingests.
//!
//! ## `skipped` reports dedupe, not drops
//!
//! `IngestSummary` has two "not written" signals: `already_ingested` (the
//! whole call was a dedupe no-op) and `chunks_dropped` (the fast-score path
//! rejected individual chunks). The contract's `skipped` is "units the driver
//! recognised as already present", so `already_ingested` wins when set and
//! `chunks_dropped` fills in otherwise. They are never summed — that would
//! double-count a number the caller uses to detect a silently-dropping driver.

use async_trait::async_trait;
use chrono::Utc;
use tinycortex::memory::ingest::canonicalize::chat::{ChatBatch, ChatMessage};
use tinycortex::memory::ingest::canonicalize::document::DocumentInput;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::{IngestItem, IngestOutcome};
use tinycortex_api::provider::MemoryIngest;
use tinycortex_api::types::MemoryTaint;

use crate::openhuman::memory::ingest_pipeline::{self, IngestResult};

use super::{host_error, EmbeddedMemoryProvider};

/// MIME types the canonicaliser's "already decoded to text" contract covers.
///
/// Deliberately a prefix/suffix rule rather than an exhaustive list: every
/// `text/*` type is text by definition, and the structured-text families
/// (`+json`, `+xml`) decode to text too. Anything else — a PDF, an image, an
/// archive — is content this path cannot honestly ingest as a string.
fn is_text_mime(mime: &str) -> bool {
    let mime = mime.trim().to_ascii_lowercase();
    let base = mime.split(';').next().unwrap_or("").trim().to_string();
    base.starts_with("text/")
        || base.ends_with("+json")
        || base.ends_with("+xml")
        || matches!(
            base.as_str(),
            "application/json" | "application/xml" | "application/x-ndjson"
        )
}

/// The checks every item must pass regardless of which method received it.
fn validate(item: &IngestItem) -> Result<(), MemoryError> {
    if item.taint != MemoryTaint::default() {
        // See the module docs. This is a refusal, not a limitation to route
        // around.
        return Err(MemoryError::Invalid(format!(
            "ingest cannot preserve taint '{}': the chunk tier has no taint column, and a \
             driver must never silently downgrade provenance",
            item.taint.as_db_str()
        )));
    }
    if let Some(mime) = item.mime.as_deref() {
        if !is_text_mime(mime) {
            return Err(MemoryError::Invalid(format!(
                "unsupported MIME '{mime}': ingest accepts decoded text only"
            )));
        }
    }
    if item.content.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "ingest content must not be empty".to_string(),
        ));
    }
    Ok(())
}

/// Maps the engine's ingest summary onto the contract's outcome.
fn to_outcome(result: IngestResult) -> IngestOutcome {
    IngestOutcome {
        written: u32::try_from(result.chunks_written).unwrap_or(u32::MAX),
        skipped: if result.already_ingested {
            1
        } else {
            u32::try_from(result.chunks_dropped).unwrap_or(u32::MAX)
        },
        ids: result.chunk_ids,
    }
}

#[async_trait]
impl MemoryIngest for EmbeddedMemoryProvider {
    async fn ingest_document(&self, item: IngestItem) -> Result<IngestOutcome, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] ingest_document source={} source_id={} content_chars={}",
            item.source.as_str(),
            item.source_id,
            item.content.chars().count()
        );
        validate(&item)?;

        let doc = DocumentInput {
            provider: item.source.as_str().to_string(),
            // `IngestItem` carries no title; see the module docs.
            title: String::new(),
            body: item.content,
            modified_at: item.timestamp.unwrap_or_else(Utc::now),
            source_ref: item.source_ref.map(|source_ref| source_ref.value),
        };

        let config = self.config().await?;
        ingest_pipeline::ingest_document_with_scope(
            config,
            &item.source_id,
            &item.owner,
            item.tags,
            doc,
            item.path_scope,
        )
        .await
        .map(to_outcome)
        .map_err(|error| host_error("ingest_document", format!("{error:#}")))
    }

    async fn ingest_chat(&self, messages: Vec<IngestItem>) -> Result<IngestOutcome, MemoryError> {
        log::debug!(
            "[memory:driver:embedded] ingest_chat items={}",
            messages.len()
        );
        // The canonicaliser treats an empty batch as nothing to ingest, so
        // this short-circuit changes no behaviour — it just avoids reading the
        // config and touching the store to do nothing.
        let Some(first) = messages.first() else {
            return Ok(IngestOutcome::default());
        };

        let source_id = first.source_id.clone();
        let owner = first.owner.clone();
        let tags = first.tags.clone();
        let platform = first.source.as_str().to_string();

        for item in &messages {
            validate(item)?;
            // The contract says the batch shares one conversation and that
            // ordering within it is significant. A batch spanning two sources
            // would be silently attributed to the first one's `source_id`,
            // which is the dedupe key — so refuse instead of guessing.
            if item.source_id != source_id {
                return Err(MemoryError::Invalid(format!(
                    "ingest_chat batch mixes source ids ('{source_id}' and '{}'); one batch is \
                     one conversation",
                    item.source_id
                )));
            }
        }

        let batch = ChatBatch {
            platform,
            channel_label: source_id.clone(),
            messages: messages
                .into_iter()
                .map(|item| ChatMessage {
                    // The only author-ish field `IngestItem` has.
                    author: item.owner,
                    timestamp: item.timestamp.unwrap_or_else(Utc::now),
                    text: item.content,
                    source_ref: item.source_ref.map(|source_ref| source_ref.value),
                })
                .collect(),
        };

        let config = self.config().await?;
        ingest_pipeline::ingest_chat(config, &source_id, &owner, tags, batch)
            .await
            .map(to_outcome)
            .map_err(|error| host_error("ingest_chat", format!("{error:#}")))
    }
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;
