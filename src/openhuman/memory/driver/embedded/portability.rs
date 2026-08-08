//! [`MemoryPortability`] for the embedded driver — the export/import pair that
//! makes binding this driver reversible.
//!
//! ## There was no host entry point, so this one is composed
//!
//! Unlike the other mandatory families, portability had nothing to delegate to.
//! It is composed from two existing engine calls — `namespace_summaries()` and
//! `list()` — and deliberately *not* from `MemoryClient::list_documents`, whose
//! SQL selects `document_id, namespace, key, title, source_type, priority,
//! created_at, updated_at, taint` and **no `content`**: an export built on it
//! would round-trip metadata and lose every byte of memory.
//!
//! ## Fidelity, stated plainly
//!
//! A record round-trips the five fields [`MemoryCore`](tinycortex_api::provider::MemoryCore)
//! owns — `key`, `content`, `category`, `session_id`, `taint` — plus its
//! namespace and export-time timestamp. Document-tier attributes (`title`,
//! `tags`, `metadata`, `source_type`, `priority`) belong to the
//! [`Documents`](tinycortex_api::capabilities::Capability::Documents) family and
//! are out of scope here; a re-import synthesises them the same way a normal
//! store does (`title = key`, `source_type = "chat"`). That is a real
//! limitation of the mandatory-only export, not an oversight — it widens when
//! the Documents family lands.
//!
//! ## Cursor
//!
//! `"{namespace_index}:{offset}"`, indexing into `namespace_summaries()`, whose
//! SQL is `ORDER BY namespace` and therefore stable. `None` starts at `"0:0"`.
//! A cursor that does not parse, or whose index is not a namespace this driver
//! holds, is [`MemoryError::Invalid`] — the contract names exactly that case.
//! Note an empty page is **not** a terminator: only a `None` next-cursor is, so
//! an empty namespace advances the index and keeps paging.

use async_trait::async_trait;
use serde_json::json;
use tinycortex_api::error::MemoryError;
use tinycortex_api::provider::types::{ExportPage, ExportRecord, ImportOutcome};
use tinycortex_api::provider::MemoryPortability;
use tinycortex_api::types::{MemoryCategory, MemoryEntry, GLOBAL_NAMESPACE};

use super::{engine_error, EmbeddedMemoryProvider};

/// The [`ExportRecord::kind`] this driver emits and accepts.
pub(super) const ENTRY_KIND: &str = "entry";

/// Parses `"{index}:{offset}"`. `None` means "start".
fn parse_cursor(cursor: Option<&str>) -> Result<(usize, usize), MemoryError> {
    let Some(raw) = cursor else {
        return Ok((0, 0));
    };
    let invalid =
        || MemoryError::Invalid(format!("export cursor not issued by this driver: {raw}"));
    let (index, offset) = raw.split_once(':').ok_or_else(invalid)?;
    Ok((
        index.parse().map_err(|_| invalid())?,
        offset.parse().map_err(|_| invalid())?,
    ))
}

fn to_record(entry: MemoryEntry) -> ExportRecord {
    ExportRecord {
        kind: ENTRY_KIND.to_string(),
        id: entry.id,
        namespace: entry.namespace,
        taint: entry.taint,
        payload: json!({
            "key": entry.key,
            "content": entry.content,
            "category": entry.category.to_string(),
            "session_id": entry.session_id,
            "timestamp": entry.timestamp,
        }),
    }
}

/// What `import_records` needs out of one record's payload.
struct ImportedEntry {
    namespace: String,
    key: String,
    content: String,
    category: MemoryCategory,
    session_id: Option<String>,
}

