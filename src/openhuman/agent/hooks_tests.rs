use super::*;

#[test]
fn sanitize_success_includes_char_count() {
    let out = sanitize_tool_output("hello world", "read_file", true);
    assert_eq!(out, "read_file: ok (11 chars)");
}

#[test]
fn sanitize_success_empty_output() {
    let out = sanitize_tool_output("", "write_file", true);
    assert_eq!(out, "write_file: ok (0 chars)");
}

#[test]
fn sanitize_failure_timeout() {
    let out = sanitize_tool_output("connection timeout after 30s", "http_request", false);
    assert_eq!(out, "http_request: failed (timeout)");
}

#[test]
fn sanitize_failure_not_found() {
    let out = sanitize_tool_output("no such file or directory", "read_file", false);
    assert_eq!(out, "read_file: failed (not_found)");
}

#[test]
fn sanitize_failure_not_found_variant() {
    let out = sanitize_tool_output("resource Not Found", "api_call", false);
    assert_eq!(out, "api_call: failed (not_found)");
}

#[test]
fn sanitize_failure_permission_denied() {
    let out = sanitize_tool_output("Permission denied", "exec", false);
    assert_eq!(out, "exec: failed (permission_denied)");
}

#[test]
fn sanitize_failure_connection_error() {
    let out = sanitize_tool_output("network unreachable", "fetch", false);
    assert_eq!(out, "fetch: failed (connection_error)");
}

#[test]
fn sanitize_failure_connection_variant() {
    let out = sanitize_tool_output("Connection refused", "fetch", false);
    assert_eq!(out, "fetch: failed (connection_error)");
}

#[test]
fn sanitize_failure_parse_error() {
    let out = sanitize_tool_output("invalid JSON syntax", "parse", false);
    assert_eq!(out, "parse: failed (parse_error)");
}

#[test]
fn sanitize_failure_parse_variant() {
    let out = sanitize_tool_output("failed to parse response", "api", false);
    assert_eq!(out, "api: failed (parse_error)");
}

#[test]
fn sanitize_failure_unknown_tool() {
    let out = sanitize_tool_output("unknown tool requested", "bad_tool", false);
    assert_eq!(out, "bad_tool: failed (unknown_tool)");
}

#[test]
fn sanitize_failure_generic_error() {
    let out = sanitize_tool_output("something went wrong", "tool", false);
    assert_eq!(out, "tool: failed (error)");
}

#[test]
fn turn_context_serde_roundtrip() {
    let ctx = TurnContext {
        user_message: "hello".into(),
        assistant_response: "hi".into(),
        tool_calls: vec![ToolCallRecord {
            name: "read".into(),
            arguments: serde_json::json!({"path": "/tmp"}),
            success: true,
            output_summary: "read: ok (100 chars)".into(),
            duration_ms: 42,
        }],
        turn_duration_ms: 500,
        session_id: Some("sess-1".into()),
        agent_id: Some("orchestrator".into()),
        entrypoint: Some("cli".into()),
        iteration_count: 2,
    };
    let json = serde_json::to_string(&ctx).unwrap();
    let back: TurnContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back.user_message, "hello");
    assert_eq!(back.tool_calls.len(), 1);
    assert_eq!(back.tool_calls[0].name, "read");
    assert_eq!(back.iteration_count, 2);
}

#[tokio::test]
async fn fire_hooks_accepts_empty_hook_list() {
    let ctx = TurnContext {
        user_message: "x".into(),
        assistant_response: "y".into(),
        tool_calls: vec![],
        turn_duration_ms: 1,
        session_id: None,
        agent_id: None,
        entrypoint: None,
        iteration_count: 1,
    };
    // Should not panic
    fire_hooks(&[], ctx);
}
