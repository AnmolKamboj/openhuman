use super::*;
use tinyagents::harness::no_progress::{
    DEFAULT_REPEAT_CALL_THRESHOLD, DEFAULT_REPEAT_OUTPUT_THRESHOLD,
};

// #4462: image-aware token estimation. A base64 image marker must be priced
// at the flat IMAGE_MARKER_TOKEN_COST, not chars/4 of its payload — otherwise
// one image reads as millions of tokens and the trimmer evicts everything,
// including the system prompt.

#[test]
fn estimate_text_tokens_markerless_is_chars_over_four() {
    assert_eq!(estimate_text_tokens(&"a".repeat(40)), (40 + 3) / 4);
    assert_eq!(estimate_text_tokens(""), 0);
}

#[test]
fn estimate_text_tokens_prices_image_marker_flat_not_by_length() {
    let huge = "x".repeat(40_000);
    let text = format!("[IMAGE:{huge}]");
    let tokens = estimate_text_tokens(&text);
    // chars/4 of the payload would be ~10_000; the flat price is 1_200.
    assert!(
        tokens >= IMAGE_MARKER_TOKEN_COST,
        "at least the flat image cost: {tokens}"
    );
    assert!(
        tokens < 2_000,
        "image priced flat, not by base64 length: {tokens}"
    );
}

#[test]
fn estimate_text_tokens_charges_each_image_marker_once() {
    let tokens = estimate_text_tokens("[IMAGE:aaaa] and [IMAGE:bbbb]");
    assert!(
        tokens >= 2 * IMAGE_MARKER_TOKEN_COST,
        "two images each priced: {tokens}"
    );
    assert!(
        tokens < 2 * IMAGE_MARKER_TOKEN_COST + 100,
        "no runaway from the surrounding text: {tokens}"
    );
}
use serde_json::json;
use tinyagents::harness::context::{RunConfig, RunContext};
use tinyagents::harness::model::ModelRequest;

fn ctx() -> RunContext<()> {
    RunContext::new(RunConfig::new("mw-test"), ())
}

// ── payload_summarizer disclosure (#5722) ──────────────────────
//
// The behaviour these pin: when summarization does not happen, the model
// must be able to see that from the payload. Previously every one of
// these cases produced byte-identical content to a successful
// pass-through, so the model could not tell a raw dump from a normal
// result and re-called the same tool.

struct StubSummarizer(std::sync::Mutex<Option<anyhow::Result<SummarizeOutcome>>>);

impl StubSummarizer {
    fn ok(outcome: SummarizeOutcome) -> Arc<Self> {
        Arc::new(Self(std::sync::Mutex::new(Some(Ok(outcome)))))
    }
}

#[async_trait]
impl PayloadSummarizer for StubSummarizer {
    async fn maybe_summarize_in_parent(
        &self,
        _parent_ctx: &RunContext<()>,
        _tool_name: &str,
        _parent_task_hint: Option<&str>,
        _raw: &str,
    ) -> anyhow::Result<SummarizeOutcome> {
        self.0
            .lock()
            .expect("stub outcome lock")
            .take()
            .expect("stub summarizer called more than once")
    }
}

fn summarizer_mw(ps: Arc<dyn PayloadSummarizer>) -> ToolOutputMiddleware {
    ToolOutputMiddleware {
        // Large enough that the byte-budget backstop never fires, so these
        // tests observe the summarizer stage alone.
        budget_bytes: 10_000_000,
        payload_summarizer: Some(ps),
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression:
            crate::openhuman::inference::tokenjuice::AgentTokenjuiceCompression::Off,
        tool_policies: HashMap::new(),
    }
}

#[tokio::test]
async fn unavailable_summarization_is_disclosed_in_the_payload() {
    let mw = summarizer_mw(StubSummarizer::ok(SummarizeOutcome::Unavailable(
        UnavailableReason::Failed,
    )));
    let mut ctx = ctx();
    let mut result = tool_result("test_tool", "RAW-TOOL-OUTPUT");

    mw.after_tool(&mut ctx, &(), &mut result)
        .await
        .expect("after_tool should not fail");

    assert!(
        result
            .content
            .starts_with(UnavailableReason::Failed.notice()),
        "the notice must be a PREFIX — the downstream per-tool cap keeps the \
         head, so an appended notice is the first thing truncated away; got: {}",
        result.content
    );
    assert!(
        result.content.contains("RAW-TOOL-OUTPUT"),
        "disclosure must not cost the payload: {}",
        result.content
    );
}

#[tokio::test]
async fn a_payload_that_needed_nothing_is_left_completely_alone() {
    // The other half of the contract. If every result carried a notice the
    // marker would be noise and the model would learn to ignore it.
    let mw = summarizer_mw(StubSummarizer::ok(SummarizeOutcome::NotNeeded));
    let mut ctx = ctx();
    let mut result = tool_result("test_tool", "RAW-TOOL-OUTPUT");

    mw.after_tool(&mut ctx, &(), &mut result)
        .await
        .expect("after_tool should not fail");

    assert_eq!(
        result.content, "RAW-TOOL-OUTPUT",
        "a below-threshold payload must be byte-identical"
    );
}

#[tokio::test]
async fn a_summarizer_error_is_disclosed_rather_than_swallowed() {
    // `Err` used to be discarded by the same `if let Ok(Some(..))` that
    // discarded `None`, so a fatal misconfiguration was indistinguishable
    // from "nothing to do".
    struct ErroringSummarizer;
    #[async_trait]
    impl PayloadSummarizer for ErroringSummarizer {
        async fn maybe_summarize_in_parent(
            &self,
            _parent_ctx: &RunContext<()>,
            _tool_name: &str,
            _parent_task_hint: Option<&str>,
            _raw: &str,
        ) -> anyhow::Result<SummarizeOutcome> {
            Err(anyhow::anyhow!("summarizer misconfigured"))
        }
    }

    let mw = summarizer_mw(Arc::new(ErroringSummarizer));
    let mut ctx = ctx();
    let mut result = tool_result("test_tool", "RAW-TOOL-OUTPUT");

    mw.after_tool(&mut ctx, &(), &mut result)
        .await
        .expect("a summarizer error must never break the tool call");

    assert!(
        result
            .content
            .starts_with(UnavailableReason::Failed.notice()),
        "an errored summarizer must be disclosed too; got: {}",
        result.content
    );
}

#[tokio::test]
async fn prompt_cache_segments_fingerprint_full_tool_schema() {
    let mw = PromptCacheSegmentMiddleware;
    let mut first =
        ModelRequest::new(vec![TaMessage::system("sys")]).with_tools(vec![ToolSchema::new(
            "lookup",
            "lookup a user",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
            }),
        )]);
    mw.before_model(&mut ctx(), &(), &mut first).await.unwrap();

    let mut second =
        ModelRequest::new(vec![TaMessage::system("sys")]).with_tools(vec![ToolSchema::new(
            "lookup",
            "lookup a user",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                },
            }),
        )]);
    mw.before_model(&mut ctx(), &(), &mut second).await.unwrap();

    let first_tool_segment = first
        .cache_segments
        .iter()
        .find(|segment| segment.role == SegmentRole::Tools)
        .expect("tool segment");
    let second_tool_segment = second
        .cache_segments
        .iter()
        .find(|segment| segment.role == SegmentRole::Tools)
        .expect("tool segment");

    assert_ne!(
        first_tool_segment.id, second_tool_segment.id,
        "same-name tools with different schemas must bust the stable prefix"
    );
    assert_ne!(first.prompt_fingerprint, second.prompt_fingerprint);
    assert_eq!(
        first.prompt_fingerprint.as_deref().unwrap().len(),
        64,
        "request prompt fingerprints use TinyAgents' SHA-256 shape"
    );
}

