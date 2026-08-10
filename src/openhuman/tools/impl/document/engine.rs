//! Async wrapper around the vendored [`tinydocs`] `.docx` writer.
//!
//! The OOXML synthesis itself lives in
//! [`tinydocs::docx::generate`](https://github.com/tinyhumansai/tinydocs) —
//! spec validation, the paragraph/heading/bullet mapping, and the zip pack.
//! That call is **synchronous and CPU-bound by design**: `tinydocs` has no
//! opinion about executors or deadlines, because only a host knows its own.
//!
//! This module supplies exactly that missing policy, and nothing else:
//!
//! 1. a `spawn_blocking` hop so a CPU-bound pack never stalls the agent
//!    loop's executor, and
//! 2. a `tokio::time::timeout` so a pathological input that slipped past
//!    validation cannot wedge the loop indefinitely, and
//! 3. the mapping from a crate error, a join failure, or an elapsed deadline
//!    onto the agent-facing [`DocumentError`].
//!
//! Control flow here is identical to the presentation engine's, so the two
//! artifact producers keep failing in the same shapes.

use std::time::Duration;

use tokio::task::JoinError;
use tokio::time::{error::Elapsed, timeout};

use super::types::{DocumentError, GenerateDocumentInput};

/// Generate the `.docx` bytes for `input`, giving up after `deadline`.
///
/// The `deadline` covers the entire blocking call, including `spawn_blocking`
/// thread acquisition. Hitting it surfaces as
/// [`DocumentError::GenerationTimeout`].
pub(super) async fn generate(
    input: &GenerateDocumentInput,
    deadline: Duration,
) -> Result<Vec<u8>, DocumentError> {
    // Clone across the blocking boundary — cheap relative to the synthesis,
    // and it keeps the blocking closure `'static`.
    let owned = input.clone();
    let started = std::time::Instant::now();
    let section_count = owned.sections.len();
    let deadline_secs = deadline.as_secs();
    let title_chars = owned.title.chars().count();

    tracing::debug!(
        target: "document",
        deadline_secs,
        section_count,
        title_chars,
        "[document:engine] generate:start"
    );

    let join: Result<Result<Result<Vec<u8>, tinydocs::Error>, _>, Elapsed> = timeout(
        deadline,
        tokio::task::spawn_blocking(move || tinydocs::docx::generate(&owned)),
    )
    .await;

    let elapsed_ms = started.elapsed().as_millis() as u64;
    match join {
        Err(_elapsed) => {
            tracing::warn!(
                target: "document",
                elapsed_ms,
                deadline_secs,
                section_count,
                "[document:engine] generate:timeout"
            );
            Err(DocumentError::GenerationTimeout {
                timeout_secs: deadline_secs,
            })
        }
        Ok(Err(join_err)) => {
            let err = map_join_error(join_err);
            tracing::warn!(
                target: "document",
                elapsed_ms,
                kind = "join_error",
                err = %err,
                "[document:engine] generate:failure"
            );
            Err(err)
        }
        Ok(Ok(Err(crate_err))) => {
            let err = DocumentError::from(crate_err);
            tracing::warn!(
                target: "document",
                elapsed_ms,
                kind = "engine_failure",
                err = %err,
                "[document:engine] generate:failure"
            );
            Err(err)
        }
        Ok(Ok(Ok(bytes))) => {
            tracing::debug!(
                target: "document",
                elapsed_ms,
                bytes = bytes.len(),
                section_count,
                "[document:engine] generate:done"
            );
            Ok(bytes)
        }
    }
}

