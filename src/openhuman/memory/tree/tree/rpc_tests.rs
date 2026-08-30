use super::*;
use chrono::Utc;
use serde_json::json;
use tempfile::TempDir;
use tinycortex::memory::ingest::canonicalize::document::DocumentInput;
use tinymemory_api::chunks::SourceKind;

fn test_config() -> (TempDir, Config) {
    let tmp = TempDir::new().unwrap();
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    cfg.memory_tree.embedding_endpoint = None;
    cfg.memory_tree.embedding_model = None;
    cfg.memory_tree.embedding_strict = false;
    (tmp, cfg)
}

/// Bind a driver reporting fixed diagnostics as `cfg`'s memory driver.
///
/// See `binding::FixedDiagnostics` for why the status handlers need one:
/// they read through the contract now, and the real driver is a compiled
/// module that a unit test cannot load.
fn bind_diagnostics(
    cfg: &Config,
    store: crate::openhuman::memory::api::provider::types::StoreStats,
    queue: crate::openhuman::memory::api::provider::types::QueueStats,
) {
    crate::openhuman::memory::binding::install_diagnostics_for_test(
        &cfg.workspace_dir,
        &cfg.subsystems.memory,
        store,
        queue,
    );
}

/// #5169 (`CORE-RUST-1P0`) — a chat batch whose messages omit `timestamp`
/// must ingest, defaulting to `now()`, not reject the whole batch.
///
/// The tolerance lives in `tinycortex` (`ChatMessage::timestamp` carries
/// `#[serde(default = "chrono_now")]`), which is a **separate repository**
/// vendored here as a submodule. Nothing in this repo guarded that
/// contract, so a submodule bump could silently reintroduce the hard
/// rejection and the 4xx-shaped payload would page again. This test is
/// that guard: it fails on the parent-repo side the moment the vendored
/// schema stops tolerating an absent timestamp.
#[test]
fn chat_payload_without_timestamp_is_accepted() {
    let payload = json!({
        "platform": "slack",
        "channel_label": "#general",
        "messages": [{ "author": "alice", "text": "no timestamp here" }],
    });

    let batch: ChatBatch = serde_json::from_value(payload)
        .expect("a chat message omitting `timestamp` must default, not reject the batch");

    assert_eq!(batch.messages.len(), 1);
    assert_eq!(batch.messages[0].text, "no timestamp here");
}

/// Sibling contract for the document arm: `modified_at` is likewise
/// optional (`#[serde(default = "now_utc")]` in tinycortex).
///
/// The payload is deliberately minimal — `title` and `body` are the only
/// required fields on `DocumentInput`. `provider` (`default_provider`),
/// `source_ref` (`Option`) and `modified_at` (`now_utc`) all carry serde
/// defaults, so omitting them together pins the whole optional set rather
/// than just the timestamp.
#[test]
fn document_payload_without_modified_at_is_accepted() {
    let payload = json!({ "title": "Launch plan", "body": "ship it" });

    let doc: DocumentInput = serde_json::from_value(payload)
        .expect("a document omitting `modified_at` must default, not reject");

    assert_eq!(doc.title, "Launch plan");
}

/// Every `SourceKind` reachable from `ingest_rpc` must produce a message
/// the classifier recognises — otherwise that arm's caller errors keep
/// paging while its siblings are demoted, which is the silent-drift
/// failure the enumerated list in `is_invalid_ingest_payload_message`
/// is meant to make impossible to miss.
#[test]
fn all_source_kinds_are_recognised_as_caller_payload_errors() {
    let err = serde_json::from_str::<ChatBatch>("{}").unwrap_err();
    for kind in [SourceKind::Chat, SourceKind::Email, SourceKind::Document] {
        let message = invalid_payload_message(kind, &err);
        assert!(
            is_invalid_ingest_payload_message(&message),
            "{} payload errors must classify as caller errors, got {message:?}",
            kind.as_str()
        );
    }
}

/// The verbatim #5169 message shape, and the negative half: unrelated
/// failures must keep their error severity so real defects still page.
#[test]
fn only_ingest_payload_errors_are_demoted() {
    assert!(is_invalid_ingest_payload_message(
        "invalid chat payload: missing field `timestamp`"
    ));

    for other in [
        "invalid",
        "invalid payload",
        "invalid audio payload: missing field `timestamp`",
        "ingest: chunk store unavailable",
        "chat payload: missing field `timestamp`",
        "something failed: invalid chat payload: missing field `timestamp`",
        "",
    ] {
        assert!(
            !is_invalid_ingest_payload_message(other),
            "{other:?} must keep paging"
        );
    }
}

