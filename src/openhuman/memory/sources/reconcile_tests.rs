use super::*;
use crate::openhuman::memory::sync::composio::SyncTarget;

fn sync_target(toolkit: &str, connection_id: &str) -> SyncTarget {
    SyncTarget {
        toolkit: toolkit.to_string(),
        connection_id: connection_id.to_string(),
    }
}

#[test]
fn build_upsert_targets_formats_label_and_preserves_order() {
    let targets = vec![
        sync_target("gmail", "ca_WaktIDFlZwXO"),
        sync_target("slack", "short"),
    ];
    let out = build_upsert_targets(&targets);
    assert_eq!(out.len(), 2);
    // (toolkit, connection_id, label) — toolkit/connection_id carried through verbatim.
    assert_eq!(out[0].0, "gmail");
    assert_eq!(out[0].1, "ca_WaktIDFlZwXO");
    assert_eq!(out[0].2, "Gmail · IDFlZwXO");
    assert_eq!(out[1].0, "slack");
    assert_eq!(out[1].1, "short");
    assert_eq!(out[1].2, "Slack · short");
}

#[test]
fn build_upsert_targets_empty_is_empty() {
    let out = build_upsert_targets(&[]);
    assert!(out.is_empty());
}

#[test]
fn short_id_truncates_ascii() {
    assert_eq!(short_id("ca_WaktIDFlZwXO"), "IDFlZwXO");
}

#[test]
fn short_id_short_input_passthrough() {
    assert_eq!(short_id("abc"), "abc");
    assert_eq!(short_id("12345678"), "12345678");
}

#[test]
fn short_id_utf8_safe() {
    // Multi-byte chars would have panicked with byte-slicing.
    let s = "🦀🐢🐙🦊🐼🐰🐯🐸🦁";
    let out = short_id(s);
    assert_eq!(out.chars().count(), 8);
}
