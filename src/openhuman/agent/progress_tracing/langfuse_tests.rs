use super::*;
use std::collections::BTreeMap;

#[test]
fn split_ingestion_batch_chunks_over_the_limit() {
    // 1201 events with max 500 -> chunks of 500, 500, 201.
    let events: Vec<Value> = (0..1201).map(|i| json!({ "id": i })).collect();
    let payload = json!({ "batch": events, "metadata": { "k": "v" } });

    let parts = split_ingestion_batch(payload, 500);
    assert_eq!(parts.len(), 3);
    let sizes: Vec<usize> = parts
        .iter()
        .map(|p| p["batch"].as_array().unwrap().len())
        .collect();
    assert_eq!(sizes, vec![500, 500, 201]);
    // Every chunk stays within the limit and preserves other top-level keys.
    for p in &parts {
        assert!(p["batch"].as_array().unwrap().len() <= 500);
        assert_eq!(p["metadata"]["k"], json!("v"));
    }
    // The first event of the run (e.g. the trace-create) lands in chunk 0.
    assert_eq!(parts[0]["batch"][0]["id"], json!(0));
    // Order is preserved across the split.
    assert_eq!(parts[2]["batch"][0]["id"], json!(1000));
}

#[test]
fn split_ingestion_batch_passes_small_payloads_through() {
    let payload = json!({ "batch": [json!({ "id": 1 }), json!({ "id": 2 })] });
    let parts = split_ingestion_batch(payload.clone(), 500);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0], payload);
    // A payload without a `batch` array is returned untouched.
    let no_batch = json!({ "hello": "world" });
    assert_eq!(split_ingestion_batch(no_batch.clone(), 500), vec![no_batch]);
}

use crate::openhuman::agent::progress_tracing::SpanKind;
use tinyagents::harness::ids::{CallId, EventId, RunId};
use tinyagents::harness::usage::Usage;

fn span(
    trace: &str,
    id: &str,
    parent: Option<&str>,
    name: &str,
    kind: SpanKind,
    status: SpanStatus,
    start: u64,
    end: Option<u64>,
) -> TraceSpan {
    let mut attributes = BTreeMap::new();
    attributes.insert("tokens".to_string(), json!(42));
    TraceSpan {
        trace_id: trace.to_string(),
        span_id: id.to_string(),
        parent_span_id: parent.map(str::to_string),
        name: name.to_string(),
        kind,
        start_unix_ms: start,
        end_unix_ms: end,
        status,
        attributes,
        input: None,
        output: None,
    }
}

fn obs(offset: u64, event: AgentEvent) -> AgentObservation {
    AgentObservation {
        event_id: EventId::new(format!("run-1-evt-{offset}")),
        run_id: RunId::new("run-1"),
        parent_run_id: None,
        root_run_id: RunId::new("run-1"),
        offset,
        ts_ms: 1_000 + offset,
        event,
    }
}

// ── production push gate (#5602) ──────────────────────────────
//
// The client already knew which environment it was in — `environment_for_base`
// has returned "production" for a prod host since it was written — and pushed
// anyway, paying an authenticated round-trip per agent turn bounded by the
// 10s `PUSH_TIMEOUT`. These pin that it now skips instead.

#[test]
fn push_is_allowed_only_in_staging_and_development() {
    assert!(push_allowed("staging"));
    assert!(push_allowed("development"));
    assert!(!push_allowed("production"));
}

#[test]
fn an_unrecognised_environment_fails_closed() {
    // The allowlist, not a `!= "production"` negation, is what makes this
    // true: a bucket nobody has thought of yet does not push.
    for unknown in ["preview", "prod", "PRODUCTION", "", "qa"] {
        assert!(
            !push_allowed(unknown),
            "{unknown:?} must not push — the allowlist is the fail-closed guard"
        );
    }
}

#[test]
fn every_environment_for_base_bucket_is_classified_deliberately() {
    // Ties the two functions together: if `environment_for_base` grows a
    // bucket, this fails until someone decides which side it belongs on.
    assert!(push_allowed(environment_for_base(
        "https://staging-api.tinyhumans.ai"
    )));
    assert!(push_allowed(environment_for_base("http://localhost:7788")));
    assert!(!push_allowed(environment_for_base(
        "https://api.tinyhumans.ai"
    )));
}