/// The ingest response is this crate's declaration of a wire the frontend
/// reads, so nothing upstream keeps its keys honest any more.
///
/// It used to be asserted against the engine's own `IngestResult`, on the
/// reasoning that comparing to the upstream type beat hand-writing a key
/// list. That held while the engine produced the body. It does not now:
/// every arm builds from the contract's `IngestOutcome`, so a comparison
/// against the engine summary would pin a shape nothing in this path
/// produces — and it kept the engine linked here purely to describe a wire
/// this crate owns.
///
/// So the expectation is written out. That is the honest form once this
/// crate is the declaring side: the keys below are what the frontend
/// parses, and renaming a field on `IngestResponse` fails here rather than
/// reaching a reader.
#[test]
fn the_response_body_serialises_exactly_as_the_declared_wire() {
    let ours = IngestResponse {
        source_id: "doc-launch".into(),
        chunks_written: 3,
        chunks_dropped: 1,
        chunk_ids: vec!["chunk-a".into(), "chunk-b".into(), "chunk-c".into()],
        extract_jobs_enqueued: 2,
        already_ingested: true,
    };

    assert_eq!(
        serde_json::to_value(&ours).unwrap(),
        serde_json::json!({
            "source_id": "doc-launch",
            "chunks_written": 3,
            "chunks_dropped": 1,
            "chunk_ids": ["chunk-a", "chunk-b", "chunk-c"],
            "extract_jobs_enqueued": 2,
            "already_ingested": true,
        }),
        "the ingest response wire moved — the frontend reads these names"
    );
}

fn sample_document(title: &str, body: &str) -> DocumentInput {
    DocumentInput {
        provider: "notion".into(),
        title: title.into(),
        body: body.into(),
        modified_at: Utc::now(),
        source_ref: Some("notion://page/launch".into()),
    }
}

/// Ingest reports what it wrote.
///
/// Bound to the in-process TinyCortex driver rather than left to resolve on
/// its own: the handler asks the driver for the `Ingest` family now, and
/// what a bare test workspace binds is the null driver, which serves none.
/// This is the engine the loadable module wraps, so the counts asserted
/// below are the ones production gets over the bus.
#[tokio::test]
async fn ingest_document_reports_the_chunks_it_wrote() {
    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);
    let outcome = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Document,
            source_id: "doc-launch".into(),
            owner: "alice".into(),
            tags: vec!["launch".into()],
            payload: serde_json::to_value(sample_document(
                "Launch Plan",
                "Phoenix launch canary checklist with rollback steps.",
            ))
            .unwrap(),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome.value.source_id, "doc-launch");
    assert_eq!(outcome.value.chunks_dropped, 0);
    assert!(outcome.value.chunks_written > 0);
    assert!(
        !outcome.value.chunk_ids.is_empty(),
        "the ids are what a caller fetches a chunk back by, so a write \
         that names none is unusable even when the count is right"
    );
}

/// The listing degrades rather than fails when the bound driver has no
/// chunk tier.
///
/// `FixedDiagnostics` is `NullMemoryProvider`-backed, so `as_chunks()` is
/// `None` — the shape of a driver that serves memory without exposing the
/// engine's storage model. The handler is read-only, and an empty page is a
/// true statement about such a driver, so it must not become a
/// caller-facing error. The log still has to report the count it served,
/// because a silent empty and a degraded empty look identical downstream.
#[tokio::test]
async fn list_chunks_reports_empty_when_the_driver_has_no_chunk_tier() {
    let (_tmp, cfg) = test_config();
    bind_diagnostics(&cfg, Default::default(), Default::default());

    let listed = list_chunks_rpc(
        &cfg,
        ListChunksRequest {
            source_kind: Some("document".into()),
            source_id: Some("doc-launch".into()),
            limit: Some(10),
            ..Default::default()
        },
    )
    .await
    .expect("a driver without the chunk family is not an error");
    assert!(listed.value.chunks.is_empty());
    assert!(listed.logs[0].contains("n=0"), "log: {}", listed.logs[0]);
}

/// The source gate is the driver's, and it survives the move onto the
/// contract: `IngestOutcome::already_ingested` is the field the v1.3.0 pin
/// did not have, and reporting a refused call as a plain empty write is
/// exactly what this test would have started passing over.
#[tokio::test]
async fn ingest_document_is_idempotent_for_duplicate_source_id() {
    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);
    let req = IngestRequest {
        source_kind: SourceKind::Document,
        source_id: "doc-dup".into(),
        owner: "alice".into(),
        tags: vec![],
        payload: serde_json::to_value(sample_document("Launch Plan", "First body")).unwrap(),
    };

    let first = ingest_rpc(&cfg, req.clone()).await.unwrap().value;
    let second = ingest_rpc(&cfg, req).await.unwrap().value;
    assert!(first.chunks_written > 0);
    assert!(!first.already_ingested);
    // `already_ingested` with a zero write count is the whole claim:
    // documents are append-only, so a repeat submission must be recognised
    // rather than duplicated — and told apart from a write that produced
    // nothing, which is the same two numbers with a different cause.
    assert_eq!(second.chunks_written, 0);
    assert!(second.already_ingested);
    assert_eq!(second.source_id, first.source_id);
}

/// Regression #3568 / CORE-2K: chat payloads with RFC-3339 timestamps must
/// be accepted — not rejected with "expected unix timestamp in milliseconds".
#[tokio::test]
async fn ingest_chat_accepts_rfc3339_timestamps() {
    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);
    let outcome = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Chat,
            source_id: "slack:#rfc3339-test".into(),
            owner: "alice".into(),
            tags: vec![],
            payload: json!({
                "platform": "slack",
                "channel_label": "#eng",
                "messages": [
                    {
                        "author": "alice",
                        "timestamp": "2026-05-17T19:30:00Z",
                        "text": "planning the launch"
                    },
                    {
                        "author": "bob",
                        "timestamp": 1779046260000_i64,
                        "text": "confirmed"
                    }
                ]
            }),
        },
    )
    .await
    .unwrap();
    assert!(!outcome.value.chunk_ids.is_empty());
}

