use super::*;
use serde_json::json;

#[test]
fn tunnel_register_response_accepts_backend_ack_shape_without_session_token() {
    let response: TunnelRegisterResponse = serde_json::from_value(json!({
        "channelId": "ch_123",
        "pairingToken": "pt_123",
        "pairingExpiresAt": "2026-06-30T15:00:00Z"
    }))
    .expect("backend register ack shape should parse");

    assert_eq!(response.channel_id, "ch_123");
    assert_eq!(response.pairing_token, "pt_123");
    assert_eq!(response.pairing_expires_at, "2026-06-30T15:00:00Z");
}

/// Regression for #5871: backend PR #709 switched the tunnel:register ACK
/// from camelCase to snake_case. The struct must accept both shapes so the
/// client works against old and new backend versions without a forced deploy.
#[test]
fn tunnel_register_response_accepts_snake_case_ack_shape() {
    let response: TunnelRegisterResponse = serde_json::from_value(json!({
        "channel_id": "ch_456",
        "pairing_token": "pt_456",
        "pairing_expires_at": "2026-09-30T12:00:00Z"
    }))
    .expect("snake_case register ack shape (backend PR #709) should parse");

    assert_eq!(response.channel_id, "ch_456");
    assert_eq!(response.pairing_token, "pt_456");
    assert_eq!(response.pairing_expires_at, "2026-09-30T12:00:00Z");
}

#[test]
fn build_core_connect_payload_omits_session_token_for_core_role() {
    let payload = build_core_connect_payload("ch_123");

    assert_eq!(payload["channelId"], "ch_123");
    assert_eq!(payload["role"], "core");
    assert!(payload.get("sessionToken").is_none());
    assert!(payload.get("pairingToken").is_none());
}
