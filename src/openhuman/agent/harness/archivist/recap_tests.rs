use super::*;

fn segment() -> ConversationSegment {
    ConversationSegment {
        segment_id: "segment".into(),
        session_id: "session".into(),
        namespace: "global".into(),
        start_episodic_id: 20,
        end_episodic_id: Some(24),
        start_timestamp: 100.000_9,
        end_timestamp: Some(100.001_1),
        turn_count: 3,
        summary: None,
        embedding: None,
        open: false,
        start_seq: Some(10),
        end_seq: Some(14),
    }
}

fn entry(sequence: Option<u32>, id: Option<i64>, timestamp: f64) -> SessionEntry {
    SessionEntry {
        sequence,
        turn: EpisodicTurn {
            id,
            session_id: "session".into(),
            timestamp,
            role: "user".into(),
            content: "content".into(),
            lesson: None,
            tool_calls_json: None,
            cost_microdollars: 0,
        },
    }
}

#[test]
fn segment_membership_uses_sequence_instead_of_rounded_timestamp() {
    let segment = segment();

    assert!(!entry(Some(9), None, 100.001).is_in_segment(&segment));
    assert!(entry(Some(10), None, 100.000).is_in_segment(&segment));
    assert!(entry(Some(15), None, 101.0).is_in_segment(&segment));
    assert!(!entry(Some(16), None, 100.001).is_in_segment(&segment));
}

#[test]
fn segment_membership_falls_back_to_episodic_id() {
    let mut segment = segment();
    segment.start_seq = None;
    segment.end_seq = None;

    assert!(!entry(None, Some(19), 100.001).is_in_segment(&segment));
    assert!(entry(None, Some(20), 100.000).is_in_segment(&segment));
    assert!(entry(None, Some(25), 101.0).is_in_segment(&segment));
    assert!(!entry(None, Some(26), 100.001).is_in_segment(&segment));
}