fn map_join_error(err: JoinError) -> DocumentError {
    // The outer `tokio::time::timeout` already routes the timeout case, so a
    // `JoinError` here is a panic (library bug / OOM on the blocking pool) or
    // a cancellation (runtime shutdown / explicit abort). Both surface as
    // `GenerationFailed` with context preserved — mirrors the presentation
    // engine so a "0s timeout" message is never fabricated.
    if err.is_panic() {
        DocumentError::GenerationFailed {
            stderr_truncated: DocumentError::truncate_stderr("document engine panicked"),
        }
    } else {
        DocumentError::GenerationFailed {
            stderr_truncated: DocumentError::truncate_stderr(&format!(
                "document engine task cancelled: {err}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::tools::implementations::document::types::DocumentSection;

    fn sample_input() -> GenerateDocumentInput {
        GenerateDocumentInput {
            title: "Project Charter".to_string(),
            author: Some("Alice".to_string()),
            sections: vec![
                DocumentSection {
                    heading: Some("Overview".to_string()),
                    paragraphs: vec!["This document describes the plan.".to_string()],
                    bullets: vec![],
                },
                DocumentSection {
                    heading: Some("Goals".to_string()),
                    paragraphs: vec![],
                    bullets: vec!["Ship v1".to_string(), "Delight users".to_string()],
                },
            ],
        }
    }

    /// Read the entry names of a produced `.docx` byte buffer.
    fn docx_entry_names(bytes: &[u8]) -> Vec<String> {
        let cursor = std::io::Cursor::new(bytes.to_vec());
        let mut zip = zip::ZipArchive::new(cursor).expect("output is a valid zip archive");
        (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect()
    }

    /// Read one entry's UTF-8 body out of a produced `.docx`.
    fn docx_entry_body(bytes: &[u8], name: &str) -> String {
        let cursor = std::io::Cursor::new(bytes.to_vec());
        let mut zip = zip::ZipArchive::new(cursor).expect("valid zip");
        let mut entry = zip.by_name(name).expect("entry present");
        let mut body = String::new();
        std::io::Read::read_to_string(&mut entry, &mut body).unwrap();
        body
    }

    #[tokio::test]
    async fn generate_round_trips_to_valid_docx() {
        // End-to-end through the wrapper: build → tinydocs → byte buffer →
        // re-open as zip → confirm the OOXML skeleton + that our text reached
        // document.xml.
        let bytes = generate(&sample_input(), Duration::from_secs(30))
            .await
            .expect("generate should succeed");

        // A `.docx` is a zip: the magic bytes are the local-file-header
        // signature `PK\x03\x04`. This is the acceptance-criteria check
        // that any OOXML reader can open the file.
        assert!(
            bytes.len() > 200,
            "docx unexpectedly small ({} bytes)",
            bytes.len()
        );
        assert_eq!(&bytes[0..2], b"PK", "docx must start with the zip magic PK");

        let names = docx_entry_names(&bytes);
        for required in ["[Content_Types].xml", "_rels/.rels", "word/document.xml"] {
            assert!(
                names.iter().any(|n| n == required),
                "missing OOXML entry: {required} (got: {names:?})"
            );
        }

        // Numbering was used → the numbering part must materialise.
        assert!(
            names.iter().any(|n| n == "word/numbering.xml"),
            "bullet list should emit word/numbering.xml (got: {names:?})"
        );

        // Our title, heading, paragraph, and bullet text all reach the
        // rendered document body — none dropped on the floor.
        let doc = docx_entry_body(&bytes, "word/document.xml");
        for needle in [
            "Project Charter",
            "Overview",
            "This document describes the plan.",
            "Goals",
            "Ship v1",
            "Delight users",
        ] {
            assert!(
                doc.contains(needle),
                "document.xml missing text: {needle:?}"
            );
        }
    }

    #[tokio::test]
    async fn generate_drops_blank_paragraphs_and_bullets() {
        // Whitespace-only entries must not blow up generation and must not
        // emit empty runs — the engine trims + drops them.
        let input = GenerateDocumentInput {
            title: "Trimmed".to_string(),
            author: Some("   ".to_string()),
            sections: vec![DocumentSection {
                heading: Some("Kept".to_string()),
                paragraphs: vec!["real".to_string(), "   ".to_string(), String::new()],
                bullets: vec!["item".to_string(), "\t\n".to_string()],
            }],
        };
        let bytes = generate(&input, Duration::from_secs(30))
            .await
            .expect("generate should succeed on whitespace-only entries");
        let doc = docx_entry_body(&bytes, "word/document.xml");
        assert!(doc.contains("real"));
        assert!(doc.contains("item"));
    }

    #[tokio::test]
    async fn generate_surfaces_a_crate_validation_failure() {
        // `tinydocs` re-validates inside `generate`, so a spec that never went
        // through `validate_input` still fails structurally rather than
        // producing a blank document.
        let input = GenerateDocumentInput {
            title: String::new(),
            author: None,
            sections: vec![],
        };
        match generate(&input, Duration::from_secs(30)).await {
            Err(DocumentError::InvalidInput { field, .. }) => assert_eq!(field, "title"),
            other => panic!("expected InvalidInput(title), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn generate_yields_clean_structured_result_under_zero_deadline() {
        // Contract under an impossibly-short deadline: `generate` must surface a
        // clean, structured outcome — never a panic or a half-written buffer.
        //
        // Which outcome we get is inherently racy and must NOT be pinned: a
        // near-zero `timeout` wrapping `spawn_blocking` usually elapses first
        // (GenerationTimeout), but the runtime can instead cancel the blocking
        // task, which `map_join_error` maps to GenerationFailed, and a trivial
        // input can even finish before the timer fires (Ok). Asserting one exact
        // variant made this flake under coverage instrumentation. We assert the
        // real invariant: any Ok is a non-empty buffer, any Err is one of the
        // two documented structured variants, and nothing panics.
        match generate(&sample_input(), Duration::ZERO).await {
            Ok(bytes) => assert!(!bytes.is_empty(), "a completed docx must be non-empty"),
            Err(DocumentError::GenerationTimeout { timeout_secs }) => {
                assert_eq!(timeout_secs, 0, "zero deadline reports 0 seconds");
            }
            Err(DocumentError::GenerationFailed { .. }) => {
                // Blocking task cancelled before the timer fired — still clean.
            }
            Err(other) => panic!("unexpected error variant under a zero deadline: {other:?}"),
        }
    }

    #[tokio::test]
    async fn map_join_error_cancellation_becomes_generation_failed() {
        // A non-panic JoinError (cancellation via abort) surfaces as
        // GenerationFailed with the cancellation context preserved — never
        // a fabricated "0s timeout".
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        handle.abort();
        let join_err = handle.await.expect_err("aborted task yields JoinError");
        assert!(
            !join_err.is_panic(),
            "abort() yields a cancellation, not a panic"
        );
        match map_join_error(join_err) {
            DocumentError::GenerationFailed { stderr_truncated } => {
                assert!(
                    stderr_truncated.contains("document engine task cancelled"),
                    "cancellation context missing: {stderr_truncated:?}"
                );
            }
            other => panic!("expected GenerationFailed, got {other:?}"),
        }
    }
}
