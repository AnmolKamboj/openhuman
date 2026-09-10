use super::journal_projection::spans_from_observations;
use super::{SpanKind, SpanStatus, TraceContext};
use tinyagents_harness::events::AgentEvent;
use tinyagents_harness::ids::{CallId, EventId, RunId};
use tinyagents_harness::observability::AgentObservation;
use tinyinference::usage::Usage;

/// Wraps an event as a journalled observation stamped at `ts`.
fn obs(offset: u64, ts: u64, event: AgentEvent) -> AgentObservation {
    AgentObservation {
        event_id: EventId::new(format!("run-1-evt-{offset}")),
        run_id: RunId::new("run-1"),
        parent_run_id: None,
        root_run_id: RunId::new("run-1"),
        offset,
        ts_ms: ts,
        event,
    }
}

fn tool_completed(call: &str, name: &str, error: Option<&str>) -> AgentEvent {
    AgentEvent::ToolCompleted {
        call_id: CallId::new(call),
        tool_name: name.to_string(),
        started_at_ms: Some(1_020),
        input: None,
        output: None,
        duration_ms: Some(30),
        output_bytes: Some(12),
        error: error.map(str::to_string),
    }
}

fn single_turn(tool_error: Option<&str>) -> Vec<AgentObservation> {
    vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ToolStarted {
                call_id: CallId::new("t1"),
                tool_name: "lookup".to_string(),
            },
        ),
        obs(3, 1_050, tool_completed("t1", "lookup", tool_error)),
        obs(
            4,
            1_060,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("c1"),
                started_at_ms: Some(1_010),
                usage: Some(Usage::new(100, 20)),
                input: None,
                output: None,
            },
        ),
        obs(
            5,
            1_070,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ]
}

fn subagent_turn() -> Vec<AgentObservation> {
    vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ModelStarted {
                call_id: CallId::new("scout-model"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::ToolStarted {
                call_id: CallId::new("scout-tool"),
                tool_name: "read_file".to_string(),
            },
        ),
        obs(4, 1_060, tool_completed("scout-tool", "read_file", None)),
        obs(
            5,
            1_070,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("scout-model"),
                started_at_ms: Some(1_020),
                usage: Some(Usage::new(11, 7)),
                input: None,
                output: None,
            },
        ),
        obs(
            6,
            1_090,
            AgentEvent::SubAgentCompleted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            7,
            1_100,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ]
}

fn ctx() -> TraceContext {
    TraceContext::new("session-1", Some("user-1".into()))
}

#[test]
fn projects_single_agent_turn_span_tree() {
    let spans = spans_from_observations(ctx(), 10, &single_turn(None));

    // Turn + iteration + tool + generation spans are all present.
    let by_kind = |k: SpanKind| spans.iter().filter(|s| s.kind == k).count();
    assert_eq!(by_kind(SpanKind::Turn), 1, "one turn span");
    assert_eq!(by_kind(SpanKind::Iteration), 1, "one iteration span");
    assert_eq!(by_kind(SpanKind::Tool), 1, "one tool span");
    assert_eq!(by_kind(SpanKind::Generation), 1, "one generation span");

    let tool = spans.iter().find(|s| s.kind == SpanKind::Tool).unwrap();
    assert_eq!(tool.name, "tool.lookup");
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(true));
    assert_eq!(tool.attributes["tool.output_chars"], serde_json::json!(12));
    assert_eq!(tool.attributes["tool.elapsed_ms"], serde_json::json!(30));

    let generation = spans
        .iter()
        .find(|s| s.kind == SpanKind::Generation)
        .unwrap();
    assert!(
        generation.name.contains("gpt-4"),
        "gen span names the model"
    );
}

#[test]
fn failed_tool_projects_error_outcome() {
    // With content capture on, a failed tool span carries the classified
    // cause reconstructed from the journalled error string, reproducing the
    // live path's `classify(error, false)` exactly.
    let spans = spans_from_observations(
        ctx().with_capture_content(true),
        10,
        &single_turn(Some("permission denied opening /etc/x")),
    );
    let tool = spans.iter().find(|s| s.kind == SpanKind::Tool).unwrap();
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(false));
    assert!(
        tool.attributes.contains_key("error.message"),
        "failed tool span carries a classified error message"
    );
}