#[tokio::test]
async fn push_spans_skips_production_without_a_session_or_a_request() {
    // No live session is seeded here, so reaching `require_live_session_token`
    // would return `Err`. `Ok(())` therefore proves the gate returned before
    // it — i.e. before any credential work, and before any network call.
    let mut config = Config::default();
    config.api_url = Some("https://api.tinyhumans.ai/api/v1".to_string());
    assert_eq!(environment_for_base(&ingestion_url(&config)), "production");

    let spans = vec![span(
        "trace:req-1",
        "span-1",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    )];

    assert_eq!(
        push_spans(&config, &spans).await,
        Ok(()),
        "a production push must be a silent no-op, not an error the caller \
         logs on every turn"
    );
}

#[tokio::test]
async fn push_observations_skips_production_too() {
    let mut config = Config::default();
    config.api_url = Some("https://api.tinyhumans.ai/api/v1".to_string());
    let ctx = TraceContext::new("trace:req-1", Some("user-1".to_string()));
    let observations = vec![obs(
        1,
        AgentEvent::ModelCompleted {
            call_id: CallId::new("model-1"),
            started_at_ms: Some(1_000),
            usage: Some(Usage::new(10, 3)),
            input: None,
            output: None,
        },
    )];

    assert_eq!(
        push_observations(&config, &ctx, &observations, None).await,
        Ok(())
    );
}

#[tokio::test]
async fn an_unresolvable_host_skips_rather_than_erroring() {
    // `ingestion_url` on a base it cannot parse lands in the catch-all
    // bucket, which is production — so the gate swallows it first. Better
    // than the previous `Err`, which the caller logged every turn.
    let mut config = Config::default();
    config.api_url = Some("not a url".to_string());

    let spans = vec![span(
        "trace:req-1",
        "span-1",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    )];
    assert_eq!(push_spans(&config, &spans).await, Ok(()));
}

#[test]
fn ingestion_url_uses_backend_origin_and_ingestion_path() {
    let mut config = Config::default();
    config.api_url = Some("https://staging-api.tinyhumans.ai/api/v1".to_string());
    assert_eq!(
        ingestion_url(&config),
        "https://staging-api.tinyhumans.ai/telemetry/langfuse/ingestion",
        "endpoint is the backend's Langfuse proxy route on the base server \
         host, replacing any inference path the base carried"
    );

    // A base carrying an inference path resolves to the proxy route on the
    // SAME host — the ingestion host tracks the base server URL, not a fixed
    // literal.
    let mut with_inference_path = Config::default();
    with_inference_path.api_url =
        Some("https://api.tinyhumans.ai/openai/v1/chat/completions".to_string());
    assert_eq!(
        ingestion_url(&with_inference_path),
        "https://api.tinyhumans.ai/telemetry/langfuse/ingestion"
    );
}

