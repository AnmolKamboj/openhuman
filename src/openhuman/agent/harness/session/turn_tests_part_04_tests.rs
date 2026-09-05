use super::*;

#[tokio::test]
async fn fetch_learned_context_returns_general_prefs_when_explicit_flag_on_learning_off() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mem = make_real_memory(tmp.path());

    // Store two general preferences in the two-lane store (where save_preference
    // writes them). The explicit path now reads `user_pref_general`, not the
    // legacy `user_profile` pinned namespace.
    mem.store(
        crate::openhuman::memory::preferences::USER_PREF_GENERAL_NAMESPACE,
        "package_manager",
        "Use pnpm for package management.",
        crate::openhuman::memory::MemoryCategory::Core,
        None,
    )
    .await
    .unwrap();
    mem.store(
        crate::openhuman::memory::preferences::USER_PREF_GENERAL_NAMESPACE,
        "verbosity",
        "Keep replies terse.",
        crate::openhuman::memory::MemoryCategory::Core,
        None,
    )
    .await
    .unwrap();

    let agent = make_agent_with_memory(
        mem,
        tmp.path().to_path_buf(),
        false, // learning_enabled — full inference stack OFF
        true,  // explicit_preferences_enabled — narrow path ON
    );

    let learned = agent.fetch_learned_context().await;

    assert_eq!(
        learned.user_profile.len(),
        2,
        "explicit flag on, learning off: expected 2 general preferences, got: {:?}",
        learned.user_profile
    );
    assert!(
        learned.user_profile.iter().any(|s| s.contains("pnpm")),
        "package_manager preference value must appear in user_profile: {:?}",
        learned.user_profile
    );
    assert!(
        learned.user_profile.iter().any(|s| s.contains("terse")),
        "verbosity preference value must appear in user_profile: {:?}",
        learned.user_profile
    );
    // Inference-derived data must remain empty — the stack was NOT engaged.
    assert!(
        learned.observations.is_empty(),
        "observations must be empty when learning_enabled=false"
    );
    assert!(
        learned.patterns.is_empty(),
        "patterns must be empty when learning_enabled=false"
    );
    assert!(
        learned.reflections.is_empty(),
        "reflections must be empty when learning_enabled=false"
    );
}

#[tokio::test]
async fn fetch_learned_context_explicit_flag_off_learning_off_returns_empty_even_with_stored_prefs()
{
    let tmp = tempfile::TempDir::new().unwrap();
    let mem = make_real_memory(tmp.path());

    mem.store(
        "user_profile",
        "pinned/style/tone",
        "[pinned] (class=style) tone: formal",
        crate::openhuman::memory::MemoryCategory::Core,
        None,
    )
    .await
    .unwrap();

    let agent = make_agent_with_memory(
        mem,
        tmp.path().to_path_buf(),
        false, // learning_enabled
        false, // explicit_preferences_enabled — both off
    );

    let learned = agent.fetch_learned_context().await;
    assert!(
        learned.user_profile.is_empty(),
        "both flags off: user_profile must be empty even when prefs exist, got: {:?}",
        learned.user_profile
    );
}

#[tokio::test]
async fn fetch_learned_context_loads_general_prefs_when_learning_enabled() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mem = make_real_memory(tmp.path());
    mem.store(
        crate::openhuman::memory::preferences::USER_PREF_GENERAL_NAMESPACE,
        "tone",
        "Be concise and direct.",
        crate::openhuman::memory::MemoryCategory::Core,
        None,
    )
    .await
    .unwrap();

    // learning_enabled=true → full path, which now also sources standing prefs
    // from the explicit user_pref_general store (inferred facets are demoted, so
    // they are no longer injected as ground truth).
    let agent = make_agent_with_memory(mem, tmp.path().to_path_buf(), true, true);
    let learned = agent.fetch_learned_context().await;
    assert!(
        learned.user_profile.iter().any(|s| s.contains("concise")),
        "learning path must inject explicit general prefs into user_profile: {:?}",
        learned.user_profile
    );
}