/// Regression #3568 / CORE-2K: email payloads with RFC-3339 timestamps must
/// be accepted.
///
/// A driver is bound, like every sibling here. The note this replaces said
/// the mail arm was "still on the in-process pipeline" and that the test
/// would need `install_tinycortex_for_test` "when it moves" — it has moved:
/// the `Email` arm now goes through `ingest_through_driver`, which resolves
/// `provider().as_ingest()` and refuses a driver that does not serve it.
/// Without the binding the test only passed because CI happens to set
/// `TINYMEMORY_TEST_MODULE` to a module that serves `Ingest`, so it would
/// fail on a machine that does not.
#[tokio::test]
async fn ingest_email_accepts_rfc3339_timestamps() {
    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);
    let outcome = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Email,
            source_id: "gmail:rfc3339-test".into(),
            owner: "alice@example.com".into(),
            tags: vec![],
            payload: json!({
                "provider": "gmail",
                "thread_subject": "Launch",
                "messages": [
                    {
                        "from": "bob@example.com",
                        "to": ["alice@example.com"],
                        "subject": "Launch",
                        "sent_at": "2026-05-17T19:30:00Z",
                        "body": "Let's ship this."
                    }
                ]
            }),
        },
    )
    .await
    .unwrap();
    assert!(!outcome.value.chunk_ids.is_empty());
}

/// One empty message must not fail the batch around it.
///
/// `validate_ingest_item` answers `Invalid` for content that trims to
/// empty, and the driver validates every item before ingesting any — so an
/// attachment-only message, which reaches this handler as a message with no
/// text, would turn a batch that has real content in it into a failed call.
/// The in-process pipeline wrote the rest of the batch and rendered that
/// message as a bare header; the filter keeps the first half of that and
/// gives up only the header.
#[tokio::test]
async fn an_empty_chat_message_does_not_fail_the_batch_around_it() {
    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::test_support::install_tinycortex_for_test(&cfg);
    let outcome = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Chat,
            source_id: "slack:#attachment-only".into(),
            owner: "alice".into(),
            tags: vec![],
            payload: json!({
                "platform": "slack",
                "channel_label": "#eng",
                "messages": [
                    {
                        "author": "alice",
                        "timestamp": "2026-05-17T19:30:00Z",
                        "text": "   "
                    },
                    {
                        "author": "bob",
                        "timestamp": "2026-05-17T19:31:00Z",
                        "text": "here is the plan"
                    }
                ]
            }),
        },
    )
    .await
    .expect("an empty message is dropped, not a batch failure");
    assert!(
        !outcome.value.chunk_ids.is_empty(),
        "the surviving message must still be written"
    );
}

/// An ingest is a write, so a driver without the family is refused rather
/// than answered with zeros.
///
/// The counts have no way to say "nothing was handed over": zero written
/// and zero dropped is what a successful ingest of nothing looks like too,
/// so degrading here would report content dropped on the floor as a
/// success. `FixedDiagnostics` advertises `Capabilities::all()` while
/// serving no `Ingest` accessor, which also pins that the refusal keys off
/// the accessor and not off the advertised set.
#[tokio::test]
async fn ingest_refuses_a_driver_that_does_not_serve_the_ingest_family() {
    let (_tmp, cfg) = test_config();
    bind_diagnostics(&cfg, Default::default(), Default::default());

    let err = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Chat,
            source_id: "slack:#no-ingest".into(),
            owner: "alice".into(),
            tags: vec![],
            payload: json!({
                "platform": "slack",
                "channel_label": "#eng",
                "messages": [{ "author": "alice", "text": "anything at all" }],
            }),
        },
    )
    .await
    .unwrap_err();
    assert!(
        err.contains("does not serve Ingest"),
        "the refusal must name the missing family: {err}"
    );
    assert!(
        err.contains("fixed-diagnostics"),
        "the refusal must name the driver that refused: {err}"
    );
}

#[tokio::test]
async fn ingest_rpc_rejects_invalid_document_payload() {
    let (_tmp, cfg) = test_config();
    let err = ingest_rpc(
        &cfg,
        IngestRequest {
            source_kind: SourceKind::Document,
            source_id: "doc-invalid".into(),
            owner: String::new(),
            tags: vec![],
            payload: json!({"title": "Missing body"}),
        },
    )
    .await
    .unwrap_err();
    assert!(err.contains("invalid document payload"));
}

