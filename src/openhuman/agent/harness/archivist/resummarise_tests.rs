//! Recovery-pass tests (#6186).
//!
//! Driven through the **contract** like the finalize-path tests beside them: a
//! `RecordingProvider` serves every family and records what reached it, so
//! "nothing was written" is asserted against the driver's call log rather than
//! inferred from an empty store.
//!
//! `ArchivistHook::new` leaves `summariser_available == false`, so
//! `summarize_entries` takes its heuristic arm deterministically — no chat
//! model is built, resolved or called anywhere here. That makes the *write*
//! assertions negative by construction, which is the right shape: the pass must
//! be provably incapable of persisting a recap it did not get from a model,
//! and that is the same rule finalize is held to.

use std::sync::Arc;

use super::*;
use crate::openhuman::memory::api::provider::{ConversationSegment, MemoryProvider, SegmentStatus};
use crate::openhuman::memory::guard::test_support::RecordingProvider;

const SESSION: &str = "resummarise-6186";

fn turn(id: i64, role: &str, content: &str) -> EpisodicTurn {
    EpisodicTurn {
        id: Some(id),
        session_id: SESSION.into(),
        timestamp: 100.0 + id as f64,
        role: role.into(),
        content: content.into(),
        lesson: None,
        tool_calls_json: None,
        cost_microdollars: 0,
    }
}

/// A pending segment. `segment_id` is a parameter because the attempt ledger
/// that bounds the head-of-queue trap is **per process**, so two tests sharing
/// an id would share its budget and run order would decide which one passed.
fn pending(segment_id: &str) -> ConversationSegment {
    ConversationSegment {
        segment_id: segment_id.into(),
        session_id: SESSION.into(),
        namespace: "global".into(),
        start_episodic_id: 1,
        end_episodic_id: Some(2),
        start_timestamp: 100.0,
        end_timestamp: Some(103.0),
        turn_count: 2,
        summary: None,
        embedding: None,
        open: false,
        status: Some(SegmentStatus::Closed),
        start_seq: None,
        end_seq: None,
    }
}

fn turns() -> Vec<EpisodicTurn> {
    vec![
        turn(1, "user", "How do I pin a submodule?"),
        turn(2, "assistant", "Record the gitlink at the commit you want."),
    ]
}

fn methods(recording: &RecordingProvider) -> Vec<String> {
    recording.calls().into_iter().map(|c| c.method).collect()
}

fn hook_over(recording: &Arc<RecordingProvider>) -> ArchivistHook {
    let provider: Arc<dyn MemoryProvider> = recording.clone();
    ArchivistHook::new(provider, true)
}

/// The pass reads the queue and the pending segment's turns — it does not stop
/// at the query.
///
/// The positive half of the assertion is the point. Without it this would also
/// pass if `resummarise_pending` had returned straight after
/// `segments_pending_summary`, which is the failure mode a purely negative test
/// cannot see.
#[tokio::test]
async fn a_pending_segment_is_read_and_recapped() {
    let recording = Arc::new(
        RecordingProvider::new()
            .with_session_turns(turns())
            .with_pending_segments(vec![pending("seg-read")]),
    );
    let hook = hook_over(&recording);

    hook.resummarise_pending(300.0).await;

    let methods = methods(&recording);
    assert!(
        methods
            .iter()
            .any(|m| m == "episodic.segments_pending_summary"),
        "the pending queue was never read: {methods:?}"
    );
    assert!(
        methods.iter().any(|m| m == "episodic.session_turns"),
        "the segment's turns were never read: {methods:?}"
    );
}

/// A recap that fails again writes nothing, and leaves the marker in place.
///
/// Same rule as finalize (#6156): the heuristic bookend is not a summary, and
/// persisting it would flip the row to `summarised` and remove it from the very
/// queue this pass selects on — turning a recoverable segment into a
/// permanently degraded one. A recovery pass that did that would be worse than
/// no recovery pass.
#[tokio::test]
async fn a_still_failing_recap_is_not_persisted_or_embedded() {
    let recording = Arc::new(
        RecordingProvider::new()
            .with_session_turns(turns())
            .with_pending_segments(vec![pending("seg-still-failing")]),
    );
    let hook = hook_over(&recording);

    hook.resummarise_pending(300.0).await;

    let methods = methods(&recording);
    for forbidden in [
        "episodic.set_segment_summary",
        "episodic.upsert_segment_embedding",
        "scoring.embed_text",
        "scoring.embedder_slug",
    ] {
        assert!(
            !methods.iter().any(|m| m == forbidden),
            "`{forbidden}` ran on a heuristic recap: {methods:?}"
        );
    }
}

/// An empty queue costs one call and stops.
///
/// Pins the early return: the pass runs after every mid-session segment close,
/// so the healthy steady state must not read turns or build a corpus for
/// segments that do not exist.
#[tokio::test]
async fn an_empty_queue_reads_nothing_else() {
    let recording = Arc::new(RecordingProvider::new().with_session_turns(turns()));
    let hook = hook_over(&recording);

    hook.resummarise_pending(300.0).await;

    let methods = methods(&recording);
    assert_eq!(
        methods,
        vec!["episodic.segments_pending_summary".to_string()],
        "an empty queue did more than query: {methods:?}"
    );
}

/// A segment that keeps failing is dropped from the pass, so the queue behind
/// it can drain.
///
/// This is the head-of-queue trap, and it is not hypothetical: the driver
/// orders oldest-first, and a segment nothing can ever summarise stays at the
/// head. Without the ledger the pass would re-attempt that same segment after
/// every close and never reach the ones behind it — the same shape as the
/// head-500 loop in #6051.
#[tokio::test]
async fn a_repeatedly_failing_segment_stops_being_attempted() {
    let recording = Arc::new(
        RecordingProvider::new()
            .with_session_turns(turns())
            .with_pending_segments(vec![pending("seg-exhausts")]),
    );
    let hook = hook_over(&recording);

    // Two attempts are the budget; the third pass must skip it.
    hook.resummarise_pending(300.0).await;
    hook.resummarise_pending(301.0).await;
    let before = methods(&recording)
        .iter()
        .filter(|m| *m == "episodic.session_turns")
        .count();

    hook.resummarise_pending(302.0).await;

    let after = methods(&recording)
        .iter()
        .filter(|m| *m == "episodic.session_turns")
        .count();
    assert_eq!(
        before, 2,
        "the first two passes should each have attempted the segment"
    );
    assert_eq!(
        after, before,
        "the third pass attempted an exhausted segment instead of skipping it"
    );
}

/// A segment whose turns are gone is skipped without a write.
///
/// The row can outlive its turns, and the honest answer is to leave it alone:
/// there is no recap that could be produced, and writing an empty summary would
/// seal it as if there were.
#[tokio::test]
async fn a_segment_with_no_turns_left_is_skipped() {
    let recording =
        Arc::new(RecordingProvider::new().with_pending_segments(vec![pending("seg-no-turns")]));
    let hook = hook_over(&recording);

    hook.resummarise_pending(300.0).await;

    let methods = methods(&recording);
    assert!(
        !methods.iter().any(|m| m == "episodic.set_segment_summary"),
        "a segment with no turns was summarised anyway: {methods:?}"
    );
}