#[test]
fn projects_subagent_scope_from_lifecycle_brackets() {
    let spans = spans_from_observations(ctx(), 10, &subagent_turn());

    let by_kind = |k: SpanKind| spans.iter().filter(|s| s.kind == k).count();
    assert_eq!(by_kind(SpanKind::Subagent), 1, "one subagent span");
    assert_eq!(
        by_kind(SpanKind::SubagentIteration),
        1,
        "one child iteration span"
    );
    assert_eq!(by_kind(SpanKind::Tool), 1, "one child tool span");
    assert_eq!(by_kind(SpanKind::Generation), 1, "one child generation");

    let subagent = spans.iter().find(|s| s.kind == SpanKind::Subagent).unwrap();
    assert_eq!(
        subagent.attributes["subagent.agent_id"],
        serde_json::json!("researcher")
    );
    assert_eq!(
        subagent.attributes["subagent.iterations"],
        serde_json::json!(1)
    );

    let child_iteration = spans
        .iter()
        .find(|s| s.kind == SpanKind::SubagentIteration)
        .unwrap();
    assert_eq!(
        child_iteration.parent_span_id.as_deref(),
        Some(subagent.span_id.as_str())
    );
}

#[test]
fn projects_failed_subagent_from_child_run_failed() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::RunFailed {
                run_id: RunId::new("run-1"),
                error: "provider unavailable".to_string(),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ];

    let spans = spans_from_observations(ctx(), 10, &observations);
    let subagent = spans.iter().find(|s| s.kind == SpanKind::Subagent).unwrap();

    assert_eq!(subagent.status, SpanStatus::Error);
    assert_eq!(subagent.attributes["error"], serde_json::json!(true));
    assert!(
        subagent.attributes.get("error.length").is_some(),
        "failed subagent span carries redacted error metadata"
    );
}

#[test]
fn projects_turn_content_from_root_model_io() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("m1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("m1"),
                started_at_ms: Some(1_010),
                usage: Some(Usage::new(5, 3)),
                input: Some(serde_json::json!([
                    {"role": "user", "content": "summarize this"}
                ])),
                output: Some(serde_json::json!({
                    "role": "assistant",
                    "content": "short summary"
                })),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ];
    let spans = spans_from_observations(ctx().with_capture_content(true), 10, &observations);
    let turn = spans.iter().find(|s| s.kind == SpanKind::Turn).unwrap();

    assert!(
        turn.input
            .as_ref()
            .unwrap()
            .to_string()
            .contains("summarize this"),
        "root model input is attached through TurnContent"
    );
    assert_eq!(
        turn.output.as_ref().unwrap(),
        &serde_json::json!("short summary")
    );
}

/// A top-level model call that journals usage: `ModelStarted` → `UsageRecorded`
/// → `ModelCompleted`, the order the agent loop emits them in
/// (`run_loop.rs` emits `UsageRecorded` before `ModelCompleted`).
fn turn_with_usage(usages: &[Usage]) -> Vec<AgentObservation> {
    let mut events = vec![obs(
        0,
        1_000,
        AgentEvent::RunStarted {
            run_id: RunId::new("run-1"),
            thread_id: None,
        },
    )];
    let mut offset = 1;
    for (index, usage) in usages.iter().enumerate() {
        let call = format!("c{}", index + 1);
        events.push(obs(
            offset,
            1_010 + offset * 10,
            AgentEvent::ModelStarted {
                call_id: CallId::new(&call),
                model: "gpt-4".to_string(),
            },
        ));
        offset += 1;
        events.push(obs(
            offset,
            1_010 + offset * 10,
            AgentEvent::UsageRecorded { usage: *usage },
        ));
        offset += 1;
        events.push(obs(
            offset,
            1_010 + offset * 10,
            AgentEvent::ModelCompleted {
                call_id: CallId::new(&call),
                started_at_ms: Some(1_010),
                usage: Some(*usage),
                input: None,
                output: None,
            },
        ));
        offset += 1;
    }
    events.push(obs(
        offset,
        1_010 + offset * 10,
        AgentEvent::RunCompleted {
            run_id: RunId::new("run-1"),
        },
    ));
    events
}

fn turn_span(spans: &[super::TraceSpan]) -> &super::TraceSpan {
    spans
        .iter()
        .find(|s| s.kind == SpanKind::Turn)
        .expect("a root turn span")
}