#[tokio::test]
async fn list_chunks_rejects_unknown_source_kind() {
    let (_tmp, cfg) = test_config();
    let err = list_chunks_rpc(
        &cfg,
        ListChunksRequest {
            source_kind: Some("nonsense".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert!(err.contains("unknown source kind: nonsense"));
}

/// An id the driver cannot resolve is `Ok(None)`, never an error — and the
/// same is true of a driver with no chunk tier at all, which is why one
/// test covers both. The two cases are indistinguishable to a caller by
/// design: "no such chunk" is the honest answer to either.
#[tokio::test]
async fn get_chunk_returns_none_for_missing_id() {
    let (_tmp, cfg) = test_config();
    bind_diagnostics(&cfg, Default::default(), Default::default());
    let outcome = get_chunk_rpc(
        &cfg,
        GetChunkRequest {
            id: "missing-chunk".into(),
        },
    )
    .await
    .unwrap();
    assert!(outcome.value.chunk.is_none());
}

/// #1574 §4b: `backfill_status_rpc` reports what the driver says is
/// queued for the backfill kind, and a non-zero count forces
/// `in_progress` so the modal stays open.
///
/// The empty case now asserts `in_progress` too. It could not before: the
/// flag was a process-global that parallel tests shared. It comes from the
/// bound driver now, so it is this test's to set.
///
/// Ready + running, and deliberately not `total - done`: a backfill job
/// that failed is finished with, and counting it as pending would leave
/// the modal open forever.
#[tokio::test]
async fn backfill_status_reports_the_drivers_pending_count() {
    use crate::openhuman::memory::api::provider::types::QueueStats;

    let (_tmp, cfg) = test_config();

    bind_diagnostics(&cfg, Default::default(), QueueStats::default());
    let s0 = backfill_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(s0.pending_jobs, 0, "idle space has no pending backfill");

    bind_diagnostics(
        &cfg,
        Default::default(),
        QueueStats {
            ready: 1,
            running: 2,
            // Neither of these is pending work.
            done: 7,
            failed: 3,
            ..Default::default()
        },
    );
    let s1 = backfill_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(
        s1.pending_jobs, 3,
        "ready + running is what is still to do; done and failed are not"
    );
    assert!(s1.in_progress, "pending>0 forces in_progress=true");
}

/// The backfill flag is the driver's answer, not the host's engine static.
///
/// This is the gap the counts cannot express: a backfill chain re-enqueues
/// itself, so between one link settling and the next being written there is
/// an instant with nothing ready, nothing running, and the work unfinished.
/// A poll that trusted the counts alone closes the re-embed modal there.
///
/// It has to come from the driver rather than
/// `tinymemory_core::queue::backfill_in_progress()`, because re-embedding
/// runs in the module and a `cdylib` has its own statics — the host-linked
/// copy reads `false` forever on that path, which is worse than coarse.
#[tokio::test]
async fn backfill_status_reports_the_drivers_flag_when_the_counts_are_empty() {
    use crate::openhuman::memory::api::provider::types::QueueStats;

    let (_tmp, cfg) = test_config();

    let driver = std::sync::Arc::new(
        crate::openhuman::memory::binding::FixedDiagnostics::new(
            Default::default(),
            QueueStats::default(),
        )
        .backfilling(),
    );
    crate::openhuman::memory::binding::install_for_test(
        &cfg.workspace_dir,
        &cfg.subsystems.memory,
        driver as std::sync::Arc<dyn crate::openhuman::memory::api::provider::MemoryProvider>,
    );

    let status = backfill_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(
        status.pending_jobs, 0,
        "precondition: the counts say the queue is empty"
    );
    assert!(
        status.in_progress,
        "and the driver still says a backfill is running, which is the whole point"
    );
}

// ── pipeline_status / set_enabled (#1856 Part 1) ─────────────────────

/// `derive_pipeline_status` precedence is locked in here so the UI can
/// rely on the wire status string without re-deriving it from the raw
/// counters.
#[test]
fn latest_quarantine_reads_the_newest_copy_and_derives_resynced() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("memory_tree");
    std::fs::create_dir_all(&dir).unwrap();
    // No quarantine file: nothing to report.
    assert!(latest_quarantine(tmp.path(), 0).is_none());

    std::fs::write(dir.join("chunks.db.corrupt-20260101T000000Z"), b"old").unwrap();
    std::fs::write(dir.join("chunks.db.corrupt-20260827T070304Z"), b"new").unwrap();
    // Side files never match the main-file prefix.
    std::fs::write(dir.join("chunks.db-wal.corrupt-20261231T235959Z"), b"wal").unwrap();
    // Garbage that starts with the prefix but has no parsable stamp is ignored.
    std::fs::write(dir.join("chunks.db.corrupt-notastamp"), b"x").unwrap();

    let at = chrono::NaiveDate::from_ymd_opt(2026, 8, 27)
        .unwrap()
        .and_hms_opt(7, 3, 4)
        .unwrap()
        .and_utc()
        .timestamp_millis();

    // The rebuilt store is still empty: the notice stands.
    let pending = latest_quarantine(tmp.path(), 0).expect("newest quarantine");
    assert_eq!(pending.quarantined_at_ms, at);
    assert!(pending
        .quarantined_path
        .ends_with("chunks.db.corrupt-20260827T070304Z"));
    assert!(!pending.resynced);

    // Any chunk in the rebuilt store: the user re-synced, the notice retires.
    // Chunk *content* time is irrelevant here: restored history predates the
    // quarantine forever, which is exactly why this is not a timestamp test.
    let done = latest_quarantine(tmp.path(), 1).expect("newest quarantine");
    assert!(done.resynced);
}

#[test]
fn derive_pipeline_status_precedence_matches_spec() {
    use crate::openhuman::memory::tree::health::{DegradedState, FailureCode, PipelineFailure};
    use tinymemory_api::host::SchedulerGateMode;

    let healthy = DegradedState::default();
    let recall_degraded = DegradedState {
        semantic_recall: true,
        structure: false,
        storage: false,
        cause: Some(PipelineFailure::new(FailureCode::EmbeddingsUnconfigured)),
    };
    let structure_degraded = DegradedState {
        semantic_recall: false,
        structure: true,
        storage: false,
        cause: Some(PipelineFailure::new(FailureCode::ExtractionTimeout)),
    };
    let storage_degraded = DegradedState {
        semantic_recall: false,
        structure: false,
        storage: true,
        cause: Some(PipelineFailure::new(FailureCode::StorageUnavailable)),
    };

    // Args: (is_paused, mode, is_syncing, failed, failed_unrecoverable,
    //        total_chunks, &degraded, queue_idle_ms).

    // paused beats everything else (even degradation)
    let (s, reason) = derive_pipeline_status(
        true,
        SchedulerGateMode::Off,
        true,
        5,
        5,
        100,
        &recall_degraded,
        None,
    );
    assert_eq!(s, "paused");
    assert!(reason.unwrap().contains("off"));

    // paused still beats a storage failure (user explicitly stood the
    // worker down; the flag won't be freshly set anyway).
    let (s, _) = derive_pipeline_status(
        true,
        SchedulerGateMode::Off,
        false,
        0,
        0,
        0,
        &storage_degraded,
        None,
    );
    assert_eq!(s, "paused", "paused beats storage");

    // storage failure → error, and it fires even with ZERO chunks (unlike
    // recall/structure degradation, which is content-relative) — a dead
    // disk is broken regardless of how much content exists.
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        0, // no chunks — must still surface
        &storage_degraded,
        None,
    );
    assert_eq!(
        s, "error",
        "storage failure is a hard error at any chunk count"
    );
    assert!(reason.unwrap().contains("storage"));

    // storage outranks transient-failed degradation too.
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        true,
        3,
        0,
        100,
        &storage_degraded,
        None,
    );
    assert_eq!(s, "error", "storage beats transient-degraded");

    // error beats degraded / syncing / running / idle — but ONLY for
    // unrecoverable failures (#3365).
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        true,
        2,
        2, // both failures unrecoverable
        100,
        &recall_degraded,
        None,
    );
    assert_eq!(s, "error");
    assert!(reason.unwrap().contains("unrecoverable"));

    // #3365: transient-only failures (failed > 0, none unrecoverable) do NOT
    // escalate to error — they self-heal via auto-requeue, so they surface
    // as `degraded` ("retrying"), beating syncing/running.
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        true,
        3,
        0,
        100,
        &healthy,
        None,
    );
    assert_eq!(s, "degraded", "transient failures must not read as error");
    assert!(reason.unwrap().contains("3 job(s) failed, retrying"));

    // #002: degraded beats syncing / running / idle (but loses to paused/error)
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        true, // syncing
        0,
        0,
        100,
        &recall_degraded,
        None,
    );
    assert_eq!(s, "degraded", "degraded must beat syncing");
    assert!(reason.unwrap().contains("semantic recall disabled"));

    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        100,
        &structure_degraded,
        None,
    );
    assert_eq!(s, "degraded");
    assert!(reason.unwrap().contains("wiki structure incomplete"));

    // syncing beats running / idle (when healthy)
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        true,
        0,
        0,
        100,
        &healthy,
        None,
    );
    assert_eq!(s, "syncing");
    assert!(reason.is_none());

    // running when chunks exist but nothing in flight
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        100,
        &healthy,
        None,
    );
    assert_eq!(s, "running");

    // idle when the store is empty and nothing is in flight (transient
    // failures with no content don't manufacture a `degraded`).
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        2,
        0,
        0,
        &healthy,
        None,
    );
    assert_eq!(s, "idle");
}

