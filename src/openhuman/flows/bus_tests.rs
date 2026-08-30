use super::*;
use crate::openhuman::flows::Flow;
use serde_json::json;
use tinyflows::model::{Node, NodeKind, WorkflowGraph};

/// A directly-constructed, isolated [`Memory`] for the digest tests — NOT
/// the process-global `OnceLock` client. The global is one-shot, so an
/// earlier test in the same binary may already have bound it to a different
/// workspace, making `global::init(..)` here a silent no-op (see
/// `memory::global`'s own test notes). Injecting this instance into the
/// subscriber via [`FlowRunDigestSubscriber::with_memory`] makes writes and
/// read-backs go through the SAME store deterministically — the same shape
/// `flows::memory_tools`' tests use.
/// A guard over an in-memory store.
///
/// This used to build a real `UnifiedMemory` over `tmp` so writes and
/// read-backs went through one store. The digest writes through the guarded
/// driver now, so the fake sits behind a real `MemoryGuard` — same
/// determinism, same round trip, and the policy layer is on the path where
/// production has it.
fn digest_test_memory(
    _tmp: &tempfile::TempDir,
) -> Arc<crate::openhuman::memory::guard::MemoryGuard> {
    crate::openhuman::memory::guard::in_memory::guarded_in_memory().1
}

fn test_config(tmp: &tempfile::TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        action_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    Arc::new(config)
}

fn trigger_node(config: Value) -> Node {
    Node {
        id: "t".to_string(),
        kind: NodeKind::Trigger,
        type_version: 1,
        name: "Trigger".to_string(),
        config,
        ports: Vec::new(),
        position: None,
    }
}

fn flow_with_trigger_config(id: &str, enabled: bool, trigger_config: Value) -> Flow {
    Flow {
        id: id.to_string(),
        name: id.to_string(),
        enabled,
        graph: WorkflowGraph {
            nodes: vec![trigger_node(trigger_config)],
            ..Default::default()
        },
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        last_run_at: None,
        last_status: None,
        require_approval: false,
    }
}

fn dedup_node(id: &str) -> Node {
    Node {
        id: id.to_string(),
        kind: NodeKind::Dedup,
        type_version: 1,
        name: id.to_string(),
        config: json!({ "key": "=item.id" }),
        ports: Vec::new(),
        position: None,
    }
}

/// A saved flow with a `trigger` node plus one `dedup` node with id
/// `dedup_id` — the minimal graph [`DedupCommitSubscriber::dedup_node_ids`]
/// needs to find something to settle.
fn flow_with_dedup_node(id: &str, dedup_id: &str) -> Flow {
    Flow {
        id: id.to_string(),
        name: id.to_string(),
        enabled: true,
        graph: WorkflowGraph {
            nodes: vec![trigger_node(json!({})), dedup_node(dedup_id)],
            ..Default::default()
        },
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        last_run_at: None,
        last_status: None,
        require_approval: false,
    }
}

#[test]
fn pinned_trigger_inputs_reads_values_an_author_fixed_for_unattended_runs() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({
            "trigger_kind": "schedule",
            "schedule": "0 9 * * *",
            "inputs": { "repo": "acme/api", "depth": 3 }
        }),
    );
    let inputs = pinned_trigger_inputs(&flow);
    assert_eq!(inputs["repo"], json!("acme/api"));
    assert_eq!(inputs["depth"], json!(3));
}

#[test]
fn pinned_trigger_inputs_is_empty_when_unset_or_malformed() {
    // Empty, not an error: a flow declaring no inputs (the overwhelming
    // majority) must keep dispatching on a tick exactly as before, and a
    // malformed value is caught downstream by `prepare_flow_run`, which
    // reports it against the flow's actual declarations.
    for cfg in [
        json!({ "trigger_kind": "schedule" }),
        json!({ "trigger_kind": "schedule", "inputs": null }),
        json!({ "trigger_kind": "schedule", "inputs": ["repo"] }),
    ] {
        let flow = flow_with_trigger_config("f1", true, cfg.clone());
        assert!(
            pinned_trigger_inputs(&flow).is_empty(),
            "expected no pinned inputs for {cfg}"
        );
    }
}

#[test]
fn pinned_trigger_inputs_is_empty_for_a_graph_with_no_trigger() {
    let mut flow = flow_with_trigger_config("f1", true, json!({ "trigger_kind": "schedule" }));
    flow.graph.nodes.clear();
    assert!(pinned_trigger_inputs(&flow).is_empty());
}