/// #6148: the journal-shadow parity check compares attribute *keys*, and the
/// root turn span's `gen_ai.*` group is written only by `TurnCostUpdated` —
/// which the projection never emitted, so every turn diverged.
#[test]
fn usage_recorded_projects_the_turn_cost_rollup() {
    let spans = spans_from_observations(ctx(), 10, &turn_with_usage(&[Usage::new(100, 20)]));
    let turn = turn_span(&spans);

    for key in [
        "gen_ai.request.model",
        "gen_ai.usage.input_tokens",
        "gen_ai.usage.output_tokens",
        "gen_ai.usage.cached_input_tokens",
        "gen_ai.usage.cost_usd",
    ] {
        assert!(
            turn.attributes.contains_key(key),
            "projected turn span carries {key} (parity with the live span)"
        );
    }
    assert_eq!(
        turn.attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(100)
    );
    assert_eq!(
        turn.attributes["gen_ai.usage.output_tokens"],
        serde_json::json!(20)
    );
    // `gen_ai.request.model` is deliberately NOT value-asserted here: the roll-up
    // writes the raw handle, and the `ModelCallCompleted` that follows overwrites
    // the root with the provider-labeled `{provider_id}.{model}` form on both
    // paths. The journal carries no `provider_id` (§2a), so this is `".gpt-4"`
    // here against `"openai.gpt-4"` live — a *value* difference the key-only
    // parity signature does not see, and the reason that key was never among the
    // four the shadow check reported missing.
}

/// The roll-up is cumulative across a turn's model calls, matching the live
/// bridge's running `BridgeState` totals rather than reporting the last call.
#[test]
fn turn_cost_rollup_accumulates_across_model_calls() {
    let spans = spans_from_observations(
        ctx(),
        10,
        &turn_with_usage(&[Usage::new(100, 20), Usage::new(50, 5)]),
    );
    let turn = turn_span(&spans);
    assert_eq!(
        turn.attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(150)
    );
    assert_eq!(
        turn.attributes["gen_ai.usage.output_tokens"],
        serde_json::json!(25)
    );
}

/// The observe-only crate `BudgetMiddleware` re-emits `UsageRecorded` for every
/// model call, so the journal holds TWO identical events per call with distinct
/// event ids. The live bridge dedupes on the iteration cursor; without the same
/// guard here every projected total would be doubled.
#[test]
fn duplicate_usage_recorded_for_one_call_is_folded_once() {
    let mut observations = turn_with_usage(&[Usage::new(100, 20)]);
    // Re-emit the middleware's duplicate immediately after the loop's own event,
    // with a distinct offset (and therefore a distinct event id).
    observations.insert(
        3,
        obs(
            99,
            1_035,
            AgentEvent::UsageRecorded {
                usage: Usage::new(100, 20),
            },
        ),
    );

    let spans = spans_from_observations(ctx(), 10, &observations);
    let turn = turn_span(&spans);
    assert_eq!(
        turn.attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(100),
        "the middleware's duplicate must not double the roll-up"
    );
    assert_eq!(
        turn.attributes["gen_ai.usage.output_tokens"],
        serde_json::json!(20)
    );
}

/// Live, a child run has its own bridge instance and its own accumulator, and
/// the per-child `TurnCostUpdated` is suppressed. The journal interleaves both
/// runs into one stream, so the projection must skip child usage outright or the
/// parent turn would over-report.
#[test]
fn subagent_usage_does_not_roll_into_the_parent_turn() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::UsageRecorded {
                usage: Usage::new(100, 20),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            4,
            1_040,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c2"),
                model: "gpt-4-mini".to_string(),
            },
        ),
        obs(
            5,
            1_050,
            AgentEvent::UsageRecorded {
                usage: Usage::new(7_000, 900),
            },
        ),
        obs(
            6,
            1_060,
            AgentEvent::SubAgentCompleted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            7,
            1_070,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ];

    let spans = spans_from_observations(ctx(), 10, &observations);
    let turn = turn_span(&spans);
    assert_eq!(
        turn.attributes["gen_ai.usage.input_tokens"],
        serde_json::json!(100),
        "the child's 7000 input tokens stay out of the parent roll-up"
    );
    assert_eq!(
        turn.attributes["gen_ai.request.model"],
        serde_json::json!("gpt-4"),
        "the child's model must not rename the parent turn"
    );
}

/// `journal_projection` used to hardcode `cache_creation_tokens: 0` even though
/// the crate `Usage` carries the real value. `record_model_call` inserts that
/// attribute only when `> 0`, so the zero dropped the key from every projected
/// generation span whenever a provider reported a cache write.
#[test]
fn cache_creation_tokens_reach_the_projected_generation_span() {
    let usage = Usage {
        cache_creation_tokens: 512,
        ..Usage::new(100, 20)
    };
    let spans = spans_from_observations(ctx(), 10, &turn_with_usage(&[usage]));
    let generation = spans
        .iter()
        .find(|s| s.kind == SpanKind::Generation)
        .expect("a generation span");
    assert_eq!(
        generation.attributes["gen_ai.usage.cache_creation_tokens"],
        serde_json::json!(512)
    );
}