/// #5324: a queue that accepts work but never drains it must report
/// `degraded`, not the `running`/`idle` that made a month-long outage look
/// healthy. Pins the threshold boundary and the full precedence chain.
#[test]
fn stalled_queue_degrades_instead_of_reading_healthy() {
    use crate::openhuman::memory::tree::health::{DegradedState, FailureCode, PipelineFailure};
    use tinymemory_api::host::SchedulerGateMode;

    let healthy = DegradedState::default();
    let stalled = Some(QUEUE_STALL_THRESHOLD_MS);
    let just_under = Some(QUEUE_STALL_THRESHOLD_MS - 1);

    // The regression itself: chunks exist, nothing failed, nothing running
    // — previously "running", which is what let the outage hide.
    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        100,
        &healthy,
        stalled,
    );
    assert_eq!(s, "degraded", "a stalled queue must not read as running");
    assert!(reason.unwrap().contains("has not completed any job"));

    // NOT gated on total_chunks: a queue that never drained has no chunks,
    // and that case must not read as `idle`.
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        0,
        &healthy,
        stalled,
    );
    assert_eq!(s, "degraded", "empty-but-stalled must not read as idle");

    // Boundary: one millisecond under the threshold is still healthy, so a
    // merely slow flush window can't trip it.
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        100,
        &healthy,
        just_under,
    );
    assert_eq!(s, "running", "under the threshold stays healthy");

    // `None` (no ready jobs, or an unreadable metric) never manufactures a
    // degraded verdict.
    let (s, _) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        0,
        0,
        100,
        &healthy,
        None,
    );
    assert_eq!(s, "running", "absent metric must not claim a stall");

    // Precedence: paused and error both outrank the stall — a typed
    // unrecoverable failure is the more specific, more actionable answer.
    let (s, _) = derive_pipeline_status(
        true,
        SchedulerGateMode::Off,
        false,
        0,
        0,
        100,
        &healthy,
        stalled,
    );
    assert_eq!(s, "paused", "paused beats stalled");

    let (s, reason) = derive_pipeline_status(
        false,
        SchedulerGateMode::Auto,
        false,
        1,
        1,
        100,
        &healthy,
        stalled,
    );
    assert_eq!(s, "error", "unrecoverable failure beats stalled");
    assert!(reason.unwrap().contains("unrecoverable"));

    // Sanity: the budget-exhausted failure this issue is about is indeed
    // classified unrecoverable, so it lands in the `error` branch above and
    // carries its own remediation key.
    let budget = PipelineFailure::new(FailureCode::BudgetExhausted);
    assert!(budget.is_unrecoverable());
    assert_eq!(
        budget.remediation_key,
        "memory.health.remediation.budget_exhausted"
    );
}