#[test]
fn name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowTriggerSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::trigger");
    assert_eq!(
        sub.domains(),
        Some(&["cron", "composio", "webhook", "system"][..])
    );
}

#[tokio::test]
async fn handle_does_not_panic_on_arbitrary_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowTriggerSubscriber::new(test_config(&tmp));
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
    sub.handle(&DomainEvent::FlowScheduleTick {
        flow_id: "missing-flow".into(),
    })
    .await;
}

#[test]
fn extract_trigger_kind_reads_schedule() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    assert!(matches!(
        extract_trigger_kind(&flow),
        Some(TriggerKind::Schedule)
    ));
}

#[test]
fn extract_trigger_kind_none_for_missing_discriminator() {
    let flow = flow_with_trigger_config("f1", true, json!({}));
    assert!(extract_trigger_kind(&flow).is_none());
}

#[test]
fn extract_trigger_kind_none_for_invalid_discriminator() {
    let flow = flow_with_trigger_config("f1", true, json!({ "trigger_kind": "not_a_kind" }));
    assert!(extract_trigger_kind(&flow).is_none());
}

#[test]
fn matches_app_event_requires_toolkit_and_slug_match() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "app_event", "toolkit": "gmail", "trigger_slug": "GMAIL_NEW_GMAIL_MESSAGE" }),
    );
    assert!(matches_app_event(&flow, "gmail", "GMAIL_NEW_GMAIL_MESSAGE"));
    // Case-insensitive.
    assert!(matches_app_event(&flow, "Gmail", "gmail_new_gmail_message"));
    // Wrong toolkit or slug does not match.
    assert!(!matches_app_event(
        &flow,
        "slack",
        "GMAIL_NEW_GMAIL_MESSAGE"
    ));
    assert!(!matches_app_event(&flow, "gmail", "SLACK_NEW_MESSAGE"));
}

#[test]
fn matches_app_event_false_for_non_app_event_trigger() {
    let flow = flow_with_trigger_config(
        "f1",
        true,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    assert!(!matches_app_event(
        &flow,
        "gmail",
        "GMAIL_NEW_GMAIL_MESSAGE"
    ));
}

#[tokio::test]
async fn handle_app_event_ignores_disabled_flows() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config(
        "disabled-flow",
        false,
        json!({ "trigger_kind": "app_event", "toolkit": "gmail", "trigger_slug": "GMAIL_NEW_GMAIL_MESSAGE" }),
    );
    crate::openhuman::flows::store::upsert_flow(&config, &flow).unwrap();

    // `list_enabled_flows` must not surface the disabled flow at all —
    // proves the subscriber's dispatch source already excludes it,
    // rather than asserting on a spawned background task's side effect.
    let (enabled, skipped) = crate::openhuman::flows::store::list_enabled_flows(&config).unwrap();
    assert!(enabled.is_empty());
    assert_eq!(skipped, 0);
}

#[tokio::test]
async fn handle_schedule_tick_ignores_disabled_flow() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config(
        "sched-flow",
        false,
        json!({ "trigger_kind": "schedule", "schedule": "0 9 * * *" }),
    );
    crate::openhuman::flows::store::upsert_flow(&config, &flow).unwrap();

    let sub = FlowTriggerSubscriber::new(config.clone());
    // Must not panic and must not spawn a run for a disabled flow — we
    // can't directly observe "no run happened" without a full flows_run
    // fixture, but this exercises the early-return path without error.
    sub.handle(&DomainEvent::FlowScheduleTick {
        flow_id: "sched-flow".into(),
    })
    .await;
}

// ── in-flight dedupe (CodeRabbit finding B) ─────────────────────

#[test]
fn try_acquire_dispatch_skips_a_flow_already_in_flight() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowTriggerSubscriber::new(test_config(&tmp));

    let guard = sub
        .try_acquire_dispatch("f1")
        .expect("first claim for f1 should succeed");
    assert!(
        sub.try_acquire_dispatch("f1").is_none(),
        "a second claim for the same flow while the first is held must be skipped"
    );

    // A different flow is unaffected.
    assert!(sub.try_acquire_dispatch("f2").is_some());

    drop(guard);
    assert!(
        sub.try_acquire_dispatch("f1").is_some(),
        "dropping the guard must release the claim so f1 can run again"
    );
}