/// A minimal openhuman [`Tool`] for the tool-set–backed middlewares. Its
/// `max_result_size_chars` and `external_effect` are configurable so the
/// budget/approval resolution paths can be exercised.
struct FakeTool {
    name: &'static str,
    cap: Option<usize>,
    external: bool,
}

#[async_trait]
impl Tool for FakeTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "fake"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({ "type": "object" })
    }
    async fn execute(
        &self,
        _args: serde_json::Value,
    ) -> anyhow::Result<crate::openhuman::tools::ToolResult> {
        Ok(crate::openhuman::tools::ToolResult::success("ok"))
    }
    fn max_result_size_chars(&self) -> Option<usize> {
        self.cap
    }
    fn external_effect_with_args(&self, _args: &serde_json::Value) -> bool {
        self.external
    }
}

fn tool_result(name: &str, content: &str) -> TaToolResult {
    TaToolResult {
        call_id: "c1".into(),
        name: name.into(),
        content: content.into(),
        raw: None,
        error: None,
        elapsed_ms: 0,
    }
}

// ── ToolOutcomeCaptureMiddleware policy-block enrichment (issue #4094) ───

fn outcome_capture_mw() -> ToolOutcomeCaptureMiddleware {
    ToolOutcomeCaptureMiddleware::new(
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    )
}

#[tokio::test]
async fn raw_security_policy_block_is_enriched_with_workaround_and_relay() {
    let mw = outcome_capture_mw();
    let mut result = tool_result(
        "run_command",
        "[policy-blocked] Security policy: read-only mode — only read commands are allowed",
    );
    result.error = Some(result.content.clone());
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    // The bare denial now carries a workaround + relay directive, and keeps the
    // marker so classification / the loop-breaker still recognise it.
    assert!(result.content.contains("Workaround:"), "{}", result.content);
    assert!(result.content.contains("Relay this to the user"));
    assert!(result
        .content
        .contains(crate::openhuman::security::POLICY_BLOCKED_MARKER));
    assert!(result.content.contains("read-only mode"));
}

#[tokio::test]
async fn already_structured_denial_is_not_double_wrapped() {
    // A ToolPolicyMiddleware-style denial already has "Workaround:"; the capture
    // middleware must leave it untouched (no second Workaround block).
    let mw = outcome_capture_mw();
    let structured =
        "Blocked: Tool 'x' denied. Reason: nope. Workaround: do y. Relay this to the user: ...";
    let mut result = tool_result("x", structured);
    result.error = Some(result.content.clone());
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(
        result.content.matches("Workaround:").count(),
        1,
        "must not double-wrap: {}",
        result.content
    );
}

// ── TurnContextMiddleware config ────────────────────────────────────────

#[test]
fn defaults_enable_the_byte_cap_only() {
    let mw = TurnContextMiddleware::defaults();
    assert_eq!(
        mw.tool_result_budget_bytes,
        DEFAULT_TOOL_RESULT_BUDGET_BYTES
    );
    assert!(mw.payload_summarizer.is_none());
    assert_eq!(mw.microcompact_keep_recent, 0);
    // Autocompaction defaults on (channel/sub-agent); the chat path overrides
    // it from config.
    assert!(mw.autocompact_enabled);
    // The byte cap alone is enough to make the bundle non-empty (CacheAlign
    // was deleted in C3, so it no longer contributes here).
    assert!(!mw.is_empty());
}

#[test]
fn an_all_default_bundle_installs_nothing() {
    assert!(TurnContextMiddleware::default().is_empty());
}

#[test]
fn tokenjuice_only_bundle_is_not_empty() {
    let mw = TurnContextMiddleware {
        tokenjuice_compaction_enabled: true,
        tokenjuice_compression: AgentTokenjuiceCompression::Light,
        ..Default::default()
    };
    assert!(!mw.is_empty());
}

// ── MicrocompactMiddleware (crate) ──────────────────────────────────────
//
// These assert the crate `MicrocompactMiddleware`, constructed with
// OpenHuman's `CLEARED_PLACEHOLDER`, reproduces the deleted in-house
// middleware byte-for-byte — the parity contract for the upstream swap.

#[tokio::test]
async fn microcompact_clears_older_tool_bodies_and_keeps_recent() {
    let mw = MicrocompactMiddleware::new(1, CLEARED_PLACEHOLDER);
    let mut req = ModelRequest::new(vec![
        TaMessage::system("sys"),
        TaMessage::user("hello"),
        TaMessage::tool("t1", "FIRST_BODY"),
        TaMessage::assistant("thinking"),
        TaMessage::tool("t2", "SECOND_BODY"),
        TaMessage::tool("t3", "THIRD_BODY"),
    ]);

    mw.before_model(&mut ctx(), &(), &mut req).await.unwrap();

    // 3 tool messages, keep_recent=1 → the two oldest cleared, newest kept.
    assert_eq!(req.messages[2].text(), CLEARED_PLACEHOLDER);
    assert_eq!(req.messages[4].text(), CLEARED_PLACEHOLDER);
    assert_eq!(req.messages[5].text(), "THIRD_BODY");
    // Non-tool messages are never touched.
    assert_eq!(req.messages[0].text(), "sys");
    assert_eq!(req.messages[1].text(), "hello");
    assert_eq!(req.messages[3].text(), "thinking");
}

#[tokio::test]
async fn microcompact_is_a_noop_when_within_keep_recent() {
    let mw = MicrocompactMiddleware::new(5, CLEARED_PLACEHOLDER);
    let mut req = ModelRequest::new(vec![TaMessage::tool("t1", "A"), TaMessage::tool("t2", "B")]);
    mw.before_model(&mut ctx(), &(), &mut req).await.unwrap();
    assert_eq!(req.messages[0].text(), "A");
    assert_eq!(req.messages[1].text(), "B");
}

#[tokio::test]
async fn microcompact_is_idempotent() {
    let mw = MicrocompactMiddleware::new(1, CLEARED_PLACEHOLDER);
    let mut req = ModelRequest::new(vec![
        TaMessage::tool("t1", "FIRST"),
        TaMessage::tool("t2", "SECOND"),
    ]);
    mw.before_model(&mut ctx(), &(), &mut req).await.unwrap();
    let after_first = req.messages[0].text();
    assert_eq!(after_first, CLEARED_PLACEHOLDER);
    // Second pass leaves the already-cleared body as the placeholder.
    mw.before_model(&mut ctx(), &(), &mut req).await.unwrap();
    assert_eq!(req.messages[0].text(), CLEARED_PLACEHOLDER);
    assert_eq!(req.messages[1].text(), "SECOND");
}

// ── ToolOutputMiddleware ────────────────────────────────────────────────

#[tokio::test]
async fn tool_output_truncates_over_the_flat_budget() {
    let mw = ToolOutputMiddleware {
        budget_bytes: 100,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression: AgentTokenjuiceCompression::Off,
        tool_policies: HashMap::new(),
    };
    let mut result = tool_result("echo", &"x".repeat(5_000));
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert!(result.content.len() < 5_000, "content should be capped");
    assert!(
        result.content.contains("truncated by tool_result_budget"),
        "a truncation marker should be appended: {}",
        result.content
    );
}