/// #5324: `queue_idle_ms` measures idle time, not backlog depth. Pins the
/// two shapes that must NOT be reported as stalled, both of which a
/// backlog-age metric would have flagged — and both of which describe the
/// heavy users this issue is about.
/// One queue snapshot, spelled out.
///
/// These read as SQL fixtures before the driver owned the query. What
/// `queue_idle_ms` decides has never depended on the rows, only on the
/// three numbers below, so the tests say those directly now. Which rows
/// produce which numbers is the driver's rule and is pinned in the
/// driver's own suite — notably that deferred work counts as `ready`
/// without becoming `eligible_now`, the distinction the third case here
/// relies on.
fn queue(
    eligible_now: u64,
    last_completed_ms: Option<i64>,
    oldest_eligible_ms: Option<i64>,
) -> crate::openhuman::memory::api::provider::types::QueueStats {
    crate::openhuman::memory::api::provider::types::QueueStats {
        eligible_now,
        last_completed_ms,
        oldest_eligible_ms,
        ..Default::default()
    }
}

#[tokio::test]
async fn queue_idle_ms_ignores_deep_but_draining_and_deferred_backlogs() {
    let now = 1_800_000_000_000_i64;
    let long_ago = now - 48 * 60 * 60 * 1000;

    // Nothing queued at all ⇒ not stalled (an empty queue is done, not stuck).
    assert_eq!(queue_idle_ms(&queue(0, None, None), now), None);

    // A deep backlog whose oldest eligible job arrived 48h ago, and which
    // has never settled anything. A naive backlog-age metric reads 48h and
    // cries "stalled" — and here it is right, because a queue that has
    // never settled a job IS stalled.
    assert!(
        queue_idle_ms(&queue(3, None, Some(long_ago)), now).unwrap() >= QUEUE_STALL_THRESHOLD_MS,
        "a queue that has never settled a job IS stalled"
    );

    // Same 48h-old backlog, but one job settled a minute ago — the
    // pipeline is draining. This is the shape a backlog-age metric gets
    // wrong, and it describes the heavy users this issue is about.
    let idle = queue_idle_ms(&queue(3, Some(now - 60_000), Some(long_ago)), now)
        .expect("work still queued");
    assert!(
        idle < QUEUE_STALL_THRESHOLD_MS,
        "a deep but draining backlog must not read as stalled (idle={idle}ms)"
    );

    // Wholly deferred: every job is backing off, so nothing is runnable.
    // Asleep on purpose is not stuck.
    assert_eq!(
        queue_idle_ms(&queue(0, Some(long_ago), None), now),
        None,
        "wholly-deferred work is asleep, not stalled"
    );
}

/// #5324 regression (CodeRabbit/Codex): a queue that drained everything,
/// sat quiet for two days, then received one fresh eligible job must start
/// the idle clock at the NEW job's arrival — not inherit the ancient
/// completion. The prior `last_settled_ms.or(oldest_eligible_ms)` picked
/// the stale 48h-old settle and reported `degraded` the instant new work
/// appeared, before the worker had any chance to touch it.
#[tokio::test]
async fn queue_idle_ms_starts_from_fresh_work_not_ancient_completion() {
    let now = 1_800_000_000_000_i64;
    let long_ago = now - 48 * 60 * 60 * 1000;
    let just_now = now - 60_000;

    // The queue settled its last job 48h ago and went quiet; one fresh
    // eligible job arrived a minute ago.
    let idle = queue_idle_ms(&queue(1, Some(long_ago), Some(just_now)), now)
        .expect("fresh work is waiting");
    assert!(
        idle < QUEUE_STALL_THRESHOLD_MS,
        "freshly-enqueued work must start its own idle window, not inherit a 48h-old \
         completion (idle={idle}ms)"
    );
    assert_eq!(
        idle,
        now - just_now,
        "the idle clock starts at the new job's arrival, not the stale settle"
    );
}