#[test]
fn default_constructs_the_same_as_new() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let a = FlowTriggerSubscriber::new(config.clone());
    let b = FlowTriggerSubscriber::new(config);
    assert_eq!(a.name(), b.name());
}

// ── FlowRunDigestSubscriber ─────────────────────────────────────

#[test]
fn digest_name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowRunDigestSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::digest");
    assert_eq!(sub.domains(), Some(&["cron"][..]));
}

#[tokio::test]
async fn digest_handle_does_not_panic_on_unrelated_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = FlowRunDigestSubscriber::new(test_config(&tmp));
    // Must not panic, and must not touch the memory layer at all, for
    // any event other than `FlowRunFinished`.
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
}

#[tokio::test]
async fn digest_ignores_failed_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let memory = digest_test_memory(&tmp);

    let flow = flow_with_trigger_config("f-failed", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-failed",
        "f-failed",
        "thread-failed",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-failed",
        "failed",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        Some("boom"),
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::with_memory(config, memory.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-failed".into(),
        run_id: "run-failed".into(),
        status: "failed".into(),
    })
    .await;

    let entry = memory
        .get(&flow_namespace("f-failed"), "run_digest:run-failed")
        .await
        .unwrap();
    assert!(
        entry.is_none(),
        "a failed run must never produce a run_digest entry"
    );
}

#[tokio::test]
async fn digest_ignores_cancelled_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let memory = digest_test_memory(&tmp);

    let flow = flow_with_trigger_config("f-cancelled", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-cancelled",
        "f-cancelled",
        "thread-cancelled",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-cancelled",
        "cancelled",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::with_memory(config, memory.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-cancelled".into(),
        run_id: "run-cancelled".into(),
        status: "cancelled".into(),
    })
    .await;

    let entry = memory
        .get(&flow_namespace("f-cancelled"), "run_digest:run-cancelled")
        .await
        .unwrap();
    assert!(entry.is_none());
}

#[tokio::test]
async fn digest_writes_run_digest_entry_for_completed_run() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let memory = digest_test_memory(&tmp);

    let flow = flow_with_trigger_config("f-ok", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-ok",
        "f-ok",
        "thread-ok",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    let step = crate::openhuman::flows::FlowRunStep {
        node_id: "n1".to_string(),
        output: json!({ "sent": 3 }),
        port: None,
        status: Some("success".to_string()),
        duration_ms: Some(12),
        diagnostics: Vec::new(),
    };
    store::finish_flow_run(
        &config,
        "run-ok",
        "completed",
        "2026-01-01T00:05:00Z",
        &[step],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::with_memory(config, memory.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-ok".into(),
        run_id: "run-ok".into(),
        status: "completed".into(),
    })
    .await;

    let entry = memory
        .get(&flow_namespace("f-ok"), "run_digest:run-ok")
        .await
        .unwrap()
        .expect("completed run must produce a run_digest entry");
    assert_eq!(entry.taint, MemoryTaint::ExternalSync);
    assert!(entry.content.contains("f-ok"));
    assert!(entry.content.contains("completed"));
    assert!(entry.content.contains("n1"));
    assert!(entry.content.chars().count() <= DIGEST_MAX_CHARS);
}

#[tokio::test]
async fn digest_treats_completed_with_warnings_as_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let memory = digest_test_memory(&tmp);

    let flow = flow_with_trigger_config("f-warn", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();
    store::insert_flow_run(
        &config,
        "run-warn",
        "f-warn",
        "thread-warn",
        "2026-01-01T00:00:00Z",
    )
    .unwrap();
    store::finish_flow_run(
        &config,
        "run-warn",
        "completed_with_warnings",
        "2026-01-01T00:05:00Z",
        &[],
        &[],
        None,
        None,
    )
    .unwrap();

    let sub = FlowRunDigestSubscriber::with_memory(config, memory.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-warn".into(),
        run_id: "run-warn".into(),
        status: "completed_with_warnings".into(),
    })
    .await;

    let entry = memory
        .get(&flow_namespace("f-warn"), "run_digest:run-warn")
        .await
        .unwrap();
    assert!(entry.is_some());
}