#[test]
fn trace_config_from_context_matches_span_trace_attribution() {
    let ctx = TraceContext::new("trace:req-1", Some("user-1".to_string()))
        .with_session_group("thread-abc")
        .with_client_id("socket-abc")
        .with_agent_id("researcher")
        .with_channel_source("chat")
        .with_run_type(crate::openhuman::agent::progress_tracing::RunType::InteractiveChat);

    let trace = trace_config_from_context(&ctx, "staging");
    assert_eq!(trace.trace_id.as_deref(), Some("trace:req-1"));
    assert_eq!(trace.name.as_deref(), Some("agent.turn:researcher"));
    assert_eq!(trace.user_id.as_deref(), Some("user-1"));
    assert_eq!(trace.session_id.as_deref(), Some("thread-abc"));
    assert_eq!(trace.environment.as_deref(), Some("staging"));
    assert_eq!(trace.tags, vec!["run:interactive_chat", "source:chat"]);
    assert_eq!(trace.metadata["client.id"], "socket-abc");
    assert_eq!(trace.metadata["agent.id"], "researcher");
    assert_eq!(trace.metadata["channel.source"], "chat");
    assert_eq!(trace.metadata["run_type"], "interactive_chat");
    assert_eq!(trace.metadata["app.version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn trace_config_from_context_stamps_run_lineage() {
    // A spawned sub-agent: its run has a parent (the spawning turn) and a
    // root. Stamping these onto trace metadata is what links the sub-agent's
    // Langfuse trace back to the parent turn (#4657).
    let ctx = TraceContext::new("trace:req-1", None).with_run_lineage(
        Some("sub-run".to_string()),
        Some("parent-run".to_string()),
        Some("root-run".to_string()),
    );
    let trace = trace_config_from_context(&ctx, "staging");
    assert_eq!(trace.metadata["run_id"], "sub-run");
    assert_eq!(trace.metadata["parent_run_id"], "parent-run");
    assert_eq!(trace.metadata["root_run_id"], "root-run");
}

#[test]
fn trace_config_omits_parent_run_id_for_top_level_turn() {
    // A top-level turn has no parent; the key must stay absent (root == run).
    let ctx = TraceContext::new("trace:req-1", None).with_run_lineage(
        Some("run-1".to_string()),
        None,
        Some("run-1".to_string()),
    );
    let trace = trace_config_from_context(&ctx, "staging");
    assert_eq!(trace.metadata["run_id"], "run-1");
    assert_eq!(trace.metadata["root_run_id"], "run-1");
    assert!(
        trace.metadata.get("parent_run_id").is_none(),
        "top-level turn must not carry a parent_run_id"
    );
}

#[test]
fn trace_ctx_with_run_lineage_derives_from_subagent_observations() {
    // Sub-agent observations carry parent/root ids pointing at the spawning
    // turn; the derived trace context stamps them so the sub-agent's trace
    // links back instead of landing as a disconnected sibling (#4657).
    let observations = vec![AgentObservation {
        event_id: EventId::new("evt-1"),
        run_id: RunId::new("sub-run"),
        parent_run_id: Some(RunId::new("parent-run")),
        root_run_id: RunId::new("root-run"),
        offset: 1,
        ts_ms: 1_000,
        event: AgentEvent::ModelCompleted {
            call_id: CallId::new("model-1"),
            started_at_ms: Some(1_000),
            usage: Some(Usage::new(1, 1)),
            input: None,
            output: None,
        },
    }];
    let base = TraceContext::new("trace:req-1", None);

    let enriched = trace_ctx_with_run_lineage(&base, &observations);
    assert_eq!(enriched.run_id.as_deref(), Some("sub-run"));
    assert_eq!(enriched.parent_run_id.as_deref(), Some("parent-run"));
    assert_eq!(enriched.root_run_id.as_deref(), Some("root-run"));

    // An empty stream leaves the context untouched (no lineage invented).
    let untouched = trace_ctx_with_run_lineage(&base, &[]);
    assert!(untouched.run_id.is_none());
    assert!(untouched.parent_run_id.is_none());
    assert!(untouched.root_run_id.is_none());
}

#[test]
fn journal_observation_content_follows_capture_gate() {
    let observations = vec![
        obs(
            1,
            AgentEvent::ModelCompleted {
                call_id: CallId::new("model-1"),
                started_at_ms: Some(1_000),
                usage: Some(Usage::new(10, 3)),
                input: Some(json!([{"role": "user", "content": "secret prompt"}])),
                output: Some(json!({"role": "assistant", "content": "secret reply"})),
            },
        ),
        obs(
            2,
            AgentEvent::ToolCompleted {
                call_id: CallId::new("tool-1"),
                tool_name: "search".to_string(),
                started_at_ms: Some(1_010),
                input: Some(json!({"query": "secret"})),
                output: Some(json!("secret result")),
                duration_ms: Some(20),
                output_bytes: Some(13),
                error: None,
            },
        ),
    ];

    let off_ctx = TraceContext::new("trace:req-1", None);
    let filtered = observations_for_export(&off_ctx, &observations);
    assert!(matches!(filtered, Cow::Owned(_)));
    match &filtered[0].event {
        AgentEvent::ModelCompleted { input, output, .. } => {
            assert!(input.is_none());
            assert!(output.is_none());
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match &filtered[1].event {
        AgentEvent::ToolCompleted { input, output, .. } => {
            assert!(input.is_none());
            assert!(output.is_none());
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match &observations[0].event {
        AgentEvent::ModelCompleted { input, output, .. } => {
            assert!(input.is_some(), "source journal observation stays intact");
            assert!(output.is_some(), "source journal observation stays intact");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let on_ctx = TraceContext::new("trace:req-1", None).with_capture_content(true);
    let passthrough = observations_for_export(&on_ctx, &observations);
    assert!(matches!(passthrough, Cow::Borrowed(_)));
    match &passthrough[1].event {
        AgentEvent::ToolCompleted { input, output, .. } => {
            assert_eq!(input.as_ref(), Some(&json!({"query": "secret"})));
            assert_eq!(output.as_ref(), Some(&json!("secret result")));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn run_telemetry_inserts_aggregate_generation() {
    let observations = vec![obs(
        1,
        AgentEvent::ModelCompleted {
            call_id: CallId::new("model-1"),
            started_at_ms: Some(1_000),
            usage: Some(Usage::new(100, 20)),
            input: None,
            output: None,
        },
    )];
    let client = LangfuseClient::proxy("https://api.tinyhumans.ai", "token").expect("proxy client");
    let trace = trace_config_from_context(&TraceContext::new("trace:req-1", None), "production");
    let mut payload = client
        .build_ingestion_batch(trace, &observations)
        .expect("batch");
    let telemetry = RunTelemetry {
        run_id: "req-1".to_string(),
        input_tokens: 120,
        output_tokens: 30,
        cached_input_tokens: 40,
        cost_usd: 0.0123,
        elapsed_ms: Some(900),
        tool_count: 2,
        model: Some("managed.chat-v1".to_string()),
        provider: Some("managed".to_string()),
        error: None,
        updated_at: None,
    };

    assert!(insert_run_telemetry_generation(
        &mut payload,
        Some(&telemetry)
    ));
    let batch = payload["batch"].as_array().expect("batch array");
    assert_eq!(batch[1]["type"], "generation-create");
    let body = &batch[1]["body"];
    assert_eq!(body["id"], "trace:req-1:openhuman-run-telemetry");
    assert_eq!(body["name"], "run.total");
    assert_eq!(body["traceId"], "trace:req-1");
    assert_eq!(body["model"], "managed.chat-v1");
    assert_eq!(body["usageDetails"]["input"], 80);
    assert_eq!(body["usageDetails"]["output"], 30);
    assert_eq!(body["usageDetails"]["total"], 150);
    assert_eq!(body["usageDetails"]["cache_read_input_tokens"], 40);
    assert_eq!(body["costDetails"]["total"], 0.0123);
    assert_eq!(body["metadata"]["source"], "openhuman.run_telemetry");
    assert_eq!(body["metadata"]["run_id"], "req-1");
    assert_eq!(body["metadata"]["tool_count"], 2);
    assert_eq!(body["metadata"]["provider"], "managed");
}

#[test]
fn iso_millis_formats_epoch_as_rfc3339() {
    // 2021-01-01T00:00:00Z = 1_609_459_200_000 ms.
    assert!(iso_millis(1_609_459_200_000).starts_with("2021-01-01T00:00:00"));
}

#[test]
fn batch_emits_trace_create_then_one_span_create_each() {
    let spans = vec![
        span(
            "trace-1",
            "root",
            None,
            "agent.turn",
            SpanKind::Turn,
            SpanStatus::Ok,
            1_000,
            Some(2_000),
        ),
        span(
            "trace-1",
            "tool-1",
            Some("root"),
            "tool.web_search",
            SpanKind::Tool,
            SpanStatus::Error,
            1_100,
            Some(1_500),
        ),
    ];
    let payload = spans_to_langfuse_batch(&spans, false, "production");
    let batch = payload["batch"].as_array().expect("batch array");
    assert_eq!(batch.len(), 3, "one trace-create + two span-create");

    assert_eq!(batch[0]["type"], "trace-create");
    assert_eq!(batch[0]["body"]["id"], "trace-1");

    // Camel-case Langfuse fields, ISO timestamps, parent linkage, error level.
    let root = &batch[1];
    assert_eq!(root["type"], "span-create");
    assert_eq!(root["body"]["id"], "root");
    assert_eq!(root["body"]["traceId"], "trace-1");
    assert!(root["body"]["startTime"].as_str().unwrap().contains('T'));
    assert_eq!(root["body"]["level"], "DEFAULT");
    assert_eq!(root["body"]["metadata"]["kind"], "turn");
    assert!(root["body"].get("parentObservationId").is_none());

    let tool = &batch[2];
    assert_eq!(tool["body"]["parentObservationId"], "root");
    assert_eq!(tool["body"]["level"], "ERROR");
    assert!(tool["body"]["endTime"].as_str().unwrap().contains('T'));

    // Event ids are unique and distinct from the observation ids.
    assert_ne!(batch[1]["id"], batch[2]["id"]);
    assert_ne!(batch[1]["id"], batch[1]["body"]["id"]);
}

#[test]
fn usage_span_becomes_generation_and_content_is_gated() {
    let mut turn = span(
        "trace-1",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.attributes.clear();
    turn.attributes
        .insert("gen_ai.request.model".into(), json!("claude-x"));
    turn.attributes
        .insert("gen_ai.usage.input_tokens".into(), json!(100));
    turn.attributes
        .insert("gen_ai.usage.output_tokens".into(), json!(20));
    turn.attributes
        .insert("gen_ai.usage.cost_usd".into(), json!(0.0123));
    turn.input = Some(json!("what is 2+2?"));
    turn.output = Some(json!("4"));
    let spans = vec![turn];

    // Content OFF (default): span is promoted to a generation with native
    // usage + cost, but prompt/reply are withheld.
    let off = spans_to_langfuse_batch(&spans, false, "production");
    let obs = &off["batch"][1];
    assert_eq!(obs["type"], "generation-create");
    assert_eq!(obs["body"]["model"], "claude-x");
    assert_eq!(obs["body"]["usageDetails"]["input"], 100);
    assert_eq!(obs["body"]["usageDetails"]["output"], 20);
    assert_eq!(obs["body"]["usageDetails"]["total"], 120);
    assert_eq!(obs["body"]["costDetails"]["total"], 0.0123);
    assert!(
        obs["body"].get("input").is_none(),
        "prompt must be withheld when capture_content is off"
    );
    assert!(obs["body"].get("output").is_none());

    // Content ON: prompt/reply included, usage/cost unchanged.
    let on = spans_to_langfuse_batch(&spans, true, "production");
    let obs = &on["batch"][1];
    assert_eq!(obs["type"], "generation-create");
    assert_eq!(obs["body"]["input"], "what is 2+2?");
    assert_eq!(obs["body"]["output"], "4");
    assert_eq!(obs["body"]["costDetails"]["total"], 0.0123);
}

#[test]
fn trace_create_carries_user_and_session_grouping() {
    // The turn span's user.id / thread.id attributes are promoted onto the
    // trace-create as Langfuse userId / sessionId so per-turn traces group
    // under one conversation and attribute to a user.
    let mut turn = span(
        "trace:req-1",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.attributes.insert("user.id".into(), json!("client-7"));
    turn.attributes
        .insert("thread.id".into(), json!("thread-abc"));
    let payload = spans_to_langfuse_batch(&[turn], false, "production");
    let trace = &payload["batch"][0];
    assert_eq!(trace["type"], "trace-create");
    assert_eq!(trace["body"]["userId"], "client-7");
    assert_eq!(trace["body"]["sessionId"], "thread-abc");
}

#[test]
fn trace_create_session_id_falls_back_to_trace_id() {
    // No thread.id attribute → the trace id itself becomes the sessionId,
    // so every trace lands with a session in Langfuse.
    let turn = span(
        "trace:req-2",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    let payload = spans_to_langfuse_batch(&[turn], false, "production");
    assert_eq!(payload["batch"][0]["body"]["sessionId"], "trace:req-2");
}

#[test]
fn trace_create_metadata_carries_attribution_and_version() {
    let mut turn = span(
        "trace-1",
        "root",
        None,
        "agent.turn:researcher",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.attributes
        .insert("client.id".into(), json!("socket-abc"));
    turn.attributes
        .insert("agent.id".into(), json!("researcher"));
    turn.attributes
        .insert("channel.source".into(), json!("chat"));
    let payload = spans_to_langfuse_batch(&[turn], false, "production");
    let trace = &payload["batch"][0]["body"];
    assert_eq!(trace["name"], "agent.turn:researcher");
    let meta = &trace["metadata"];
    assert_eq!(meta["client.id"], "socket-abc");
    assert_eq!(meta["agent.id"], "researcher");
    assert_eq!(meta["channel.source"], "chat");
    assert_eq!(meta["app.version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn trace_create_input_output_follow_content_gate() {
    let mut turn = span(
        "trace-1",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.input = Some(json!("the prompt"));
    turn.output = Some(json!("the reply"));
    let spans = vec![turn];

    let on = spans_to_langfuse_batch(&spans, true, "production");
    assert_eq!(on["batch"][0]["body"]["input"], "the prompt");
    assert_eq!(on["batch"][0]["body"]["output"], "the reply");

    let off = spans_to_langfuse_batch(&spans, false, "production");
    assert!(off["batch"][0]["body"].get("input").is_none());
    assert!(off["batch"][0]["body"].get("output").is_none());
}

#[test]
fn environment_derivation_from_backend_base() {
    assert_eq!(
        environment_for_base("https://staging-api.tinyhumans.ai"),
        "staging"
    );
    assert_eq!(environment_for_base("http://localhost:5000"), "development");
    assert_eq!(environment_for_base("http://127.0.0.1:5000"), "development");
    assert_eq!(
        environment_for_base("https://api.tinyhumans.ai"),
        "production"
    );
}

/// A hostname that merely *contains* `staging` is not ours. The classifier
/// gates whether a live session token leaves the process, so anything it
/// cannot positively recognise has to land on `production` — which does
/// not push.
#[test]
fn a_lookalike_staging_host_is_production_not_staging() {
    for base in [
        // The substring match this replaced classified all of these as
        // staging, and `push_allowed` would then have let them through.
        "https://staging-attacker.invalid",
        "https://staging.evil.example",
        "http://staging-api.tinyhumans.ai.evil.example",
        // Right domain, wrong label position.
        "https://api-staging-mirror.tinyhumans.ai",
        // A public IP literal is never a deployment of ours.
        "https://93.184.216.34",
    ] {
        let environment = environment_for_base(base);
        assert_eq!(
            environment, "production",
            "{base} must classify as production, got {environment}"
        );
        assert!(!push_allowed(environment), "{base} must not be pushable");
    }
}

/// The local buckets the substring form missed. An IPv6 loopback backend
/// is an ordinary local setup, and before the parse it classified as
/// production — so turning the push gate on would have silently stopped
/// exports that had been working.
#[test]
fn local_backends_are_development_including_ipv6_and_private_ranges() {
    for base in [
        "http://[::1]:7788",
        "http://[0:0:0:0:0:0:0:1]:7788",
        "http://[::]:7788",
        "http://192.168.1.20:5000",
        "http://10.0.0.5:5000",
        "http://api.localhost:5000",
        "http://0.0.0.0:5000",
    ] {
        let environment = environment_for_base(base);
        assert_eq!(
            environment, "development",
            "{base} must classify as development, got {environment}"
        );
        assert!(push_allowed(environment), "{base} must stay pushable");
    }
}

/// An unparseable base is the fail-closed default rather than a panic or a
/// pushable bucket. `ingestion_url` returns a non-URL placeholder when no
/// backend host resolves.
#[test]
fn an_unparseable_base_is_production() {
    for base in ["", "not a url", "/api/v1/ingestion"] {
        assert_eq!(
            environment_for_base(base),
            "production",
            "{base:?} must fail closed"
        );
    }
}

#[test]
fn trace_create_carries_environment_release_and_run_tags() {
    let mut turn = span(
        "trace-1",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.attributes
        .insert("run.type".into(), json!("autonomous_task"));
    turn.attributes
        .insert("channel.source".into(), json!("autonomous"));
    let payload = spans_to_langfuse_batch(&[turn], false, "staging");
    let trace = &payload["batch"][0]["body"];
    // Top-level Langfuse trace fields, not metadata.
    assert_eq!(trace["environment"], "staging");
    assert_eq!(trace["release"], env!("CARGO_PKG_VERSION"));
    // Filterable run tags + run_type metadata.
    assert_eq!(
        trace["tags"],
        json!(["run:autonomous_task", "source:autonomous"])
    );
    assert_eq!(trace["metadata"]["run_type"], "autonomous_task");
}

#[test]
fn interactive_chat_trace_gets_interactive_run_tag() {
    let mut turn = span(
        "trace-1",
        "root",
        None,
        "agent.turn",
        SpanKind::Turn,
        SpanStatus::Ok,
        1_000,
        Some(2_000),
    );
    turn.attributes
        .insert("run.type".into(), json!("interactive_chat"));
    turn.attributes
        .insert("channel.source".into(), json!("chat"));
    let payload = spans_to_langfuse_batch(&[turn], false, "production");
    let trace = &payload["batch"][0]["body"];
    assert_eq!(
        trace["tags"],
        json!(["run:interactive_chat", "source:chat"])
    );
    assert_eq!(trace["metadata"]["run_type"], "interactive_chat");
}

#[test]
fn generation_usage_details_map_reasoning_and_cache_tokens() {
    let mut gen = span(
        "trace-1",
        "gen-1",
        Some("root"),
        "llm.agentic-v1",
        SpanKind::Generation,
        SpanStatus::Ok,
        1_000,
        Some(1_500),
    );
    gen.attributes.clear();
    gen.attributes
        .insert("gen_ai.request.model".into(), json!("agentic-v1"));
    gen.attributes
        .insert("gen_ai.usage.input_tokens".into(), json!(1_000));
    gen.attributes
        .insert("gen_ai.usage.output_tokens".into(), json!(200));
    gen.attributes
        .insert("gen_ai.usage.cached_input_tokens".into(), json!(0));
    gen.attributes
        .insert("gen_ai.usage.reasoning_tokens".into(), json!(128));
    gen.attributes
        .insert("gen_ai.usage.cache_creation_tokens".into(), json!(64));
    gen.attributes
        .insert("gen_ai.usage.cost_usd".into(), json!(0.0042));
    gen.attributes
        .insert("gen_ai.provider".into(), json!("managed"));

    let payload = spans_to_langfuse_batch(&[gen], false, "production");
    let obs = &payload["batch"][1];
    assert_eq!(obs["type"], "generation-create");
    let usage = &obs["body"]["usageDetails"];
    assert_eq!(usage["input"], 1_000);
    assert_eq!(usage["output"], 200);
    // Cache reads always flow, even at 0.
    assert_eq!(usage["cache_read_input_tokens"], 0);
    assert_eq!(usage["reasoning_tokens"], 128);
    assert_eq!(usage["cache_creation_input_tokens"], 64);
    assert_eq!(obs["body"]["costDetails"]["total"], 0.0042);
    // Provenance rides in observation metadata.
    assert_eq!(obs["body"]["metadata"]["gen_ai.provider"], "managed");
}

#[test]
fn generation_without_reasoning_or_cache_write_omits_those_usage_keys() {
    let mut gen = span(
        "trace-1",
        "gen-1",
        Some("root"),
        "llm.agentic-v1",
        SpanKind::Generation,
        SpanStatus::Ok,
        1_000,
        Some(1_500),
    );
    gen.attributes.clear();
    gen.attributes
        .insert("gen_ai.usage.input_tokens".into(), json!(10));
    gen.attributes
        .insert("gen_ai.usage.output_tokens".into(), json!(5));
    let payload = spans_to_langfuse_batch(&[gen], false, "production");
    let usage = &payload["batch"][1]["body"]["usageDetails"];
    assert_eq!(
        usage["cache_read_input_tokens"], 0,
        "cache reads always present"
    );
    assert!(usage.get("reasoning_tokens").is_none());
    assert!(usage.get("cache_creation_input_tokens").is_none());
}

#[test]
fn error_span_gets_error_level_and_status_message() {
    let mut tool = span(
        "trace-1",
        "tool-1",
        Some("root"),
        "tool.shell",
        SpanKind::Tool,
        SpanStatus::Error,
        1_000,
        Some(1_200),
    );
    tool.attributes
        .insert("error.message".into(), json!("The command timed out"));
    let payload = spans_to_langfuse_batch(&[tool], false, "production");
    let obs = &payload["batch"][1]["body"];
    assert_eq!(obs["level"], "ERROR");
    assert_eq!(obs["statusMessage"], "The command timed out");

    // Without a captured message: ERROR level, no statusMessage.
    let bare = span(
        "trace-1",
        "tool-2",
        Some("root"),
        "tool.shell",
        SpanKind::Tool,
        SpanStatus::Error,
        1_000,
        Some(1_200),
    );
    let payload = spans_to_langfuse_batch(&[bare], false, "production");
    let obs = &payload["batch"][1]["body"];
    assert_eq!(obs["level"], "ERROR");
    assert!(obs.get("statusMessage").is_none());
}

#[tokio::test]
async fn empty_spans_push_is_ok_noop() {
    let config = Config::default();
    // Empty batch short-circuits before any host/token resolution or network.
    assert!(push_spans(&config, &[]).await.is_ok());
}