#[tokio::test]
async fn tool_output_leaves_small_results_untouched() {
    let mw = ToolOutputMiddleware {
        budget_bytes: 1_000,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression: AgentTokenjuiceCompression::Off,
        tool_policies: HashMap::new(),
    };
    let mut result = tool_result("echo", "tiny");
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(result.content, "tiny");
}

#[test]
fn tool_char_cap_reads_the_tools_own_declared_cap() {
    let mut tool_policies = HashMap::new();
    tool_policies.insert(
        "big".to_string(),
        TaToolPolicy::classified().with_runtime(tinyagents::harness::tool::ToolRuntime {
            timeout_ms: None,
            timeout: tinyagents::harness::tool::ToolTimeout::Inherit,
            max_retries: None,
            idempotent: false,
            cancelable: true,
            sandbox: tinyagents::harness::tool::SandboxMode::Inherit,
            max_result_bytes: Some(10),
            streaming: false,
        }),
    );
    let mw = ToolOutputMiddleware {
        budget_bytes: 1_000,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression: AgentTokenjuiceCompression::Off,
        tool_policies,
    };
    // Tool declares its own char cap → surfaced for the per-tool truncation.
    assert_eq!(mw.tool_char_cap("big"), Some(10));
    // Unknown tool → no per-tool cap (the flat byte budget applies instead).
    assert_eq!(mw.tool_char_cap("other"), None);
}

/// openhuman#5722 review: the disclosure used to be prefixed *before* the
/// per-tool char cap, so a tool declaring a cap shorter than the notice had
/// `chars().take(cap)` slice through the notice itself — dropping the
/// reason and the do-not-re-run sentence, and leaving the model a truncated
/// fragment that still reads as tool output. The notice is applied after
/// every cap now, so it survives intact whatever the tool declared.
#[tokio::test]
async fn an_unavailable_notice_survives_a_tool_cap_shorter_than_itself() {
    let mut tool_policies = HashMap::new();
    tool_policies.insert(
        "terse".to_string(),
        TaToolPolicy::classified().with_runtime(tinyagents::harness::tool::ToolRuntime {
            timeout_ms: None,
            timeout: tinyagents::harness::tool::ToolTimeout::Inherit,
            max_retries: None,
            idempotent: false,
            cancelable: true,
            sandbox: tinyagents::harness::tool::SandboxMode::Inherit,
            // Far shorter than the ~165-char notice.
            max_result_bytes: Some(12),
            streaming: false,
        }),
    );
    let mw = ToolOutputMiddleware {
        // Large enough that the byte-budget backstop never fires, so this
        // observes the per-tool cap alone.
        budget_bytes: 10_000_000,
        payload_summarizer: Some(StubSummarizer::ok(SummarizeOutcome::Unavailable(
            UnavailableReason::Failed,
        ))),
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression:
            crate::openhuman::inference::tokenjuice::AgentTokenjuiceCompression::Off,
        tool_policies,
    };

    let mut result = tool_result("terse", &"payload ".repeat(200));
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();

    let notice = UnavailableReason::Failed.notice();
    assert!(
        result.content.starts_with(notice),
        "the complete notice must lead the content, got {:?}",
        result.content.chars().take(200).collect::<String>()
    );
    assert!(
        result
            .content
            .contains("Do not re-run the tool for a summary"),
        "the do-not-re-run instruction is the whole point of the notice and must survive"
    );
    // The payload itself is still capped — deferring the notice must not
    // smuggle the tool past its own declared limit.
    let payload = result
        .content
        .strip_prefix(notice)
        .expect("notice prefix")
        .trim_start();
    assert!(
        payload.contains("[truncated by tool cap:"),
        "the raw payload must still be truncated to the tool's cap, got {payload:?}"
    );
}

#[tokio::test]
async fn tool_output_honors_a_tools_own_cap() {
    let mut tool_policies = HashMap::new();
    tool_policies.insert(
        "capped".to_string(),
        TaToolPolicy::classified().with_runtime(tinyagents::harness::tool::ToolRuntime {
            timeout_ms: None,
            timeout: tinyagents::harness::tool::ToolTimeout::Inherit,
            max_retries: None,
            idempotent: false,
            cancelable: true,
            sandbox: tinyagents::harness::tool::SandboxMode::Inherit,
            max_result_bytes: Some(20),
            streaming: false,
        }),
    );
    let mw = ToolOutputMiddleware {
        budget_bytes: 100_000,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression: AgentTokenjuiceCompression::Off,
        tool_policies,
    };
    let mut result = tool_result("capped", &"y".repeat(500));
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert!(
        result
            .content
            .contains("truncated by tool cap: 480 more chars not shown"),
        "the tool's own 20-char cap should truncate with the tool-cap marker: {}",
        result.content
    );
}

// ── ToolOutputMiddleware: COMPACTION_EXEMPT_TOOLS (workflow proposals) ───

/// A `workflow_proposal` payload with enough uniform-object rows to clear
/// tinyjuice's `MIN_ROWS` (3) and OpenHuman's default 2 KiB compaction
/// floor — i.e. exactly the shape that used to get its `"type"` marker
/// stripped by the `[json table: …]` rewrite before the middleware
/// exemption existed.
fn large_workflow_proposal_json() -> String {
    let nodes: Vec<serde_json::Value> = (0..20)
        .map(|i| {
            json!({
                "id": format!("node-{i}"),
                "kind": if i == 0 { "trigger" } else { "tool_call" },
                "name": format!("Step {i}"),
                "config": {
                    "slug": format!("oh:placeholder_action_{i}"),
                    "args": { "input": format!("value-{i}"), "note": "generic placeholder payload for size padding" }
                }
            })
        })
        .collect();
    serde_json::to_string(&json!({
        "type": "workflow_proposal",
        "flow_id": "flow-large-graph",
        "graph": { "nodes": nodes, "edges": [] },
    }))
    .unwrap()
}

fn compaction_enabled_mw() -> ToolOutputMiddleware {
    ToolOutputMiddleware {
        budget_bytes: 1_000_000,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: true,
        tokenjuice_compression: AgentTokenjuiceCompression::Full,
        tool_policies: HashMap::new(),
    }
}

#[test]
fn compaction_exempt_tools_contains_every_proposal_tool() {
    for tool in [
        "propose_workflow",
        "revise_workflow",
        "edit_workflow",
        "save_workflow",
        "create_workflow",
    ] {
        assert!(
            COMPACTION_EXEMPT_TOOLS.contains(&tool),
            "{tool} must be exempt from tokenjuice/summarizer compaction"
        );
    }
}