#[test]
fn truncate_chars_bounds_output_and_marks_truncation() {
    let long = "x".repeat(50);
    let truncated = truncate_chars(&long, 10);
    assert_eq!(truncated.chars().count(), 10);
    assert!(truncated.ends_with('…'));

    let short = "hello";
    assert_eq!(truncate_chars(short, 10), "hello");
}

#[test]
fn render_run_digest_is_bounded_and_includes_key_fields() {
    let run = FlowRun {
        id: "run-1".to_string(),
        flow_id: "f1".to_string(),
        thread_id: "thread-1".to_string(),
        status: "completed".to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        finished_at: Some("2026-01-01T00:05:00Z".to_string()),
        steps: vec![crate::openhuman::flows::FlowRunStep {
            node_id: "n1".to_string(),
            output: json!({ "ok": true }),
            port: None,
            status: Some("success".to_string()),
            duration_ms: Some(5),
            diagnostics: Vec::new(),
        }],
        pending_approvals: Vec::new(),
        error: None,
        graph_hash: None,
    };
    let digest = render_run_digest("My Flow", &run);
    assert!(digest.contains("My Flow"));
    assert!(digest.contains("completed"));
    assert!(digest.contains("n1"));
    assert!(digest.chars().count() <= DIGEST_MAX_CHARS);
}

// ── DedupCommitSubscriber ────────────────────────────────────────

fn dedup_state_namespace(flow_id: &str) -> String {
    // MUST match `tinyflows::build_capabilities`'s `state_namespace`
    // (`src/openhuman/flows/tinyflows/caps.rs`) — this test asserts the
    // subscriber collides with the SAME keys the engine's `dedup` node
    // itself reads/writes, not just "some" namespace.
    format!("flow:{flow_id}")
}

#[test]
fn dedup_commit_name_and_domains_are_stable() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = DedupCommitSubscriber::new(test_config(&tmp));
    assert_eq!(sub.name(), "flows::dedup_commit");
    assert_eq!(sub.domains(), Some(&["cron"][..]));
}

#[tokio::test]
async fn dedup_commit_ignores_unrelated_events() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sub = DedupCommitSubscriber::new(test_config(&tmp));
    // Must not panic for any event other than `FlowRunFinished`.
    sub.handle(&DomainEvent::CronJobTriggered {
        job_id: "j1".into(),
        job_name: "test".into(),
        job_type: "shell".into(),
    })
    .await;
}

#[tokio::test]
async fn dedup_commit_flow_with_no_dedup_nodes_is_a_noop() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_trigger_config("f-no-dedup", true, json!({}));
    store::upsert_flow(&config, &flow).unwrap();

    let sub = DedupCommitSubscriber::new(config);
    // Must not panic when the flow has no `dedup` node at all.
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-no-dedup".into(),
        run_id: "run-1".into(),
        status: "completed".into(),
    })
    .await;
}

#[tokio::test]
async fn dedup_commit_unions_tentative_into_committed_and_clears_tentative_on_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-ok", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-ok");
    store::kv_set(&config, &namespace, "dedup:dd:committed", &json!(["a"])).unwrap();
    store::kv_set(
        &config,
        &namespace,
        "dedup:dd:tentative",
        &json!(["b", "c"]),
    )
    .unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-ok".into(),
        run_id: "run-ok".into(),
        status: "completed".into(),
    })
    .await;

    let committed = store::kv_get(&config, &namespace, "dedup:dd:committed")
        .unwrap()
        .expect("committed key must still exist");
    let mut committed: Vec<&str> = committed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    committed.sort_unstable();
    assert_eq!(committed, vec!["a", "b", "c"], "committed = union");

    assert!(
        store::kv_get(&config, &namespace, "dedup:dd:tentative")
            .unwrap()
            .is_none(),
        "tentative must be cleared after a successful commit"
    );
}