/// One failure as the driver reports it.
///
/// These used to plant rows and read the answer back through a `SELECT`.
/// Which rows the driver reports is the driver's rule and is pinned in the
/// driver's own suite; what the host decides to *do* with a reported
/// failure is the rule below, and it depends on nothing but these three
/// values.
fn reported_failure(
    reason: &str,
    failed_at_ms: Option<i64>,
    last_success_ms: Option<i64>,
) -> crate::openhuman::memory::api::provider::types::QueueFailure {
    crate::openhuman::memory::api::provider::types::QueueFailure {
        reason: reason.to_string(),
        class: Some("unrecoverable".to_string()),
        completed_at_ms: failed_at_ms,
        last_success_ms,
    }
}

/// The active production defect: a signed-in user was told "No embeddings
/// credentials found. Log in to OpenHuman" because a batch of `auth_missing`
/// jobs had failed 27 days earlier and, being unrecoverable, was never
/// retried. The queue had been completing jobs the whole time since.
///
/// A failure the pipeline has already worked past is not the current
/// blocking cause, so no remediation is surfaced for it.
#[test]
fn blocking_cause_is_withheld_once_the_queue_has_succeeded_since() {
    let failed_at = 1_800_000_000_000_i64;
    let succeeded_after = failed_at + 27 * 24 * 60 * 60 * 1000;

    assert!(
        blocking_cause(&reported_failure(
            "auth_missing",
            Some(failed_at),
            Some(succeeded_after)
        ))
        .is_none(),
        "a month-old auth failure the queue has since worked past must not be \
         presented as the user's current problem"
    );
}

/// The other half of the same rule: a failure with no successful settle
/// after it IS the current blocking cause and must still surface, otherwise
/// the fix would silence the diagnosis it exists to deliver.
#[test]
fn blocking_cause_surfaces_when_nothing_has_succeeded_since() {
    use crate::openhuman::memory::tree::health::{FailureClass, FailureCode};

    let succeeded_before = 1_800_000_000_000_i64;
    let failed_after = succeeded_before + 60_000;

    let failure = blocking_cause(&reported_failure(
        "budget_exhausted",
        Some(failed_after),
        Some(succeeded_before),
    ))
    .expect("a failure with no success after it is the live cause");
    assert_eq!(failure.code, FailureCode::BudgetExhausted);
    assert_eq!(failure.class, FailureClass::Unrecoverable);
    assert_eq!(
        failure.remediation_key,
        "memory.health.remediation.budget_exhausted"
    );
}

/// A queue that has never completed anything has no watermark to compare
/// against, so the failure stands — this is the "broken from the first
/// sync" shape, where the diagnosis matters most.
#[test]
fn blocking_cause_surfaces_when_the_queue_has_never_succeeded() {
    let failure = blocking_cause(&reported_failure(
        "auth_invalid",
        Some(1_800_000_000_000_i64),
        None,
    ))
    .expect("no successful settle exists to supersede this failure");
    assert_eq!(
        failure.remediation_key,
        "memory.health.remediation.auth_invalid"
    );
}

/// A settle on the same millisecond as the failure does NOT supersede it.
///
/// The comparison is strictly `>`, and it has to be: `completed_at_ms` is
/// stamped on failure as well as success, so a job that fails and a job
/// that succeeds within the same millisecond are ordered by nothing. `>=`
/// would resolve that tie by hiding the failure, which is the direction
/// that loses a real diagnosis.
#[test]
fn a_settle_in_the_same_millisecond_does_not_supersede_the_failure() {
    let at = 1_800_000_000_000_i64;
    assert!(
        blocking_cause(&reported_failure("auth_invalid", Some(at), Some(at))).is_some(),
        "a success that cannot be shown to be later must not withhold the diagnosis"
    );
}

/// A failure carrying no completion time has nothing to compare against,
/// so it surfaces unconditionally rather than being withheld by a
/// watermark it cannot be ordered against.
#[test]
fn an_untimestamped_failure_surfaces_regardless_of_the_watermark() {
    let succeeded_at = 1_800_000_000_000_i64;
    assert!(
        blocking_cause(&reported_failure("auth_invalid", None, Some(succeeded_at))).is_some(),
        "without a failure timestamp there is no ordering, and withholding would \
         hide a live cause on a guess"
    );
}

/// On a fresh workspace the panel must report `idle` with zero
/// counters — the UI uses this to swap the loading skeleton for a
/// "no memory yet" state.
#[tokio::test]
async fn pipeline_status_returns_idle_for_empty_store() {
    // #002: the degraded flags are process-global; reset+serialise so a
    // parallel test (factory None-path, extract transport-fail) can't leak
    // a "degraded" signal into this fresh-workspace assertion.
    let _g = crate::openhuman::memory::tree::health::test_guard();
    let (_tmp, cfg) = test_config();
    // An empty driver, bound explicitly. Without a binding installed this
    // resolves the real one, which means loading the compiled module — and
    // in a test process that blocks rather than failing.
    bind_diagnostics(&cfg, Default::default(), Default::default());
    let out = pipeline_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(out.status, "idle");
    assert_eq!(out.total_chunks, 0);
    assert_eq!(out.last_sync_ms, 0);
    assert_eq!(out.pipeline_jobs.ready, 0);
    assert_eq!(out.pipeline_jobs.running, 0);
    assert_eq!(out.pipeline_jobs.failed, 0);
    assert!(!out.is_syncing);
    assert!(!out.is_paused);
    assert_eq!(out.wiki_size_bytes, 0, "no content dir yet");
    assert!(out.reason.is_none());
}