/// Reads a record into the fields the engine's store needs.
///
/// # Errors
///
/// An operator-facing reason with **no record content in it** — these strings
/// land in [`ImportOutcome::errors`], which is logged.
fn read_record(record: &ExportRecord) -> Result<ImportedEntry, String> {
    if record.kind != ENTRY_KIND {
        return Err(format!(
            "record {}: unsupported kind '{}' (this driver exports '{ENTRY_KIND}')",
            record.id, record.kind
        ));
    }
    let key = record
        .payload
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("record {}: payload is missing a string 'key'", record.id))?;
    let content = record
        .payload
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "record {}: payload is missing a string 'content'",
                record.id
            )
        })?;
    let raw_category = record
        .payload
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "record {}: payload is missing a string 'category'",
                record.id
            )
        })?;
    let category: MemoryCategory = raw_category
        .parse()
        .map_err(|_| format!("record {}: category is not a known category", record.id))?;

    Ok(ImportedEntry {
        namespace: record
            .namespace
            .clone()
            .unwrap_or_else(|| GLOBAL_NAMESPACE.to_string()),
        key: key.to_string(),
        content: content.to_string(),
        category,
        session_id: record
            .payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

#[async_trait]
impl MemoryPortability for EmbeddedMemoryProvider {
    async fn export_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<ExportPage, MemoryError> {
        let (index, offset) = parse_cursor(cursor)?;
        let memory = self.memory().await?;
        let summaries = memory.namespace_summaries().await.map_err(engine_error)?;

        if index >= summaries.len() {
            // A start-of-export against an empty store lands here legitimately;
            // any other out-of-range index came from a cursor we did not issue.
            if cursor.is_some() && !summaries.is_empty() {
                return Err(MemoryError::Invalid(format!(
                    "export cursor names namespace #{index}, but this driver holds {}",
                    summaries.len()
                )));
            }
            return Ok(ExportPage {
                records: Vec::new(),
                next_cursor: None,
            });
        }

        let namespace = &summaries[index].namespace;
        let entries = memory
            .list(Some(namespace), None, None)
            .await
            .map_err(engine_error)?;
        if offset > entries.len() {
            return Err(MemoryError::Invalid(format!(
                "export cursor offset {offset} is past the end of namespace #{index}"
            )));
        }

        let end = offset.saturating_add(limit).min(entries.len());
        let records: Vec<ExportRecord> = entries[offset..end]
            .iter()
            .cloned()
            .map(to_record)
            .collect();

        let next_cursor = if end < entries.len() {
            Some(format!("{index}:{end}"))
        } else if index + 1 < summaries.len() {
            Some(format!("{}:0", index + 1))
        } else {
            None
        };

        log::debug!(
            "[memory:driver:embedded] export_page index={index} offset={offset} \
             emitted={} more={}",
            records.len(),
            next_cursor.is_some()
        );
        Ok(ExportPage {
            records,
            next_cursor,
        })
    }

    async fn import_records(
        &self,
        records: Vec<ExportRecord>,
    ) -> Result<ImportOutcome, MemoryError> {
        let memory = self.memory().await?;
        let mut outcome = ImportOutcome::default();

        for record in records {
            let entry = match read_record(&record) {
                Ok(entry) => entry,
                Err(reason) => {
                    // Per-record rejection is reported, never fatal: a
                    // million-record restore must not abort on one bad row.
                    outcome.failed = outcome.failed.saturating_add(1);
                    outcome.errors.push(reason);
                    continue;
                }
            };

            // `store_with_taint` with the record's *own* taint: an importing
            // driver must persist what it is given and must not re-stamp
            // provenance. `Memory::store` would stamp `Internal`.
            memory
                .store_with_taint(
                    &entry.namespace,
                    &entry.key,
                    &entry.content,
                    entry.category,
                    entry.session_id.as_deref(),
                    record.taint,
                )
                .await
                .map_err(engine_error)?;
            outcome.imported = outcome.imported.saturating_add(1);
        }

        log::debug!(
            "[memory:driver:embedded] import_records imported={} failed={}",
            outcome.imported,
            outcome.failed
        );
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "portability_tests.rs"]
mod tests;