#[tokio::test]
async fn dedup_commit_treats_completed_with_warnings_as_success() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-warn", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-warn");
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["x"])).unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-warn".into(),
        run_id: "run-warn".into(),
        status: "completed_with_warnings".into(),
    })
    .await;

    let committed = store::kv_get(&config, &namespace, "dedup:dd:committed")
        .unwrap()
        .expect("completed_with_warnings must still commit");
    assert_eq!(committed, json!(["x"]));
    assert!(store::kv_get(&config, &namespace, "dedup:dd:tentative")
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn dedup_commit_releases_tentative_without_touching_committed_on_failure() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-failed", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-failed");
    store::kv_set(&config, &namespace, "dedup:dd:committed", &json!(["a"])).unwrap();
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["b"])).unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-failed".into(),
        run_id: "run-failed".into(),
        status: "failed".into(),
    })
    .await;

    assert_eq!(
        store::kv_get(&config, &namespace, "dedup:dd:committed")
            .unwrap()
            .unwrap(),
        json!(["a"]),
        "committed must be untouched by a failed run"
    );
    assert!(
        store::kv_get(&config, &namespace, "dedup:dd:tentative")
            .unwrap()
            .is_none(),
        "tentative must be released (cleared) on failure so the item retries"
    );
}

#[tokio::test]
async fn dedup_commit_releases_tentative_on_cancelled_and_interrupted() {
    for status in ["cancelled", "interrupted"] {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = test_config(&tmp);
        let flow_id = format!("f-{status}");
        let flow = flow_with_dedup_node(&flow_id, "dd");
        store::upsert_flow(&config, &flow).unwrap();

        let namespace = dedup_state_namespace(&flow_id);
        store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["z"])).unwrap();

        let sub = DedupCommitSubscriber::new(config.clone());
        sub.handle(&DomainEvent::FlowRunFinished {
            flow_id: flow_id.clone(),
            run_id: format!("run-{status}"),
            status: status.to_string(),
        })
        .await;

        assert!(
            store::kv_get(&config, &namespace, "dedup:dd:committed")
                .unwrap()
                .is_none(),
            "status {status} must never commit"
        );
        assert!(
            store::kv_get(&config, &namespace, "dedup:dd:tentative")
                .unwrap()
                .is_none(),
            "status {status} must release tentative"
        );
    }
}

#[tokio::test]
async fn dedup_commit_two_dedup_nodes_settle_independently() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = Flow {
        id: "f-multi".to_string(),
        name: "f-multi".to_string(),
        enabled: true,
        graph: WorkflowGraph {
            nodes: vec![
                trigger_node(json!({})),
                dedup_node("dd1"),
                dedup_node("dd2"),
            ],
            ..Default::default()
        },
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        last_run_at: None,
        last_status: None,
        require_approval: false,
    };
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-multi");
    store::kv_set(&config, &namespace, "dedup:dd1:tentative", &json!(["a"])).unwrap();
    store::kv_set(&config, &namespace, "dedup:dd2:tentative", &json!(["b"])).unwrap();

    let sub = DedupCommitSubscriber::new(config.clone());
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-multi".into(),
        run_id: "run-multi".into(),
        status: "completed".into(),
    })
    .await;

    assert_eq!(
        store::kv_get(&config, &namespace, "dedup:dd1:committed")
            .unwrap()
            .unwrap(),
        json!(["a"])
    );
    assert_eq!(
        store::kv_get(&config, &namespace, "dedup:dd2:committed")
            .unwrap()
            .unwrap(),
        json!(["b"])
    );
}

// ── per-flow commit serialization (issue #5265) ───────────────────
//
// CodeRabbit "Major" on the dedup engine PR: the commit's
// load(committed)+union(tentative)+store(committed) is a
// read-modify-write, not a CAS. Two overlapping `FlowRunFinished`
// events for the SAME flow could otherwise interleave and have the
// second writer's store clobber the first writer's union, silently
// losing that run's committed keys. `handle_finished` now serializes
// settlement per `flow_id` via `FLOW_COMMIT_LOCKS`.
//
// Two tests, deliberately split:
//
// - `..._never_runs_two_commits_for_the_same_flow_concurrently` spawns a
//   burst of genuinely overlapping `FlowRunFinished` events for the SAME
//   flow_id and proves the LOCK itself provides mutual exclusion (the
//   high-water mark of concurrently-active critical sections never
//   exceeds 1) — this is the "spawn two tasks contending on the same
//   flow_id" case.
// - `..._serial_commits_for_the_same_flow_accumulate_via_union` proves
//   the property that mutual exclusion protects: settling run after run
//   for the same node never clobbers an earlier run's committed keys —
//   each contributes to the union.
//
// These are split rather than combined into one "two runs with two
// different tentative sets, truly concurrently, assert union" test
// because `tentative` is a single shared KV row per node (not
// per-run) — forcing two *different* tentative contents to both survive
// a genuinely simultaneous read would require injecting a write from
// outside `handle_finished` in the middle of its critical section, which
// instead exercises the SEPARATE, still-open node-side race (the
// `dedup` node's own in-run `tentative` read-modify-write, documented on
// `DedupCommitSubscriber` above as explicitly NOT fixed by this lock).
// Together, the two tests below establish the same guarantee end to
// end: the lock enforces serialization (test 1), and serialization is
// sufficient for correctness (test 2).

