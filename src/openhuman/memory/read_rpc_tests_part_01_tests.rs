use super::*;

#[tokio::test]
#[ignore = "needs a built tinymemory module (OPENHUMAN_MODULE_PATH) and its own process: \
chunk detail is read through the bound driver, not the in-process engine"]
async fn read_chunk_row_returns_preview_and_metadata() {
    let (_tmp, cfg) = test_config();
    seed_chat_chunk(
        &cfg,
        "slack:#eng",
        "phoenix migration scheduled friday with context and source refs",
    )
    .await;
    let chunk = list_chunks_rpc(&cfg, ChunkFilter::default())
        .await
        .unwrap()
        .value
        .chunks
        .into_iter()
        .next()
        .expect("seeded chunk");

    let row = read_chunk_row(&chunk.id).await.unwrap().expect("chunk row");
    assert_eq!(row.id, chunk.id);
    assert_eq!(row.source_kind, "chat");
    assert_eq!(row.source_id, "slack:#eng");
    assert_eq!(row.source_ref.as_deref(), Some("slack://x"));
    assert_eq!(row.owner, "alice");
    assert_eq!(row.lifecycle_status, "pending_extraction");
    assert!(row.content_path.is_some());
    assert!(row
        .content_preview
        .as_deref()
        .unwrap_or("")
        .contains("phoenix migration scheduled friday"));
}

#[tokio::test]
#[ignore = "needs a built tinymemory module (OPENHUMAN_MODULE_PATH) and its own process: \
chunk detail is read through the bound driver, not the in-process engine"]
async fn read_chunk_row_falls_back_to_sqlite_preview_when_file_missing() {
    let (_tmp, cfg) = test_config();
    let body = "sqlite preview survives missing file";
    seed_chat_chunk(&cfg, "slack:#eng", body).await;
    let chunk = list_chunks_rpc(&cfg, ChunkFilter::default())
        .await
        .unwrap()
        .value
        .chunks
        .into_iter()
        .next()
        .expect("seeded chunk");

    let rel_path = chunk.content_path.clone().expect("content path present");
    let abs_path = cfg.memory_tree_content_root().join(rel_path);
    std::fs::remove_file(&abs_path).expect("remove chunk file");

    let row = read_chunk_row(&chunk.id).await.unwrap().expect("chunk row");
    assert_eq!(row.content_path, chunk.content_path);
    assert!(row.content_preview.as_deref().unwrap_or("").contains(body));
}

/// The handler forwards the driver's flush outcome onto the wire unchanged.
///
/// The behaviour this test used to stage — an ingest producing a stale buffer,
/// the second flush deduplicating inside the window — is the *driver's* and
/// moved with the SQL to the conformance suite
/// (`flushing_twice_in_a_window_schedules_the_work_once`), where a real store
/// exists. What is the host's here is only the mapping: both fields pass
/// through, and the u64→u32 buffer count clamps rather than wraps.
#[tokio::test]
async fn flush_now_reports_the_drivers_outcome() {
    use crate::openhuman::memory::api::provider::types::FlushOutcome;

    let (_tmp, cfg) = test_config();
    crate::openhuman::memory::binding::install_for_test(
        &cfg.workspace_dir,
        &cfg.subsystems.memory,
        std::sync::Arc::new(
            crate::openhuman::memory::binding::FixedDiagnostics::new(
                Default::default(),
                Default::default(),
            )
            .flushing(FlushOutcome {
                enqueued: false,
                stale_buffers: u64::from(u32::MAX) + 7,
            }),
        ) as std::sync::Arc<dyn crate::openhuman::memory::api::provider::MemoryProvider>,
    );

    let resp = flush_now_rpc(&cfg).await.expect("flush_now").value;
    assert!(
        !resp.enqueued,
        "`enqueued: false` passes through — with a non-zero buffer count it \
         means \"already scheduled\", not \"nothing to do\""
    );
    assert_eq!(
        resp.stale_buffers,
        u32::MAX,
        "a count past the wire type's range clamps rather than wraps to a small lie"
    );
}