/// #4118: the crate recovers an unavailable tool call without emitting
/// `ToolStarted`/`ToolCompleted`, and the live bridge synthesises the pair. The
/// projection had no arm, so it was short a whole tool span — a span *count*
/// divergence, not merely a missing attribute.
#[test]
fn unknown_tool_call_projects_a_failed_tool_span() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c1"),
                model: "gpt-4".to_string(),
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::UnknownToolCall {
                call_id: CallId::new("t1"),
                requested_name: "send_fax".to_string(),
                arguments: serde_json::json!({ "to": "1234" }),
                recovery: "rewrite:none".to_string(),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ];

    let spans = spans_from_observations(ctx(), 10, &observations);
    let tool = spans
        .iter()
        .find(|s| s.kind == SpanKind::Tool)
        .expect("the recovered call still produces a tool span");
    assert_eq!(tool.name, "tool.send_fax");
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(false));
    assert_eq!(tool.status, SpanStatus::Error);
}

/// The child-scope half of the `UnknownToolCall` projection: a sub-agent that
/// names an unavailable tool gets the same synthesised failed-call pair, nested
/// under its subagent span rather than the root turn.
#[test]
fn unknown_tool_call_inside_a_subagent_projects_a_child_tool_span() {
    let observations = vec![
        obs(
            0,
            1_000,
            AgentEvent::RunStarted {
                run_id: RunId::new("run-1"),
                thread_id: None,
            },
        ),
        obs(
            1,
            1_010,
            AgentEvent::SubAgentStarted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            2,
            1_020,
            AgentEvent::ModelStarted {
                call_id: CallId::new("c1"),
                model: "gpt-4-mini".to_string(),
            },
        ),
        obs(
            3,
            1_030,
            AgentEvent::UnknownToolCall {
                call_id: CallId::new("t1"),
                requested_name: "send_fax".to_string(),
                arguments: serde_json::json!({ "to": "1234" }),
                recovery: "rewrite:none".to_string(),
            },
        ),
        obs(
            4,
            1_040,
            AgentEvent::SubAgentCompleted {
                name: "researcher".to_string(),
                depth: 1,
            },
        ),
        obs(
            5,
            1_050,
            AgentEvent::RunCompleted {
                run_id: RunId::new("run-1"),
            },
        ),
    ];

    let spans = spans_from_observations(ctx(), 10, &observations);
    let subagent = spans
        .iter()
        .find(|s| s.kind == SpanKind::Subagent)
        .expect("a subagent span");
    let tool = spans
        .iter()
        .find(|s| s.kind == SpanKind::Tool)
        .expect("the recovered child call still produces a tool span");
    assert_eq!(tool.name, "tool.send_fax");
    assert_eq!(tool.attributes["tool.success"], serde_json::json!(false));
    assert_eq!(tool.status, SpanStatus::Error);
    assert_ne!(
        tool.parent_span_id.as_deref(),
        None,
        "the child tool span is parented inside the subagent scope"
    );
    assert!(
        spans.iter().any(|s| s.kind == SpanKind::SubagentIteration),
        "the child iteration span brackets it (subagent {})",
        subagent.span_id
    );
}

/// The exhaustive no-op arms: events that carry no span data must project to
/// nothing, leaving the span tree byte-identical to the same journal without
/// them. Guards against a future arm being moved out of the no-op group by
/// accident.
#[test]
fn non_span_bearing_events_project_to_no_spans() {
    let baseline = spans_from_observations(ctx(), 10, &turn_with_usage(&[Usage::new(100, 20)]));

    let mut noisy = turn_with_usage(&[Usage::new(100, 20)]);
    noisy.insert(
        2,
        obs(
            90,
            1_015,
            AgentEvent::CacheHit {
                call_id: CallId::new("c1"),
                key: "k".to_string(),
            },
        ),
    );
    noisy.insert(3, obs(91, 1_016, AgentEvent::MemoryLoaded));
    noisy.insert(4, obs(92, 1_017, AgentEvent::StreamClosed));
    let projected = spans_from_observations(ctx(), 10, &noisy);

    assert_eq!(
        projected.len(),
        baseline.len(),
        "diagnostic events add no spans"
    );
    let kinds = |spans: &[super::TraceSpan]| spans.iter().map(|s| s.kind).collect::<Vec<_>>();
    assert_eq!(kinds(&projected), kinds(&baseline));
}