// ── assistant_message_has_tool_calls — TAURI-RUST-7 envelope check ─────

#[test]
fn assistant_message_has_tool_calls_detects_native_envelope() {
    let body = serde_json::json!({
        "content": "calling tool",
        "tool_calls": [{
            "id": "tc-1",
            "name": "shell",
            "arguments": "{}"
        }]
    })
    .to_string();
    let msg = ChatMessage::assistant(body);
    assert!(super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_non_assistant_role() {
    let body = serde_json::json!({
        "content": "x",
        "tool_calls": [{ "id": "tc-1", "name": "shell", "arguments": "{}" }]
    })
    .to_string();
    let msg = ChatMessage::user(body);
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_plain_text_reply() {
    // Most common positive case for the previous over-broad check: a plain
    // assistant reply whose text happens to mention `tool_calls`.
    let msg = ChatMessage::assistant("I considered using tool_calls but chose not to.");
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_envelope_without_content_field() {
    // A bare `{"tool_calls": [...]}` JSON in the content (no `content` field)
    // is not the envelope `dispatcher.rs` emits.
    let body = serde_json::json!({
        "tool_calls": [{ "id": "tc-1", "name": "shell", "arguments": "{}" }]
    })
    .to_string();
    let msg = ChatMessage::assistant(body);
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_empty_tool_calls_array() {
    let body = serde_json::json!({
        "content": "no tools",
        "tool_calls": []
    })
    .to_string();
    let msg = ChatMessage::assistant(body);
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_malformed_tool_call_items() {
    // tool_call object missing `id` — not the native envelope shape.
    let body_no_id = serde_json::json!({
        "content": "x",
        "tool_calls": [{ "name": "shell", "arguments": "{}" }]
    })
    .to_string();
    assert!(!super::super::assistant_message_has_tool_calls(
        &ChatMessage::assistant(body_no_id)
    ));

    // tool_call object missing `arguments` — also rejected.
    let body_no_args = serde_json::json!({
        "content": "x",
        "tool_calls": [{ "id": "tc-1", "name": "shell" }]
    })
    .to_string();
    assert!(!super::super::assistant_message_has_tool_calls(
        &ChatMessage::assistant(body_no_args)
    ));
}

#[test]
fn assistant_message_has_tool_calls_rejects_non_object_root() {
    // Content is a JSON array, not an object.
    let msg = ChatMessage::assistant(r#"["just", "an", "array"]"#.to_string());
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn assistant_message_has_tool_calls_rejects_non_json_content() {
    // Plain prose that doesn't parse as JSON at all — early-returns false via
    // the `let Ok(value) = serde_json::from_str(...)` arm. Keeps the message
    // when the trailing-strip uses this helper.
    let msg = ChatMessage::assistant("Just a normal text reply, no JSON here.");
    assert!(!super::super::assistant_message_has_tool_calls(&msg));
}

#[test]
fn bound_cached_transcript_messages_pops_trailing_tool_calls_envelope() {
    let agent = make_agent(None); // max_history_messages = 3
                                  // Need > max so the bound runs (early-returns when len <= max).
    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("u1"),
        ChatMessage::assistant("a1"),
        ChatMessage::user("u2"),
        ChatMessage::assistant(tool_calls_envelope("tc-trailing")),
    ];

    // With `max_history_messages = 3` and the leading `system` message,
    // `bound_cached_transcript_messages` keeps the last 2 non-system entries
    // — i.e. `[system, u2, trailing-envelope]`. After the envelope pop the
    // tail is `user("u2")`, not the dropped assistant message.
    let bounded = agent.bound_cached_transcript_messages(messages);
    assert!(
        bounded
            .last()
            .is_some_and(|m| m.role == "user" && m.content == "u2"),
        "trailing tool_calls envelope must be popped; expected user tail 'u2' — got tail role={:?} content={:?}",
        bounded.last().map(|m| m.role.as_str()),
        bounded.last().map(|m| m.content.as_str())
    );
    assert!(
        !bounded
            .iter()
            .any(super::super::assistant_message_has_tool_calls),
        "no tool_calls envelope should survive the strip"
    );
}

#[test]
fn bound_cached_transcript_messages_leaves_plain_assistant_tail_intact() {
    let agent = make_agent(None); // max_history_messages = 3
    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("u1"),
        ChatMessage::assistant("a1"),
        ChatMessage::user("u2"),
        ChatMessage::assistant("plain text reply, no tool_calls"),
    ];

    let bounded = agent.bound_cached_transcript_messages(messages);
    let tail = bounded.last().expect("bounded transcript is non-empty");
    assert_eq!(tail.role, "assistant");
    assert_eq!(tail.content, "plain text reply, no tool_calls");
}

#[test]
fn bound_cached_transcript_messages_strips_multiple_trailing_envelopes() {
    // Defence-in-depth: if the cached transcript ends on multiple consecutive
    // unpaired tool_calls envelopes (e.g. two abortive turns), pop them all.
    let agent = make_agent(None);
    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("u1"),
        ChatMessage::assistant("a1"),
        ChatMessage::assistant(tool_calls_envelope("tc-1")),
        ChatMessage::assistant(tool_calls_envelope("tc-2")),
    ];

    let bounded = agent.bound_cached_transcript_messages(messages);
    let any_envelope = bounded
        .iter()
        .any(super::super::assistant_message_has_tool_calls);
    assert!(
        !any_envelope,
        "all trailing tool_calls envelopes must be stripped"
    );
}

#[test]
fn integration_announcement_fires_once_for_new_toolkit() {
    // Seed the announced set with the startup-connected toolkit, mirroring the
    // turn-1 seed in `run_turn`.
    let mut announced: HashSet<String> = HashSet::new();
    announced.insert("gmail".to_string());

    // A mid-session connect adds `slack`: it should be announced, and recorded
    // so it never re-announces.
    let connected = vec!["gmail".to_string(), "slack".to_string()];
    let newly = newly_connected_slugs(&connected, &mut announced);
    assert_eq!(newly, vec!["slack".to_string()]);
    let note = integration_announcement_note(&newly)
        .expect("a newly-connected toolkit must produce an announcement");
    assert!(
        note.contains("slack"),
        "announcement must name the new toolkit slug, got: {note}"
    );
    assert!(
        !note.contains("gmail"),
        "already-announced toolkit must not be re-announced, got: {note}"
    );
    assert!(
        announced.contains("slack"),
        "the new slug must be recorded as announced"
    );

    // A second refresh with the identical connected set parks nothing — every
    // slug is now in `announced`.
    let second = newly_connected_slugs(&connected, &mut announced);
    assert!(
        second.is_empty(),
        "an unchanged connected set must not re-surface a slug, got: {second:?}"
    );
    assert!(integration_announcement_note(&second).is_none());
}

#[test]
fn mcp_announcement_fires_once_for_new_server() {
    // Seed the announced set with the startup-connected MCP server, mirroring
    // the turn-1 seed in `run_turn` (those are already in the system prompt's
    // `## Connected MCP Servers` block, so only mid-session connects announce).
    let mut announced: HashSet<String> = HashSet::new();
    announced.insert("ac.tandem/docs-mcp".to_string());

    // A mid-session connect adds a weather server: it should be announced once,
    // and recorded so it never re-announces.
    let connected = vec![
        "ac.tandem/docs-mcp".to_string(),
        "io.weather/mcp".to_string(),
    ];
    let newly = newly_connected_slugs(&connected, &mut announced);
    assert_eq!(newly, vec!["io.weather/mcp".to_string()]);
    let note = mcp_announcement_note(&newly)
        .expect("a newly-connected MCP server must produce an announcement");
    assert!(
        note.contains("io.weather/mcp"),
        "announcement must name the new server, got: {note}"
    );
    assert!(
        note.contains("use_mcp_server"),
        "announcement must point the model at the use_mcp_server delegate, got: {note}"
    );
    assert!(
        !note.contains("ac.tandem/docs-mcp"),
        "an already-announced server must not be re-announced, got: {note}"
    );

    // A second pass with the identical connected set parks nothing.
    let second = newly_connected_slugs(&connected, &mut announced);
    assert!(
        second.is_empty(),
        "an unchanged connected set must not re-surface a server, got: {second:?}"
    );
    assert!(mcp_announcement_note(&second).is_none());
}

#[test]
fn integration_announcement_accumulates_two_connects_in_one_note() {
    // Two mid-session connects between consecutive user turns must BOTH be
    // announced — the second must not overwrite the first (#3044 regression:
    // the old `Option<String>` field dropped the earlier note).
    let mut announced: HashSet<String> = HashSet::new();
    announced.insert("gmail".to_string());
    let mut pending: Vec<String> = Vec::new();

    // First connect: notion.
    for slug in newly_connected_slugs(&["gmail".to_string(), "notion".to_string()], &mut announced)
    {
        if !pending.contains(&slug) {
            pending.push(slug);
        }
    }
    // Second connect before the user turn: slack.
    for slug in newly_connected_slugs(
        &[
            "gmail".to_string(),
            "notion".to_string(),
            "slack".to_string(),
        ],
        &mut announced,
    ) {
        if !pending.contains(&slug) {
            pending.push(slug);
        }
    }

    let note = integration_announcement_note(&pending).expect("two connects must produce a note");
    assert!(
        note.contains("notion"),
        "first connect must survive: {note}"
    );
    assert!(
        note.contains("slack"),
        "second connect must be present: {note}"
    );
    assert!(
        !note.contains("gmail"),
        "startup slug must not re-announce: {note}"
    );
}

#[test]
fn skill_announcement_note_empty_yields_none() {
    assert!(super::super::skill_announcement_note(&[]).is_none());
}

#[test]
fn skill_announcement_note_mentions_ids_and_run_skill() {
    let note = super::super::skill_announcement_note(&[
        "ascii-art".to_string(),
        "github-issues".to_string(),
    ])
    .expect("non-empty input should yield a note");
    assert!(note.contains("[skills update]"));
    assert!(note.contains("ascii-art"));
    assert!(note.contains("github-issues"));
    assert!(
        note.contains("run_skill"),
        "note must steer the model to run_skill: {note}"
    );
}

#[test]
fn skill_retraction_note_empty_yields_none() {
    assert!(super::super::skill_retraction_note(&[]).is_none());
}

#[test]
fn skill_retraction_note_names_removed_skills_and_warns_against_run_skill() {
    let note = super::super::skill_retraction_note(&[
        "ascii-art".to_string(),
        "github-issues".to_string(),
    ])
    .expect("non-empty input should yield a note");
    assert!(note.contains("[skills retracted]"));
    assert!(note.contains("ascii-art"));
    assert!(note.contains("github-issues"));
    assert!(
        note.contains("run_skill"),
        "note must mention run_skill so the model knows not to invoke it: {note}"
    );
    assert!(
        !note.contains("[skills update]"),
        "retraction note must not look like an install announcement: {note}"
    );
}

// ── Lane C: auto-recall of facts about the user (#6040) ──────────────────────
//
// The lane is exercised through a real `turn()`: a scripted source stands in
// for the memory driver, the scripted chat model records every request, and
// the assertions read the user message the model was actually sent.

use crate::openhuman::memory::api::error::MemoryError;
use crate::openhuman::memory::api::provider::retrieval::{
    FastRetrieveQuery, RetrievalHit, RetrievalNodeKind, RetrievalResponse,
};
use crate::openhuman::memory::auto_recall::{AutoRecall, AutoRecallSource, AUTO_RECALL_BANNER};

const IDOL_QUESTION: &str = "who is my idol and why?";
const IDOL_FACT: &str = "Idol: Virat Kohli, because of his dedication and consistency";

struct ScriptedAutoRecallSource {
    hits: Vec<RetrievalHit>,
    delay: Duration,
    calls: AtomicUsize,
}

impl ScriptedAutoRecallSource {
    fn with_fact(content: &str) -> Arc<Self> {
        Arc::new(Self {
            hits: vec![RetrievalHit {
                node_id: "leaf-1".into(),
                node_kind: RetrievalNodeKind::Leaf,
                tree_id: String::new(),
                tree_kind: None,
                tree_scope: "folder:profile".into(),
                level: 0,
                content: content.to_string(),
                entities: Vec::new(),
                topics: Vec::new(),
                time_range_start: chrono::DateTime::<chrono::Utc>::default(),
                time_range_end: chrono::DateTime::<chrono::Utc>::default(),
                score: 0.9,
                child_ids: Vec::new(),
                source_ref: None,
            }],
            delay: Duration::ZERO,
            calls: AtomicUsize::new(0),
        })
    }

    fn slow(content: &str, delay: Duration) -> Arc<Self> {
        let mut source = Arc::try_unwrap(Self::with_fact(content))
            .ok()
            .expect("fresh arc");
        source.delay = delay;
        Arc::new(source)
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl AutoRecallSource for ScriptedAutoRecallSource {
    async fn fast_retrieve(
        &self,
        _query: &str,
        _options: FastRetrieveQuery,
    ) -> Result<RetrievalResponse, MemoryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        Ok(RetrievalResponse {
            total: self.hits.len(),
            hits: self.hits.clone(),
            truncated: false,
        })
    }
}

fn scripted_reply(text: &str) -> Arc<SequenceProvider> {
    Arc::new(SequenceProvider {
        responses: AsyncMutex::new(vec![Ok(ChatResponse {
            text: Some(text.into()),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })]),
        requests: AsyncMutex::new(Vec::new()),
        tool_counts: AsyncMutex::new(Vec::new()),
    })
}

/// A turn-capable agent with Lane C bound (or deliberately absent), over the
/// same real embedded store the other turn tests use.
fn make_agent_with_auto_recall(
    provider: Arc<dyn ChatModel<()>>,
    auto_recall: Option<Arc<AutoRecall>>,
) -> Agent {
    let workspace = tempfile::TempDir::new().expect("temp workspace");
    let workspace_path = workspace.path().to_path_buf();
    std::mem::forget(workspace);
    let memory_cfg = crate::openhuman::config::MemoryConfig {
        backend: "none".into(),
        ..crate::openhuman::config::MemoryConfig::default()
    };
    crate::openhuman::memory::host_impls::install_for_tests();
    let mem: Arc<dyn Memory> =
        Arc::from(tinymemory_core::store::create_memory(&memory_cfg, &workspace_path).unwrap());

    Agent::builder()
        .chat_model(provider)
        .tools(vec![])
        .memory(mem)
        .auto_recall(auto_recall)
        .tool_dispatcher(Box::new(XmlToolDispatcher))
        .config(crate::openhuman::config::AgentConfig::default())
        .context_config(crate::openhuman::config::ContextConfig::default())
        .workspace_dir(workspace_path)
        .event_context("turn-test-session", "turn-test-channel")
        .build()
        .unwrap()
}

/// The messages the model was sent on the first (only) request, split by role.
async fn first_request(provider: &SequenceProvider) -> (Vec<String>, Vec<String>) {
    let requests = provider.requests.lock().await;
    let request = requests.first().expect("the model was called once");
    let system = request
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.clone())
        .collect();
    let user = request
        .iter()
        .filter(|m| m.role == "user")
        .map(|m| m.content.clone())
        .collect();
    (system, user)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_recall_block_rides_the_user_message_for_a_personal_question() {
    let source = ScriptedAutoRecallSource::with_fact(IDOL_FACT);
    let lane = Arc::new(AutoRecall::new(source.clone(), true, None));
    let provider_impl = scripted_reply("Virat Kohli, for his dedication and consistency.");
    let provider: Arc<dyn ChatModel<()>> = provider_impl.clone();
    let mut agent = make_agent_with_auto_recall(provider, Some(lane));

    let reply = agent.turn(IDOL_QUESTION).await.expect("turn succeeds");
    assert!(reply.contains("Virat"));
    assert_eq!(source.calls(), 1, "one bounded lookup for one gated turn");

    let (system, user) = first_request(&provider_impl).await;
    let last_user = user.last().expect("a user message");
    assert!(
        last_user.contains(AUTO_RECALL_BANNER) && last_user.contains("Virat Kohli"),
        "the block must ride the user message: {last_user}"
    );
    assert!(
        last_user.contains(IDOL_QUESTION),
        "the user's own words must follow the block: {last_user}"
    );
    assert!(
        system.iter().all(|s| !s.contains(AUTO_RECALL_BANNER)),
        "the block must never land in the system prompt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_recall_stays_silent_and_costs_nothing_on_small_talk() {
    let source = ScriptedAutoRecallSource::with_fact(IDOL_FACT);
    let lane = Arc::new(AutoRecall::new(source.clone(), true, None));
    let provider_impl = scripted_reply("Sunny, probably.");
    let provider: Arc<dyn ChatModel<()>> = provider_impl.clone();
    let mut agent = make_agent_with_auto_recall(provider, Some(lane));

    agent
        .turn("what's the weather today")
        .await
        .expect("turn succeeds");
    assert_eq!(source.calls(), 0, "a closed gate never reaches the store");
    let (_, user) = first_request(&provider_impl).await;
    assert!(user.iter().all(|u| !u.contains(AUTO_RECALL_BANNER)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_recall_disabled_by_the_hooks_switch_injects_nothing() {
    let source = ScriptedAutoRecallSource::with_fact(IDOL_FACT);
    let lane = Arc::new(AutoRecall::new(source.clone(), false, None));
    let provider_impl = scripted_reply("I don't have that stored.");
    let provider: Arc<dyn ChatModel<()>> = provider_impl.clone();
    let mut agent = make_agent_with_auto_recall(provider, Some(lane));

    agent.turn(IDOL_QUESTION).await.expect("turn succeeds");
    assert_eq!(source.calls(), 0);
    let (_, user) = first_request(&provider_impl).await;
    assert!(user.iter().all(|u| !u.contains(AUTO_RECALL_BANNER)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_recall_timeout_yields_an_ordinary_turn() {
    let source = ScriptedAutoRecallSource::slow(IDOL_FACT, Duration::from_millis(300));
    let lane = Arc::new(
        AutoRecall::new(source.clone(), true, None).with_budget(Duration::from_millis(20)),
    );
    let provider_impl = scripted_reply("Still here.");
    let provider: Arc<dyn ChatModel<()>> = provider_impl.clone();
    let mut agent = make_agent_with_auto_recall(provider, Some(lane));

    let reply = agent.turn(IDOL_QUESTION).await.expect("turn succeeds");
    assert_eq!(reply, "Still here.");
    assert_eq!(source.calls(), 1);
    let (_, user) = first_request(&provider_impl).await;
    assert!(user.iter().all(|u| !u.contains(AUTO_RECALL_BANNER)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_without_the_lane_turns_as_before() {
    let provider_impl = scripted_reply("Hello.");
    let provider: Arc<dyn ChatModel<()>> = provider_impl.clone();
    let mut agent = make_agent_with_auto_recall(provider, None);

    let reply = agent.turn(IDOL_QUESTION).await.expect("turn succeeds");
    assert_eq!(reply, "Hello.");
    let (_, user) = first_request(&provider_impl).await;
    assert!(user.iter().all(|u| !u.contains(AUTO_RECALL_BANNER)));
}
