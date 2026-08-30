//! Maps the agent's per-turn [`AgentProgress`] stream onto Medulla's native
//! harness-event envelope.

use serde::{Deserialize, Serialize};

use crate::openhuman::agent::progress::AgentProgress;

const MEDULLA_ENVELOPE_VERSION: &str = "medulla.harness.session.v1";

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextPayload {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolCallPayload {
    #[serde(default)]
    pub call_id: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_kind: String,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolResultPayload {
    #[serde(default)]
    pub call_id: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub output_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ApprovalRequestPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StatusPayload {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ErrorPayload {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub fatal: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum HarnessEventKind {
    AgentMessage(TextPayload),
    AgentThinking(TextPayload),
    ToolCall(ToolCallPayload),
    ToolResult(ToolResultPayload),
    ApprovalRequest(ApprovalRequestPayload),
    Status(StatusPayload),
    Error(ErrorPayload),
    Unknown(serde_json::Value),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessScope {
    #[serde(rename = "type", default)]
    pub scope_type: String,
    #[serde(default)]
    pub wrapper_session_id: String,
    #[serde(default)]
    pub harness_session_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessEvent {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub seq: i64,
    #[serde(default)]
    pub ts: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl HarnessEvent {
    pub fn decoded(&self) -> HarnessEventKind {
        serde_json::from_value(serde_json::json!({
            "kind": self.kind,
            "payload": self.payload,
        }))
        .unwrap_or_else(|_| HarnessEventKind::Unknown(self.payload.clone()))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessEnvelope {
    #[serde(default)]
    pub envelope_version: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub scope: HarnessScope,
    #[serde(default)]
    pub event: HarnessEvent,
}

impl HarnessEnvelope {
    pub fn is_valid(&self) -> bool {
        self.envelope_version == MEDULLA_ENVELOPE_VERSION
            && !self.scope.harness_session_id.is_empty()
    }

    pub fn parse(body: &str) -> Option<Self> {
        let envelope: Self = serde_json::from_str(body).ok()?;
        envelope.is_valid().then_some(envelope)
    }
}

/// `role` stamped on every openhuman-produced event: these are agent-side
/// stream frames (`owner` is reserved for `user_prompt`, which the agent never
/// emits about itself).
const AGENT_ROLE: &str = "agent";

/// Translate one [`AgentProgress`] event into a typed v2 event kind.
///
/// Returns `None` for progress variants that carry no user-facing stream frame
/// (cost rollups, per-call token accounting, arg-delta fragments, …) so the
/// forwarded stream stays close to the `agent_message / agent_thinking /
/// tool_call / tool_result / status / approval_request / error` vocabulary the
/// spec enumerates.
pub fn progress_to_event_kind(progress: &AgentProgress) -> Option<HarnessEventKind> {
    let kind = match progress {
        AgentProgress::TurnStarted => HarnessEventKind::Status(StatusPayload {
            state: "running".to_string(),
            detail: "turn started".to_string(),
            active_call_id: None,
        }),
        AgentProgress::IterationStarted {
            iteration,
            max_iterations,
        } => HarnessEventKind::Status(StatusPayload {
            state: "running".to_string(),
            detail: format!("iteration {iteration}/{max_iterations}"),
            active_call_id: None,
        }),
        AgentProgress::TextDelta { delta, .. } => HarnessEventKind::AgentMessage(TextPayload {
            text: delta.clone(),
        }),
        AgentProgress::ThinkingDelta { delta, .. } => {
            HarnessEventKind::AgentThinking(TextPayload {
                text: delta.clone(),
            })
        }
        AgentProgress::ToolCallStarted {
            call_id,
            tool_name,
            arguments,
            display_label,
            ..
        } => HarnessEventKind::ToolCall(ToolCallPayload {
            call_id: call_id.clone(),
            tool_name: tool_name.clone(),
            tool_kind: "other".to_string(),
            display: display_label.clone().unwrap_or_else(|| tool_name.clone()),
            input: arguments.clone(),
        }),
        AgentProgress::ToolCallCompleted {
            call_id,
            success,
            output,
            output_chars,
            ..
        } => HarnessEventKind::ToolResult(ToolResultPayload {
            call_id: call_id.clone(),
            ok: *success,
            exit_code: None,
            is_error: !*success,
            output: output.clone(),
            output_bytes: *output_chars as i64,
        }),
        AgentProgress::SubagentAwaitingUser {
            task_id, question, ..
        } => HarnessEventKind::ApprovalRequest(ApprovalRequestPayload {
            call_id: Some(task_id.clone()),
            tool_name: "subagent".to_string(),
            display: question.clone(),
            reason: None,
        }),
        AgentProgress::TurnCompleted { .. } => HarnessEventKind::Status(StatusPayload {
            state: "idle".to_string(),
            detail: "turn completed".to_string(),
            active_call_id: None,
        }),
        // Everything else (arg deltas, cost/usage rollups, per-call model
        // accounting, subagent-internal frames, task-board writes, raw
        // TurnContent) carries no distinct stream frame in this vocabulary.
        _ => return None,
    };
    Some(kind)
}

/// Wrap a typed [`HarnessEventKind`] in a full [`HarnessEnvelope`] anchored to
/// `session_id`, ready to serialize into a `medulla:task_envelope` frame.
///
/// `seq` is the monotonic per-session ordering counter; `ts` is an ISO-8601
/// timestamp.
pub fn envelope_for_kind(session_id: &str, seq: i64, kind: &HarnessEventKind) -> HarnessEnvelope {
    // `HarnessEventKind` is adjacently tagged (`{ "kind": .., "payload": .. }`),
    // so serializing it yields exactly the `kind`/`payload` pair `HarnessEvent`
    // stores — extract them rather than hand-writing the discriminator strings.
    let tagged = serde_json::to_value(kind).unwrap_or_default();
    let kind_str = tagged
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let payload = tagged
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    HarnessEnvelope {
        envelope_version: MEDULLA_ENVELOPE_VERSION.to_string(),
        version: 1,
        scope: HarnessScope {
            scope_type: "session".to_string(),
            wrapper_session_id: session_id.to_string(),
            harness_session_id: session_id.to_string(),
        },
        event: HarnessEvent {
            id: format!("{session_id}-{seq}"),
            seq,
            ts: chrono::Utc::now().to_rfc3339(),
            role: AGENT_ROLE.to_string(),
            kind: kind_str,
            payload,
        },
    }
}

/// Convenience: build a bare `status` envelope (used to bookend a task run).
pub fn status_envelope(session_id: &str, seq: i64, state: &str, detail: &str) -> HarnessEnvelope {
    envelope_for_kind(
        session_id,
        seq,
        &HarnessEventKind::Status(StatusPayload {
            state: state.to_string(),
            detail: detail.to_string(),
            active_call_id: None,
        }),
    )
}

/// Convenience: build an `error` envelope.
pub fn error_envelope(session_id: &str, seq: i64, message: &str, fatal: bool) -> HarnessEnvelope {
    envelope_for_kind(
        session_id,
        seq,
        &HarnessEventKind::Error(ErrorPayload {
            message: message.to_string(),
            fatal,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_text_delta_to_agent_message() {
        let p = AgentProgress::TextDelta {
            delta: "hello".to_string(),
            iteration: 1,
        };
        let kind = progress_to_event_kind(&p).expect("mapped");
        assert!(matches!(kind, HarnessEventKind::AgentMessage(ref t) if t.text == "hello"));
    }

    #[test]
    fn maps_thinking_delta_to_agent_thinking() {
        let p = AgentProgress::ThinkingDelta {
            delta: "hmm".to_string(),
            iteration: 1,
        };
        assert!(matches!(
            progress_to_event_kind(&p),
            Some(HarnessEventKind::AgentThinking(_))
        ));
    }

    #[test]
    fn drops_non_stream_variants() {
        let p = AgentProgress::TurnContent {
            input: Some("q".to_string()),
            output: Some("a".to_string()),
        };
        assert!(progress_to_event_kind(&p).is_none());
    }

    #[test]
    fn envelope_is_valid_and_round_trips_through_the_wire() {
        let kind = HarnessEventKind::AgentMessage(TextPayload {
            text: "done".to_string(),
        });
        let env = envelope_for_kind("sess-1", 7, &kind);
        assert!(env.is_valid());
        assert_eq!(env.event.kind, "agent_message");
        assert_eq!(env.event.seq, 7);

        // Serialize and parse back through the Medulla decoder.
        let wire = serde_json::to_string(&env).unwrap();
        let parsed = HarnessEnvelope::parse(&wire).expect("valid Medulla wire");
        match parsed.event.decoded() {
            HarnessEventKind::AgentMessage(t) => assert_eq!(t.text, "done"),
            other => panic!("unexpected decoded kind: {other:?}"),
        }
    }

    #[test]
    fn tool_call_and_result_map_to_their_kinds() {
        let started = AgentProgress::ToolCallStarted {
            call_id: "c1".to_string(),
            tool_name: "Bash".to_string(),
            arguments: serde_json::json!({ "cmd": "ls" }),
            iteration: 1,
            display_label: Some("List files".to_string()),
            display_detail: None,
        };
        match progress_to_event_kind(&started) {
            Some(HarnessEventKind::ToolCall(tc)) => {
                assert_eq!(tc.call_id, "c1");
                assert_eq!(tc.tool_name, "Bash");
                assert_eq!(tc.display, "List files");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }

        let completed = AgentProgress::ToolCallCompleted {
            call_id: "c1".to_string(),
            tool_name: "Bash".to_string(),
            success: true,
            output_chars: 3,
            output: "ok\n".to_string(),
            arguments: None,
            elapsed_ms: 5,
            iteration: 1,
            failure: None,
        };
        match progress_to_event_kind(&completed) {
            Some(HarnessEventKind::ToolResult(tr)) => {
                assert!(tr.ok);
                assert!(!tr.is_error);
                assert_eq!(tr.output, "ok\n");
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }
}