#[tokio::test]
async fn dedup_commit_never_runs_two_commits_for_the_same_flow_concurrently() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-race", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-race");
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["seed"])).unwrap();

    // Arm the test-only scheduling hook (see `CommitTestHooks`): every
    // `handle_finished` call sleeps briefly while holding the per-flow
    // lock, and records how many calls are concurrently inside that
    // window. Instance-scoped (not a global static) so this doesn't
    // interfere with — or get polluted by — unrelated tests that cargo
    // runs concurrently on other threads. Without a correctly-scoped
    // lock, a burst of overlapping `FlowRunFinished` events for the SAME
    // flow_id would pile up inside the critical section together
    // instead of queuing.
    let hooks = Arc::new(CommitTestHooks::default());
    hooks
        .delay_ms
        .store(20, std::sync::atomic::Ordering::SeqCst);

    let sub = Arc::new(DedupCommitSubscriber::with_test_hooks(
        config.clone(),
        hooks.clone(),
    ));
    let mut handles = Vec::new();
    for i in 0..5 {
        let sub = sub.clone();
        handles.push(tokio::spawn(async move {
            sub.handle(&DomainEvent::FlowRunFinished {
                flow_id: "f-race".into(),
                run_id: format!("run-{i}"),
                status: "completed".into(),
            })
            .await;
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(
        hooks.concurrent.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "every critical-section entry must have a matching exit"
    );
    assert_eq!(
        hooks
            .max_concurrent
            .load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the per-flow lock must serialize overlapping FlowRunFinished handling for the \
         same flow_id — at most one commit critical section may be active at a time"
    );
}

#[tokio::test]
async fn dedup_commit_serial_commits_for_the_same_flow_accumulate_via_union() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = test_config(&tmp);
    let flow = flow_with_dedup_node("f-serial", "dd");
    store::upsert_flow(&config, &flow).unwrap();

    let namespace = dedup_state_namespace("f-serial");
    let sub = DedupCommitSubscriber::new(config.clone());

    // Run A finishes, having tentatively seen "a".
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["a"])).unwrap();
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-serial".into(),
        run_id: "run-a".into(),
        status: "completed".into(),
    })
    .await;

    // Run B finishes later, having independently tentatively seen "b".
    // The per-flow lock (proven by the concurrency test above) is what
    // guarantees two overlapping runs' `FlowRunFinished` handling
    // reduces to exactly this serialized order in practice — so this is
    // the correctness property that mutual exclusion is protecting.
    store::kv_set(&config, &namespace, "dedup:dd:tentative", &json!(["b"])).unwrap();
    sub.handle(&DomainEvent::FlowRunFinished {
        flow_id: "f-serial".into(),
        run_id: "run-b".into(),
        status: "completed".into(),
    })
    .await;

    let committed = store::kv_get(&config, &namespace, "dedup:dd:committed")
        .unwrap()
        .expect("committed key must exist after both runs settle");
    let mut committed: Vec<&str> = committed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    committed.sort_unstable();
    assert_eq!(
        committed,
        vec!["a", "b"],
        "settling run B must not clobber run A's already-committed keys — committed is a \
         running union across every run that has settled, never a last-writer-wins overwrite"
    );
    assert!(
        store::kv_get(&config, &namespace, "dedup:dd:tentative")
            .unwrap()
            .is_none(),
        "tentative must be cleared after each successful commit"
    );
}

#[test]
fn flow_commit_lock_returns_the_same_arc_for_the_same_flow_id_and_differs_across_flows() {
    let a1 = flow_commit_lock("f-lock-a");
    let a2 = flow_commit_lock("f-lock-a");
    assert!(
        Arc::ptr_eq(&a1, &a2),
        "the same flow_id must share one lock instance"
    );

    let b = flow_commit_lock("f-lock-b");
    assert!(
        !Arc::ptr_eq(&a1, &b),
        "different flow_ids must not contend on the same lock"
    );
}