/// When the scheduler gate is `off`, the aggregated status flips to
/// `paused` regardless of the rest of the signals. This is the
/// invariant the toggle relies on.
#[tokio::test]
async fn pipeline_status_reflects_paused_when_scheduler_off() {
    use tinymemory_api::host::SchedulerGateMode;

    let (_tmp, mut cfg) = test_config();
    cfg.scheduler_gate.mode = SchedulerGateMode::Off;
    bind_diagnostics(&cfg, Default::default(), Default::default());
    let out = pipeline_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(out.status, "paused");
    assert!(out.is_paused);
    let reason = out.reason.expect("paused must carry a reason");
    assert!(reason.contains("off"), "reason should name the mode");
}

/// `pipeline_status` renders the aggregates the driver reports, and
/// derives a terminal status from them.
///
/// This used to ingest a document and assert the counters moved. That
/// half — an ingest raising the chunk count — is the driver's, and is
/// pinned in the driver's conformance suite against a real store. What is
/// the host's, and what this pins, is that the reported numbers reach the
/// wire unchanged and that a populated, idle store reads as terminal
/// rather than syncing.
#[tokio::test]
async fn pipeline_status_renders_the_drivers_chunk_aggregates() {
    use crate::openhuman::memory::api::provider::types::{QueueStats, StoreStats};

    // #002: reset+serialise the process-global degraded flags so this
    // "running" assertion isn't flipped to "degraded" by a parallel test.
    let _g = crate::openhuman::memory::tree::health::test_guard();
    let (_tmp, cfg) = test_config();

    let ingested_at = 1_800_000_000_000_i64;
    bind_diagnostics(
        &cfg,
        StoreStats {
            chunks: 4,
            chunks_with_structure: 1,
            most_recent_chunk_ms: Some(ingested_at),
        },
        QueueStats::default(),
    );

    let out = pipeline_status_rpc(&cfg).await.unwrap().value;
    assert_eq!(out.total_chunks, 4, "the driver's count reaches the wire");
    assert_eq!(
        out.last_sync_ms, ingested_at,
        "and so does its newest chunk's timestamp"
    );
    assert_eq!(
        out.extraction_coverage,
        Some(0.25),
        "coverage is the pair the driver reported, divided once"
    );
    // Provider availability differs between local and CI harnesses, so a
    // populated store may read as fully running or as degraded because
    // semantic recall or wiki structure was skipped. Both are terminal,
    // non-syncing states and both preserve the aggregates above.
    match out.status.as_str() {
        "running" => assert!(out.reason.is_none()),
        "degraded" => {
            let reason = out.reason.as_deref().unwrap_or_default();
            assert!(
                reason.contains("semantic recall disabled")
                    || reason.contains("wiki structure incomplete"),
                "degraded status should explain recall or structure loss: {:?}",
                out.reason
            );
        }
        other => panic!("expected running or degraded for a populated store, got {other}"),
    }
    assert!(!out.is_syncing);
}

/// `set_enabled` flips the persisted scheduler-gate mode and reports
/// `changed=true`; calling it again with the same value is a no-op
/// reporting `changed=false`. Uses an isolated `config_path` under
/// the workspace tempdir so `config.save()` doesn't touch the
/// host's real ~/.openhuman directory.
#[tokio::test]
async fn set_enabled_toggles_scheduler_gate_mode() {
    use tinymemory_api::host::SchedulerGateMode;

    let (tmp, mut cfg) = test_config();
    // Pin config_path inside the tempdir so `save()` stays sandboxed.
    cfg.config_path = tmp.path().join("config.toml");

    assert_eq!(cfg.scheduler_gate.mode, SchedulerGateMode::Auto);

    let off = set_enabled_rpc(&mut cfg, SetEnabledRequest { enabled: false })
        .await
        .unwrap()
        .value;
    assert!(!off.enabled);
    assert!(off.changed);
    assert_eq!(off.mode, "off");
    assert_eq!(cfg.scheduler_gate.mode, SchedulerGateMode::Off);

    // Calling with the same value must report no-op.
    let again = set_enabled_rpc(&mut cfg, SetEnabledRequest { enabled: false })
        .await
        .unwrap()
        .value;
    assert!(!again.changed, "duplicate toggle must be a no-op");

    // Flip back.
    let on = set_enabled_rpc(&mut cfg, SetEnabledRequest { enabled: true })
        .await
        .unwrap()
        .value;
    assert!(on.enabled);
    assert!(on.changed);
    assert_eq!(on.mode, "auto");
    assert_eq!(cfg.scheduler_gate.mode, SchedulerGateMode::Auto);
}