#[tokio::test]
#[ignore = "requires a built TinyJuice module"]
async fn tool_output_tabulates_a_large_graph_for_a_non_exempt_tool() {
    // Sanity baseline proving this test's payload actually exercises real
    // tinyjuice tabulation (and isn't just below-threshold): a tool name
    // NOT in COMPACTION_EXEMPT_TOOLS loses the `"type"` marker.
    // Resolve the explicit release fixture before `after_tool` performs
    // ambient config initialisation. A pristine CI workspace otherwise
    // exercises the production fail-open path before the test override is
    // admitted, hiding a usable module behind unchanged output.
    crate::openhuman::inference::tokenjuice::install_from_config(
        &crate::openhuman::config::Config::default(),
    )
    .await
    .expect("released TinyJuice module must load and accept host configuration");
    let mw = compaction_enabled_mw();
    let payload = large_workflow_proposal_json();
    assert!(
        payload.len()
            >= crate::openhuman::config::Config::default()
                .tokenjuice
                .min_bytes_to_compress,
        "baseline payload must clear OpenHuman's configured compaction floor"
    );
    let mut result = tool_result("some_other_tool", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_ne!(
        result.content, payload,
        "a non-exempt tool's large uniform-array payload should be rewritten by tokenjuice"
    );
    let reparsed: Result<serde_json::Value, _> = serde_json::from_str(&result.content);
    let marker_survived = reparsed
        .ok()
        .and_then(|v| v.get("type").and_then(|t| t.as_str().map(str::to_string)))
        == Some("workflow_proposal".to_string());
    assert!(
        !marker_survived,
        "baseline expectation: tabulation strips the type marker for non-exempt tools"
    );
}

#[tokio::test]
async fn tool_output_leaves_propose_workflow_byte_for_byte_intact() {
    let mw = compaction_enabled_mw();
    let payload = large_workflow_proposal_json();
    let mut result = tool_result("propose_workflow", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(
        result.content, payload,
        "propose_workflow results must pass through compaction untouched"
    );
    let reparsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(reparsed["type"], "workflow_proposal");
    assert_eq!(reparsed["graph"]["nodes"].as_array().unwrap().len(), 20);
}

#[tokio::test]
async fn tool_output_leaves_every_exempt_tool_name_intact() {
    let mw = compaction_enabled_mw();
    let payload = large_workflow_proposal_json();
    for tool in COMPACTION_EXEMPT_TOOLS {
        let mut result = tool_result(tool, &payload);
        mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
        assert_eq!(
            result.content, payload,
            "{tool}'s result must pass through compaction untouched"
        );
    }
}

// ── ToolOutputMiddleware: truncation exemption (#4888 follow-up, gap 1) ──

/// A `workflow_proposal` payload with `node_count` nodes, each padded with
/// a 500-byte `note`, so the caller can force the serialized size past the
/// ~16 KiB shared byte-budget backstop (`DEFAULT_TOOL_RESULT_BUDGET_BYTES`)
/// — the size class a real ≥10-node graph proposal routinely reaches, and
/// exactly what used to get UTF-8-boundary-truncated into unparseable JSON
/// before the truncation exemption existed.
fn oversized_workflow_proposal_json(node_count: usize) -> String {
    let nodes: Vec<serde_json::Value> = (0..node_count)
        .map(|i| {
            json!({
                "id": format!("node-{i}"),
                "kind": if i == 0 { "trigger" } else { "tool_call" },
                "name": format!("Step {i}"),
                "config": {
                    "slug": format!("oh:placeholder_action_{i}"),
                    "args": { "input": format!("value-{i}"), "note": "a".repeat(500) }
                }
            })
        })
        .collect();
    serde_json::to_string(&json!({
        "type": "workflow_proposal",
        "flow_id": "flow-oversized-graph",
        "graph": { "nodes": nodes, "edges": [] },
    }))
    .unwrap()
}

/// Middleware config isolating the byte-cap stages (3+4): tokenjuice off,
/// no tool-declared char cap, the real `DEFAULT_TOOL_RESULT_BUDGET_BYTES`
/// (~16 KiB) as the shared backstop, and no artifact store (so an
/// over-budget non-exempt tool falls straight to inline truncation instead
/// of being persisted — deterministic to assert on).
fn truncation_probe_mw() -> ToolOutputMiddleware {
    ToolOutputMiddleware {
        budget_bytes: DEFAULT_TOOL_RESULT_BUDGET_BYTES,
        payload_summarizer: None,
        artifact_store: None,
        tokenjuice_compaction_enabled: false,
        tokenjuice_compression: AgentTokenjuiceCompression::Off,
        tool_policies: HashMap::new(),
    }
}

#[tokio::test]
async fn tool_output_leaves_an_oversized_propose_workflow_byte_for_byte_intact() {
    // Gap 1: a ≥10-node proposal routinely exceeds the ~16 KiB shared
    // byte-budget backstop. Before the truncation exemption, step 4
    // truncated it at a UTF-8 boundary — invalid JSON, so both
    // `flows::ops::extract_workflow_proposal` and the frontend's
    // `parseWorkflowProposal` silently fell back to `proposal: None` and a
    // blank canvas. This must survive byte-for-byte regardless of size.
    let mw = truncation_probe_mw();
    let payload = oversized_workflow_proposal_json(30);
    assert!(
        payload.len() > DEFAULT_TOOL_RESULT_BUDGET_BYTES,
        "test payload must exceed the shared byte budget to exercise step 4: {} bytes",
        payload.len()
    );
    let mut result = tool_result("propose_workflow", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(
        result.content, payload,
        "an oversized propose_workflow result must not be truncated by the shared byte-budget backstop"
    );
    let reparsed: serde_json::Value = serde_json::from_str(&result.content)
        .expect("must still be valid JSON after passing through after_tool");
    assert_eq!(reparsed["type"], "workflow_proposal");
    assert_eq!(reparsed["graph"]["nodes"].as_array().unwrap().len(), 30);
}

#[tokio::test]
async fn tool_output_truncates_the_same_oversized_payload_for_a_non_exempt_tool() {
    // Baseline pairing with the test above: proves the identical oversized
    // payload IS truncated (and consequently unparseable) for a tool that
    // is NOT truncation-exempt, so the exemption test isn't vacuously true
    // because the payload never actually crossed the budget.
    let mw = truncation_probe_mw();
    let payload = oversized_workflow_proposal_json(30);
    let mut result = tool_result("some_other_tool", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_ne!(
        result.content, payload,
        "a non-exempt tool's oversized payload should be truncated by the shared byte-budget backstop"
    );
    assert!(
        result.content.contains("truncated by tool_result_budget"),
        "expected the byte-budget truncation marker: {}",
        result.content
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(&result.content).is_err(),
        "truncated JSON should no longer parse as a whole document"
    );
}

// ── ToolOutputMiddleware: sampling tools (#4888 follow-up, gap 2) ────────

/// A large uniform-array JSON payload shaped like a real sampled tool
/// response (no `workflow_proposal` envelope) — what `get_tool_output_sample`
/// / `get_tool_contract` actually return so the model can derive an exact
/// `primary_array_path`/`output_fields` from the real shape. `row_count`
/// rows of ≥3 clear tinyjuice's tabulation threshold.
fn large_sample_response_json(row_count: usize) -> String {
    let rows: Vec<serde_json::Value> = (0..row_count)
        .map(|i| {
            json!({
                "id": i,
                "title": format!("Issue {i}"),
                "state": "open",
                "body": "padding padding padding padding padding padding",
            })
        })
        .collect();
    serde_json::to_string(&json!({ "items": rows })).unwrap()
}

#[tokio::test]
async fn get_tool_output_sample_is_compaction_exempt() {
    // Gap 2: tokenjuice tabulation elides the very array the model calls
    // this tool to observe, so it would derive a wrong or nonexistent
    // `split_out.path` from the tabulated summary instead of the real
    // response shape. The sample must reach the model untabulated.
    let mw = compaction_enabled_mw();
    let payload = large_sample_response_json(10);
    let mut result = tool_result("get_tool_output_sample", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(
        result.content, payload,
        "get_tool_output_sample's response must not be tokenjuice-tabulated"
    );
}

#[tokio::test]
async fn get_tool_contract_is_compaction_exempt() {
    let mw = compaction_enabled_mw();
    let payload = large_sample_response_json(10);
    let mut result = tool_result("get_tool_contract", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_eq!(
        result.content, payload,
        "get_tool_contract's response must not be tokenjuice-tabulated"
    );
}

#[tokio::test]
async fn sampling_tool_output_still_hits_the_byte_budget_backstop() {
    // Unlike the proposal tools, sampling tools are deliberately NOT
    // truncation-exempt: a truncated-but-untabulated sample is still a
    // usable (if partial) real response, and these calls can be genuinely
    // large, so the shared byte-budget backstop keeps protecting the
    // context budget for them.
    let mw = truncation_probe_mw();
    let payload = large_sample_response_json(400);
    assert!(
        payload.len() > DEFAULT_TOOL_RESULT_BUDGET_BYTES,
        "test payload must exceed the shared byte budget: {} bytes",
        payload.len()
    );
    let mut result = tool_result("get_tool_output_sample", &payload);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    assert_ne!(
        result.content, payload,
        "get_tool_output_sample must still be subject to the shared byte-budget backstop"
    );
    assert!(
        result.content.contains("truncated by tool_result_budget"),
        "expected the byte-budget truncation marker: {}",
        result.content
    );
}

#[test]
fn compaction_and_truncation_exempt_sets_are_distinct() {
    // Proposal tools: exempt from both compaction and truncation.
    for tool in COMPACTION_EXEMPT_TOOLS {
        assert!(
            is_compaction_exempt(tool),
            "{tool} must be compaction-exempt"
        );
        assert!(
            is_truncation_exempt(tool),
            "{tool} must be truncation-exempt"
        );
    }
    // Sampling tools: exempt from compaction only.
    for tool in SAMPLING_TOOLS {
        assert!(
            is_compaction_exempt(tool),
            "{tool} must be compaction-exempt"
        );
        assert!(
            !is_truncation_exempt(tool),
            "{tool} must remain subject to the char cap / shared byte-budget backstop"
        );
    }
    // An arbitrary non-listed tool: exempt from neither.
    assert!(!is_compaction_exempt("some_other_tool"));
    assert!(!is_truncation_exempt("some_other_tool"));
}

// ── CostBudgetMiddleware ────────────────────────────────────────────────

#[tokio::test]
async fn cost_budget_is_a_noop_without_a_global_tracker() {
    // No global CostTracker is installed in the unit-test process, so the
    // gate self-disables and the model call proceeds.
    let mw = CostBudgetMiddleware::new();
    let mut req = ModelRequest::new(vec![TaMessage::user("hi")]);
    assert!(mw.before_model(&mut ctx(), &(), &mut req).await.is_ok());
}

// ── CostBudgetMiddleware shadow (W2-budget-dedupe) ──────────────────────

/// The shadow comparison at `after_agent` logs parity when the crate
/// `BudgetMiddleware`'s tracker matches the runtime `AgentRun.usage`, and
/// never fails the run — in both the matching and diverging cases. It also
/// must be inert (no panic, `Ok`) when no shadow tracker is installed.
#[tokio::test]
async fn cost_budget_shadow_after_agent_never_fails_the_run() {
    use tinyagents::harness::usage::Usage;

    // No shadow tracker: after_agent is a silent no-op.
    let plain = CostBudgetMiddleware::new();
    let mut run = AgentRun::new();
    run.usage.record(Usage::new(100, 40));
    assert!(plain.after_agent(&mut ctx(), &(), &mut run).await.is_ok());

    // Matching tracker (parity): the crate tracker accumulated the same
    // single call's usage the runtime recorded into `run.usage`.
    let tracker = BudgetTracker::new();
    tracker.record(Usage::new(100, 40), Default::default());
    let shadow = CostBudgetMiddleware::with_shadow(tracker.clone());
    let mut run = AgentRun::new();
    run.usage.record(Usage::new(100, 40));
    assert!(shadow.after_agent(&mut ctx(), &(), &mut run).await.is_ok());

    // Diverging tracker (crate missed a call): still only logs, never fails.
    let mut diverged_run = AgentRun::new();
    diverged_run.usage.record(Usage::new(100, 40));
    diverged_run.usage.record(Usage::new(10, 5));
    assert!(shadow
        .after_agent(&mut ctx(), &(), &mut diverged_run)
        .await
        .is_ok());
}

// ── RepeatedToolFailureMiddleware ───────────────────────────────────────

fn failing_result(name: &str, err: &str) -> TaToolResult {
    let mut r = tool_result(name, err);
    r.error = Some(err.to_string());
    r
}

/// Count how many of the steering commands drained from `handle` are
/// `Pause` (the halt signal). The tracker-driven breaker now also emits a
/// `Redirect` **nudge** below the retry cap, so a raw `pending()` count no
/// longer isolates the halt — the tests classify by command kind instead.
fn drain_pause_count(handle: &SteeringHandle) -> usize {
    handle
        .drain()
        .into_iter()
        .filter(|c| matches!(c, SteeringCommand::Pause))
        .count()
}

#[tokio::test]
async fn repeated_tool_failure_pauses_only_after_the_threshold() {
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    // Two identical failures: below the halt threshold. The crate ladder
    // nudges (Redirect) on the second, but must NOT pause (halt) yet.
    for _ in 0..2 {
        let mut r = failing_result("flaky", "boom");
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "no halt before the threshold"
    );
    // Third identical failure exhausts the same-strategy retries → halt.
    let mut r = failing_result("flaky", "boom");
    mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    assert_eq!(
        drain_pause_count(&handle),
        1,
        "the third identical failure should pause (halt) the run"
    );
}

#[tokio::test]
async fn repeated_tool_failure_resets_on_a_success() {
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    // Two failures, then a success clears the counter.
    for _ in 0..2 {
        let mut r = failing_result("t", "boom");
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    let mut ok = tool_result("t", "fine"); // error = None
    mw.after_tool(&mut ctx(), &(), &mut ok).await.unwrap();
    // Two more failures — still below the halt threshold because the counter
    // reset, so the ladder never reaches the third identical repeat.
    for _ in 0..2 {
        let mut r = failing_result("t", "boom");
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "a success should reset the breaker so it never halts"
    );
}

#[tokio::test]
async fn repeated_tool_failure_ignores_distinct_errors() {
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    // Three *different* errors never trip the breaker — only an identical,
    // deterministic failure loop does (and the varied-failure backstop nudges
    // at 4 / halts at 6, both above this count).
    for err in ["e1", "e2", "e3"] {
        let mut r = failing_result("t", err);
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        handle.pending(),
        0,
        "distinct errors below the backstop must not steer the run"
    );
}

#[test]
fn user_actionable_escalation_detects_missing_connection() {
    // A not-connected blocker → a user-directed ask with a concrete next step.
    let ask = user_actionable_escalation(
        "gmail_send",
        "Gmail is not connected. Ask the user to connect 'gmail' in Connections.",
    )
    .expect("a missing-connection failure is user-actionable");
    assert!(ask.contains("without your input"));
    assert!(ask.contains("Connections"));
    assert!(ask.to_lowercase().contains("connect"));
    assert!(ask.contains("gmail_send"));
    // The original tool text is relayed so the user sees which service.
    assert!(ask.to_lowercase().contains("gmail"));

    // A plain environment failure is NOT user-actionable → keep crate summary.
    assert!(user_actionable_escalation("read_file", "file not found").is_none());
    assert!(user_actionable_escalation("shell", "exit code 1: segfault").is_none());
    assert!(user_actionable_escalation(
        "gmail_send",
        "[composio:error:insufficient_scope] `gmail_send` was rejected because the connected \
         gmail account is missing required permissions (insufficient authentication scopes). \
         Reconnect the integration in Connections → gmail and grant the scopes \
         requested during OAuth."
    )
    .is_none());
    assert!(user_actionable_escalation(
        "gmail_trigger",
        "[composio:error:trigger_permission] Couldn't enable this trigger: the connected \
         gmail account doesn't have permission to manage triggers. Reconnect gmail in \
         Connections → gmail and grant the permissions requested during OAuth, \
         then try again."
    )
    .is_none());
}

#[tokio::test]
async fn halt_on_missing_connection_asks_the_user_instead_of_reporting_back() {
    // #4092: a repeated not-connected failure halts with a user-directed ask,
    // not the crate's generic "unreachable environment, report this back".
    let handle = SteeringHandle::allow_all();
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mw = RepeatedToolFailureMiddleware::new(handle.clone(), 3, slot.clone());
    // Three identical not-connected failures → halt.
    for _ in 0..3 {
        let mut r = failing_result(
            "slack_post",
            "Slack is not connected — connect it in Connections.",
        );
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    let summary = slot
        .lock()
        .unwrap()
        .clone()
        .expect("halt records a summary");
    assert!(
        summary.contains("without your input") && summary.contains("Connections"),
        "the halt should ask the user to connect the service: {summary}"
    );
    assert!(
        !summary.contains("Report this back"),
        "a user-actionable blocker must not use the generic report-back summary: {summary}"
    );
    assert_eq!(
        drain_pause_count(&handle),
        1,
        "it still pauses the run to surface the ask"
    );
}

/// Collect the nudge system-message texts drained from `handle`. The nudge
/// rides the `InjectMessage` lane (not `Redirect`) so it is permitted on the
/// user's interactive turn — see the test below.
fn drain_nudge_messages(handle: &SteeringHandle) -> Vec<String> {
    handle
        .drain()
        .into_iter()
        .filter_map(|c| match c {
            SteeringCommand::InjectMessage(message) => Some(message.text()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn repeated_tool_failure_nudges_change_of_strategy_before_the_halt() {
    use crate::openhuman::agent::tinyagents::orchestration::{
        openhuman_steering_handle, SteeringRunClass,
    };
    use tinyagents::harness::steering::SteeringCommandKind;

    // #4089: before the same-strategy retry cap, the breaker must feed a
    // structured "no progress since step X" corrective back into the loop so
    // the model changes approach rather than retrying the identical failing
    // call — and it must do so *without* pausing yet.
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    // First identical failure: not a loop yet — no steering.
    let mut r = failing_result("read_file", "file not found");
    mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    assert!(
        handle.drain().is_empty(),
        "a single failure is never a loop"
    );
    // Second identical failure: the nudge fires, still no halt.
    let mut r = failing_result("read_file", "file not found");
    mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    let nudges = drain_nudge_messages(&handle);
    assert_eq!(
        nudges.len(),
        1,
        "the repeat should steer the model to change strategy before the retry cap"
    );
    let nudge = &nudges[0];
    assert!(
        nudge.contains("no progress"),
        "the nudge carries the structured no-progress signal: {nudge}"
    );
    assert!(
        nudge.to_lowercase().contains("read_file"),
        "the nudge names the failing call so the model knows what not to repeat: {nudge}"
    );

    // Regression for the #4473 crash: the nudge must ride a steering lane the
    // user's *interactive* turn permits. `Redirect` is Background-only, so a
    // Redirect nudge aborted interactive turns; `InjectMessage` is permitted
    // on both classes. Assert the interactive policy accepts the lane we use.
    let interactive = openhuman_steering_handle(SteeringRunClass::Interactive);
    assert!(
        interactive
            .policy()
            .is_allowed(SteeringCommandKind::InjectMessage),
        "the no-progress nudge must use a lane the interactive turn permits"
    );
    assert!(
        !interactive
            .policy()
            .is_allowed(SteeringCommandKind::Redirect),
        "sanity: interactive still refuses Redirect (the lane that crashed it)"
    );
}

// ── RepeatedToolFailureMiddleware body-level ok:false (flows breaker) ────

/// A `ToolResult::success` (no `error`) whose JSON body carries a top-level
/// `"ok": false` — the shape `validate_workflow` / `dry_run_workflow` return
/// for an invalid graph / aborted sandbox run.
fn body_failure_result(name: &str, extra: serde_json::Value) -> TaToolResult {
    let mut body = json!({ "ok": false });
    if let serde_json::Value::Object(map) = extra {
        body.as_object_mut().unwrap().extend(map);
    }
    tool_result(name, &serde_json::to_string_pretty(&body).unwrap())
}

#[test]
fn is_body_level_failure_detects_validate_and_dry_run_only() {
    assert!(is_body_level_failure(
        "validate_workflow",
        r#"{"ok": false, "errors": ["bad node"]}"#,
    ));
    assert!(is_body_level_failure(
        "dry_run_workflow",
        r#"{"sandbox": true, "ok": false, "error": "aborted"}"#,
    ));
    // ok:true never counts as a failure.
    assert!(!is_body_level_failure(
        "validate_workflow",
        r#"{"ok": true}"#,
    ));
    // A different tool's ok:false is not reinterpreted as a failure — it may
    // be legitimate data.
    assert!(!is_body_level_failure(
        "some_other_tool",
        r#"{"ok": false}"#,
    ));
    // Tolerant of non-JSON / missing `ok`: never guess.
    assert!(!is_body_level_failure("validate_workflow", "not json"));
    assert!(!is_body_level_failure("validate_workflow", r#"{}"#));
}

#[tokio::test]
async fn repeated_validate_workflow_ok_false_trips_the_breaker() {
    // The bug: `validate_workflow` reports an invalid graph via a `success`
    // result body-level `"ok": false`, never `result.error` — so the breaker
    // must synthesize a failure signal from the body or it burns the whole
    // iteration budget on a graph it can never fix.
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    let mut halted = false;
    // Same invalid graph re-validated repeatedly (same content each time, no
    // `error` field): well within the varied-failure any-failure backstop
    // (halts at 6 consecutive) even before the identical-repeat threshold.
    for _ in 0..8 {
        let mut r = body_failure_result(
            "validate_workflow",
            json!({ "errors": ["node 'x' has no outgoing edge"] }),
        );
        assert!(r.error.is_none(), "the tool call itself did not error");
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
        if drain_pause_count(&handle) > 0 {
            halted = true;
            break;
        }
    }
    assert!(
        halted,
        "repeated validate_workflow ok:false must trip the no-progress breaker"
    );
}

#[tokio::test]
async fn single_or_unrelated_ok_false_does_not_falsely_trip_the_breaker() {
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    // A single validate_workflow ok:false is not a loop.
    let mut r = body_failure_result("validate_workflow", json!({}));
    mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "a single body-level failure must not halt"
    );

    // An unrelated tool's ok:false, repeated, must never be reinterpreted as
    // a failure signal — it may be legitimate data from that tool.
    for _ in 0..8 {
        let mut r = body_failure_result("some_other_tool", json!({ "count": 0 }));
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "an unrelated tool's ok:false must not trip the breaker"
    );
    assert!(
        handle.drain().is_empty(),
        "an unrelated tool's ok:false must not even nudge the run"
    );
}

#[tokio::test]
async fn existing_error_is_some_behavior_is_unchanged_by_body_level_check() {
    // Regression guard: a real `result.error` (no body-level ok:false at all)
    // must still drive the breaker exactly as before — three identical
    // failures halt, matching `repeated_tool_failure_pauses_only_after_the_threshold`.
    let handle = SteeringHandle::allow_all();
    let mw = RepeatedToolFailureMiddleware::new(
        handle.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    for _ in 0..2 {
        let mut r = failing_result("flaky", "boom");
        mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "no halt before the threshold"
    );
    let mut r = failing_result("flaky", "boom");
    mw.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    assert_eq!(
        drain_pause_count(&handle),
        1,
        "error.is_some() behavior must be unchanged by the body-level check"
    );

    // A tool result with BOTH `error` set AND a body-level ok:false must not
    // be double-counted — it is still exactly one failed attempt per call.
    let handle2 = SteeringHandle::allow_all();
    let mw2 = RepeatedToolFailureMiddleware::new(
        handle2.clone(),
        3,
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );
    for _ in 0..2 {
        let mut r = body_failure_result("validate_workflow", json!({}));
        r.error = Some("validation failed".to_string());
        mw2.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    }
    assert_eq!(
        drain_pause_count(&handle2),
        0,
        "two identical error+ok:false results are one repeat each, not two — below the halt threshold"
    );
    let mut r = body_failure_result("validate_workflow", json!({}));
    r.error = Some("validation failed".to_string());
    mw2.after_tool(&mut ctx(), &(), &mut r).await.unwrap();
    assert_eq!(
        drain_pause_count(&handle2),
        1,
        "the third identical error+ok:false result halts, same as a plain error"
    );
}

// ── RepeatProgressMiddleware / crate SuccessfulRepeatTracker ───────────

fn repeated_success_response(tool: &str, args: serde_json::Value) -> ModelResponse {
    ModelResponse {
        message: tinyagents::harness::message::AssistantMessage {
            id: None,
            content: vec![ContentBlock::Text("working".to_string())],
            tool_calls: vec![TaToolCall::new("repeat-1", tool, args)],
            usage: None,
        },
        usage: None,
        finish_reason: Some("tool_calls".to_string()),
        raw: None,
        resolved_model: None,
        continue_turn: None,
        served_from_cache: false,
    }
}

async fn run_successful_repeat_cycle(
    mw: &RepeatProgressMiddleware,
    tool: &str,
    args: serde_json::Value,
    error: Option<&str>,
) {
    let mut response = repeated_success_response(tool, args);
    mw.after_model(&mut ctx(), &(), &mut response)
        .await
        .unwrap();
    let mut result = tool_result(tool, "ok");
    result.error = error.map(str::to_string);
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
}

#[tokio::test]
async fn successful_repeat_tracker_halt_maps_to_summary_and_pause() {
    let handle = SteeringHandle::allow_all();
    let summary = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mw = RepeatProgressMiddleware::new(handle.clone(), summary.clone());

    for _ in 0..DEFAULT_REPEAT_CALL_THRESHOLD - 1 {
        run_successful_repeat_cycle(&mw, "lookup", json!({"id": 1}), None).await;
        assert_eq!(drain_pause_count(&handle), 0);
    }
    run_successful_repeat_cycle(&mw, "lookup", json!({"id": 1}), None).await;

    assert_eq!(drain_pause_count(&handle), 1);
    assert!(
        summary
            .lock()
            .unwrap()
            .as_deref()
            .is_some_and(|text| text.contains("successful tool-call batch")),
        "crate halt summary should be preserved for the host turn result"
    );
}

#[tokio::test]
async fn successful_repeat_tracker_resets_failed_and_exempt_batches() {
    let handle = SteeringHandle::allow_all();
    let mw = RepeatProgressMiddleware::new(
        handle.clone(),
        std::sync::Arc::new(std::sync::Mutex::new(None)),
    );

    for _ in 0..DEFAULT_REPEAT_CALL_THRESHOLD - 1 {
        run_successful_repeat_cycle(&mw, "lookup", json!({"id": 1}), None).await;
    }
    run_successful_repeat_cycle(&mw, "lookup", json!({"id": 1}), Some("temporary failure")).await;
    for _ in 0..DEFAULT_REPEAT_CALL_THRESHOLD - 1 {
        run_successful_repeat_cycle(&mw, "lookup", json!({"id": 1}), None).await;
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "a failed batch resets the successful-repeat streak"
    );

    for _ in 0..DEFAULT_REPEAT_OUTPUT_THRESHOLD + 1 {
        run_successful_repeat_cycle(&mw, "wait_subagent", json!({"task_id": "t"}), None).await;
    }
    assert_eq!(
        drain_pause_count(&handle),
        0,
        "polling tools remain exempt from successful-repeat halts"
    );
}

// ── ApprovalSecurityMiddleware ──────────────────────────────────────────

#[test]
fn approval_external_effect_resolution_walks_the_tool_sets() {
    let tools: Arc<Vec<Box<dyn Tool>>> = Arc::new(vec![
        Box::new(FakeTool {
            name: "send_email",
            cap: None,
            external: true,
        }),
        Box::new(FakeTool {
            name: "read_file",
            cap: None,
            external: false,
        }),
    ]);
    let mw = ApprovalSecurityMiddleware::new(vec![tools]);
    assert!(mw.has_external_effect("send_email", &json!({})));
    assert!(!mw.has_external_effect("read_file", &json!({})));
    // Unknown tool defaults to no external effect (nothing to gate).
    assert!(!mw.has_external_effect("missing", &json!({})));
}

// ── MemoryProtocolMiddleware (issue #4116) ──────────────────────────────

use crate::openhuman::agent::harness::memory_protocol::MEMORY_PROTOCOL_MARKER;

/// Drive one full tool cycle through the middleware: `before_tool` (captures
/// the arguments the result won't carry) then `after_tool`, correlated by a
/// shared call id. Returns the (possibly annotated) result.
async fn run_cycle(
    mw: &MemoryProtocolMiddleware,
    name: &str,
    args: serde_json::Value,
    content: &str,
    error: Option<&str>,
) -> TaToolResult {
    let mut call = TaToolCall {
        id: "c1".into(),
        name: name.into(),
        arguments: args,
        invalid: None,
    };
    mw.before_tool(&mut ctx(), &(), &mut call).await.unwrap();
    let mut result = tool_result(name, content); // call_id "c1" matches
    result.error = error.map(|e| e.to_string());
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    result
}

#[tokio::test]
async fn memory_write_without_index_read_gets_a_corrective_note() {
    let mw = MemoryProtocolMiddleware::new();
    let result = run_cycle(&mw, "memory_store", json!({}), "stored entry 42", None).await;
    assert!(
        result.content.contains(MEMORY_PROTOCOL_MARKER),
        "a write with no preceding dedupe read should be annotated: {}",
        result.content
    );
    assert!(result
        .content
        .contains("without first reading the memory index"));
    assert!(result.content.contains("update_memory_md"));
    // The original tool output is preserved, guidance is appended.
    assert!(result.content.starts_with("stored entry 42"));
}

#[tokio::test]
async fn full_cycle_read_then_write_then_update_only_reminds_on_the_write() {
    let mw = MemoryProtocolMiddleware::new();

    let read = run_cycle(&mw, "memory_recall", json!({}), "no dupes", None).await;
    assert!(
        !read.content.contains(MEMORY_PROTOCOL_MARKER),
        "a read is not annotated"
    );

    let write = run_cycle(&mw, "memory_store", json!({}), "stored", None).await;
    assert!(write.content.contains(MEMORY_PROTOCOL_MARKER));
    // The read preceded the write, so no missing-read complaint — just the
    // forward "sync the index" reminder.
    assert!(!write
        .content
        .contains("without first reading the memory index"));

    let update = run_cycle(
        &mw,
        "update_memory_md",
        json!({ "file": "MEMORY.md" }),
        "index updated",
        None,
    )
    .await;
    assert!(
        !update.content.contains(MEMORY_PROTOCOL_MARKER),
        "closing the cycle needs no guidance"
    );
}

#[tokio::test]
async fn skill_md_update_does_not_close_the_memory_cycle() {
    let mw = MemoryProtocolMiddleware::new();
    run_cycle(&mw, "memory_recall", json!({}), "checked", None).await;
    run_cycle(&mw, "memory_store", json!({}), "stored", None).await;
    // update_memory_md targeting SKILL.md must NOT reconcile the MEMORY.md
    // index, so the stale-index warning is still owed at run end.
    run_cycle(
        &mw,
        "update_memory_md",
        json!({ "file": "SKILL.md" }),
        "skill updated",
        None,
    )
    .await;
    let mut run = AgentRun::new();
    // Still pending → after_agent takes its warn path without erroring.
    mw.after_agent(&mut ctx(), &(), &mut run).await.unwrap();
    // A following write reports drift, proving pending was not cleared.
    let next = run_cycle(&mw, "memory_store", json!({}), "again", None).await;
    assert!(
        next.content.contains("drifting"),
        "SKILL.md update must not mask the stale MEMORY.md index: {}",
        next.content
    );
}

#[tokio::test]
async fn consolidated_memory_tree_ingest_is_treated_as_a_write() {
    let mw = MemoryProtocolMiddleware::new();
    let ingest = run_cycle(
        &mw,
        "memory_tree",
        json!({ "mode": "ingest_document" }),
        "ingested",
        None,
    )
    .await;
    assert!(
        ingest.content.contains(MEMORY_PROTOCOL_MARKER),
        "memory_tree ingest_document is a write and must be annotated: {}",
        ingest.content
    );
}

#[tokio::test]
async fn failed_memory_write_does_not_advance_the_protocol() {
    let mw = MemoryProtocolMiddleware::new();
    let failed = run_cycle(
        &mw,
        "memory_store",
        json!({}),
        "disk full",
        Some("disk full"),
    )
    .await;
    // A failed write is not annotated and leaves nothing pending, so a later
    // run-end sweep must not warn about a stale index.
    assert!(!failed.content.contains(MEMORY_PROTOCOL_MARKER));
    let mut run = AgentRun::new();
    // after_agent is a no-op warn path; it must not error.
    mw.after_agent(&mut ctx(), &(), &mut run).await.unwrap();
}

#[tokio::test]
async fn second_write_without_an_update_flags_index_drift() {
    let mw = MemoryProtocolMiddleware::new();
    run_cycle(&mw, "memory_recall", json!({}), "checked", None).await;
    let first = run_cycle(&mw, "memory_store", json!({}), "a", None).await;
    assert!(!first.content.contains("drifting"));

    // No update_memory_md between the two writes → the index is drifting.
    let second = run_cycle(&mw, "memory_store", json!({}), "b", None).await;
    assert!(
        second.content.contains("drifting"),
        "a second unsynced write should flag index drift: {}",
        second.content
    );
}

// ── EmbedderToolHooksMiddleware ──────────────────────────────────────────

/// Records lifecycle notifications for a test hook, optionally vetoing every
/// pre-tool call so the veto path can be exercised.
struct RecordingToolHook {
    name: &'static str,
    pre: std::sync::Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>,
    post: std::sync::Arc<
        std::sync::Mutex<Vec<(String, serde_json::Value, Option<bool>, Option<u64>)>>,
    >,
    veto: bool,
}

#[async_trait]
impl crate::openhuman::agent::hooks::ToolHook for RecordingToolHook {
    fn name(&self) -> &str {
        self.name
    }
    async fn before_tool(
        &self,
        context: &crate::openhuman::agent::hooks::ToolHookContext,
    ) -> anyhow::Result<()> {
        self.pre
            .lock()
            .unwrap()
            .push((context.tool_name.clone(), context.arguments.clone()));
        if self.veto {
            anyhow::bail!("vetoed by test hook");
        }
        Ok(())
    }
    async fn after_tool(
        &self,
        context: &crate::openhuman::agent::hooks::ToolHookContext,
    ) -> anyhow::Result<()> {
        self.post.lock().unwrap().push((
            context.tool_name.clone(),
            context.arguments.clone(),
            context.success,
            context.duration_ms,
        ));
        Ok(())
    }
}

fn embedder_hook_mw(
    pre: std::sync::Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>,
    post: std::sync::Arc<
        std::sync::Mutex<Vec<(String, serde_json::Value, Option<bool>, Option<u64>)>>,
    >,
    veto: bool,
) -> EmbedderToolHooksMiddleware {
    EmbedderToolHooksMiddleware::new(vec![std::sync::Arc::new(RecordingToolHook {
        name: "recording",
        pre,
        post,
        veto,
    })])
}

#[tokio::test]
async fn embedder_tool_hooks_post_use_replays_the_normalized_pre_call_arguments() {
    let pre = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let post = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mw = embedder_hook_mw(pre.clone(), post.clone(), false);

    let mut call = TaToolCall {
        id: "call-1".into(),
        name: "lookup".into(),
        arguments: json!({"id": 42}),
        invalid: None,
    };
    mw.before_tool(&mut ctx(), &(), &mut call).await.unwrap();

    let mut result = TaToolResult {
        call_id: "call-1".into(),
        name: "lookup".into(),
        content: "found".into(),
        raw: None,
        error: None,
        elapsed_ms: 7,
    };
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();

    assert_eq!(pre.lock().unwrap().len(), 1, "one pre-use notification");
    let post = post.lock().unwrap();
    assert_eq!(post.len(), 1, "one post-use notification");
    let (tool, arguments, success, duration) = &post[0];
    assert_eq!(tool, "lookup");
    assert_eq!(
        *arguments,
        json!({"id": 42}),
        "post-use context must preserve the normalized pre-call arguments, not Null"
    );
    assert_eq!(*success, Some(true));
    assert_eq!(*duration, Some(7));
}

#[tokio::test]
async fn embedder_tool_hooks_veto_denies_the_call_and_skips_post_use() {
    let pre = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let post = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mw = embedder_hook_mw(pre.clone(), post.clone(), true);

    let mut call = TaToolCall {
        id: "call-2".into(),
        name: "rm".into(),
        arguments: json!({"path": "/"}),
        invalid: None,
    };
    let error = mw
        .before_tool(&mut ctx(), &(), &mut call)
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("vetoed"),
        "veto must surface as a tool error: {error}"
    );
    // The call was vetoed — no post-use event, and no cache entry leaks.
    assert_eq!(pre.lock().unwrap().len(), 1, "pre-use hook still observed");
    assert!(
        post.lock().unwrap().is_empty(),
        "no post-use for a vetoed call"
    );
    assert!(
        mw.arguments_by_call_id.lock().unwrap().is_empty(),
        "a vetoed call must not leave a cached argument entry"
    );
}

#[tokio::test]
async fn embedder_tool_hooks_post_use_without_pre_call_falls_back_to_null() {
    let pre = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let post = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mw = embedder_hook_mw(pre.clone(), post.clone(), false);

    // A result with no matching `before_tool` (defensive path) must not panic
    // and falls back to `Null`, the pre-fix behaviour.
    let mut result = TaToolResult {
        call_id: "orphan".into(),
        name: "lookup".into(),
        content: "found".into(),
        raw: None,
        error: None,
        elapsed_ms: 3,
    };
    mw.after_tool(&mut ctx(), &(), &mut result).await.unwrap();
    let post = post.lock().unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(post[0].1, serde_json::Value::Null);
    assert_eq!(post[0].2, Some(true));
}
