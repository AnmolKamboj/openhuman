use super::*;
use tempfile::TempDir;

fn test_gate() -> (ApprovalGate, TempDir) {
    let dir = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    // Mirrors the `session-<uuid>` shape minted by
    // `bootstrap_core_runtime` in production so the
    // `debug_assert!` regression guard in `ApprovalGate::new`
    // doesn't trip in tests.
    let session = format!("session-{}", uuid::Uuid::new_v4());
    // 500ms TTL was racing the 50×10ms poll loop on slow CI
    // runners — the row would expire (and get denied by
    // list_pending's lazy-expire) before `decide` could fire,
    // surfacing as "pending row never appeared". 2s gives the
    // polling tests enough headroom while keeping
    // `timeout_returns_deny` fast (PR #2367 CI flake).
    let gate = ApprovalGate::new(config, session, Duration::from_secs(2));
    (gate, dir)
}

/// A chat context — the gate only parks within a live chat turn now, so
/// tests that exercise parking must run intercept inside this scope.
fn chat_ctx() -> ApprovalChatContext {
    ApprovalChatContext {
        thread_id: "t-test".into(),
        client_id: "c-test".into(),
    }
}

/// A matching web-chat origin for the chat context fixture. Tests
/// exercising the parking flow scope BOTH task-locals — production
/// callers in `web_chat` do the same.
fn web_origin() -> AgentTurnOrigin {
    AgentTurnOrigin::WebChat {
        thread_id: "t-test".into(),
        client_id: "c-test".into(),
        request_id: Some("req-test".into()),
    }
}

#[test]
fn guard_cleanup_only_clears_routing_it_still_owns() {
    // Regression for #4774: on external turn teardown a replacement turn may
    // have already parked a new approval on the same thread and
    // overwritten the routing entry. The dropped guard for the *old* request
    // must not clobber the *new* request's mapping.
    let (gate, _dir) = test_gate();

    gate.thread_to_request
        .lock()
        .insert("thread-1".into(), "req-new".into());

    // Stale guard for the superseded request is a no-op.
    gate.clear_thread_route_if_owned("thread-1", "req-old");
    assert_eq!(
        gate.pending_for_thread("thread-1").as_deref(),
        Some("req-new")
    );

    // The owning request's guard clears its own routing.
    gate.clear_thread_route_if_owned("thread-1", "req-new");
    assert!(gate.pending_for_thread("thread-1").is_none());
}

#[tokio::test]
async fn approve_once_returns_allow() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept("composio", "send slack", serde_json::json!({})),
            ),
        )
        .await
    });

    // Wait for pending row to land.
    let mut tries = 0;
    let pending = loop {
        let list = gate.list_pending().unwrap();
        if let Some(p) = list.into_iter().next() {
            break p;
        }
        tries += 1;
        assert!(tries < 50, "pending row never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    gate.decide(&pending.request_id, ApprovalDecision::ApproveOnce)
        .unwrap();

    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
}

#[tokio::test]
async fn deny_returns_deny_with_reason() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept("pushover", "send push", serde_json::json!({})),
            ),
        )
        .await
    });

    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    gate.decide(&pending.request_id, ApprovalDecision::Deny)
        .unwrap();

    let outcome = handle.await.unwrap();
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("pushover")),
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn aborting_older_chat_waiter_preserves_newer_thread_route() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let old_gate = gate.clone();
    let old_handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                old_gate.intercept("composio", "old action", serde_json::json!({})),
            ),
        )
        .await
    });

    let mut tries = 0;
    let old_request_id = loop {
        if let Some(request_id) = gate.pending_for_thread("t-test") {
            break request_id;
        }
        tries += 1;
        assert!(tries < 1_000, "old chat approval route never appeared");
        tokio::task::yield_now().await;
    };

    let new_gate = gate.clone();
    let new_handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                new_gate.intercept("composio", "new action", serde_json::json!({})),
            ),
        )
        .await
    });

    let mut tries = 0;
    let new_request_id = loop {
        if let Some(request_id) = gate.pending_for_thread("t-test") {
            if request_id != old_request_id {
                break request_id;
            }
        }
        tries += 1;
        assert!(tries < 1_000, "new chat approval route never appeared");
        tokio::task::yield_now().await;
    };

    old_handle.abort();
    assert!(old_handle.await.unwrap_err().is_cancelled());

    assert_eq!(
        gate.pending_for_thread("t-test").as_deref(),
        Some(new_request_id.as_str())
    );
    assert!(!gate.waiters.lock().contains_key(&old_request_id));
    assert!(gate.waiters.lock().contains_key(&new_request_id));
    assert_eq!(
        store::get_decision(&gate.config, &old_request_id).unwrap(),
        Some(ApprovalDecision::Deny)
    );

    gate.decide(&new_request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    assert!(matches!(new_handle.await.unwrap(), GateOutcome::Allow));
    assert!(gate.pending_for_thread("t-test").is_none());
}

#[tokio::test]
async fn auto_approve_tool_skips_prompt() {
    // The gate reads the "Always allow" allowlist from the process-global
    // live policy. Serialize with the other tests that install/reload it
    // (the `live_policy` module test + the autonomy `ops` tests, which all
    // take this same lock) so a parallel install can't clobber ours mid-test.
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();

    // A tool name unique to this test so leaving it in the global allowlist
    // afterwards can't make a sibling gate test (which use "composio" /
    // "pushover") skip its expected prompt.
    let tool = "openhuman_test_always_allow_tool";
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve: vec![tool.into()],
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    crate::openhuman::security::live_policy::install(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    // An allow-listed tool short-circuits the gate to `Allow` immediately —
    // before any parking — even with a live chat context present, and
    // without persisting a pending row. The shortcut runs regardless of
    // origin (it's the user's persisted "Always allow" allowlist), so we
    // do not need to scope an origin for this case.
    let outcome = APPROVAL_CHAT_CONTEXT
        .scope(
            chat_ctx(),
            gate.intercept(tool, "noop", serde_json::json!({})),
        )
        .await;
    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "an auto-approved call must not create a pending approval row"
    );
}

/// With `auto_approve_all: true`, a WebChat-origin call resolves to
/// `Allow` immediately — no pending row is created and the chat context
/// is never consulted, proving the short-circuit fires above the park.
#[tokio::test]
async fn auto_approve_all_resolves_allow() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    // Scoped: restores whatever live_policy held before this test on drop
    // (including on panic), so a leaked `auto_approve_all: true` can never
    // reach a sibling gate test that doesn't hold `TEST_ENV_LOCK`.
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    let outcome = turn_origin::with_origin(
        web_origin(),
        gate.intercept("openhuman_test_aaa_webchat", "noop", serde_json::json!({})),
    )
    .await;

    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "auto_approve_all must short-circuit before any pending row is persisted"
    );
}

/// Control test: with `auto_approve_all: false` (the default), a
/// WebChat-origin call parks normally — it does NOT resolve to `Allow`
/// until a decision is sent on the oneshot.
#[tokio::test]
async fn auto_approve_all_off_still_parks() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: false,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept("openhuman_test_aaa_off", "noop", serde_json::json!({})),
            ),
        )
        .await
    });

    // The call must actually park: poll for the pending row instead of
    // racing an immediate result.
    let mut tries = 0;
    let pending = loop {
        let rows = gate.list_pending().unwrap();
        if let Some(p) = rows.into_iter().next() {
            break p;
        }
        tries += 1;
        assert!(
            tries < 50,
            "pending row never appeared — call resolved without parking"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    gate.decide(&pending.request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
}

/// `auto_approve_all: true` must NOT override a `SubconsciousTainted`
/// origin — the gate still hard-denies it (indirect prompt injection
/// defense).
#[tokio::test]
async fn auto_approve_all_does_not_override_subconscioustainted() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "job-tainted".into(),
        source: TrustedAutomationSource::SubconsciousTainted,
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("openhuman_test_aaa_tainted", "noop", serde_json::json!({})),
    )
    .await;

    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("external-sync")),
        other => panic!("expected deny, got {other:?}"),
    }
}

/// `auto_approve_all: true` must NOT override an `Unknown` origin — the
/// gate still fails closed for unlabelled call sites.
#[tokio::test]
async fn auto_approve_all_does_not_override_unknown() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    // No `with_origin` scope at all — mirrors an unlabelled call site,
    // which `turn_origin::current()` maps to `AgentTurnOrigin::Unknown`.
    let outcome = gate
        .intercept("openhuman_test_aaa_unknown", "noop", serde_json::json!({}))
        .await;

    match outcome {
        // The deny message is specific and actionable (issues #5508 / #5499,
        // 2nd acceptance criterion): it names the missing origin label, calls
        // out the scheduling/external-effect tools it affects, and frames it
        // as an internal wiring gap rather than user error.
        GateOutcome::Deny { reason } => {
            assert!(reason.contains("origin label"), "reason was: {reason}");
            assert!(reason.contains("cron_add"), "reason was: {reason}");
            assert!(reason.contains("external-effect"), "reason was: {reason}");
        }
        other => panic!("expected deny, got {other:?}"),
    }
}

/// `auto_approve_all: true` overrides the `GoalContinuation` bypass —
/// normally that origin skips the per-tool allowlist and always parks,
/// but the blanket bypass sits above that check and allows immediately.
#[tokio::test]
async fn auto_approve_all_overrides_bypass_shortcut() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "goal-1".into(),
        source: TrustedAutomationSource::GoalContinuation,
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("openhuman_test_aaa_goal", "noop", serde_json::json!({})),
    )
    .await;

    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "auto_approve_all must short-circuit before any pending row is persisted"
    );
}

/// `auto_approve_all: true` overrides a `Workflow { require_approval: true }`
/// origin — normally the user's per-flow "gate every action" choice forces
/// a park, but the blanket bypass sits above that check too.
#[tokio::test]
async fn auto_approve_all_overrides_require_approval_workflow() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "flow-1".into(),
        source: TrustedAutomationSource::Workflow {
            require_approval: true,
        },
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("openhuman_test_aaa_workflow", "noop", serde_json::json!({})),
    )
    .await;

    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "auto_approve_all must short-circuit before any pending row is persisted"
    );
}

/// The `auto_approve_all` × remote-origin-triage interaction, pinned by
/// name because it is a **decision**, not an emergent behaviour.
///
/// Since openhuman#5634 a Composio/webhook payload reaching
/// `triage.escalate` carries `Workflow { require_approval: true }`, so
/// normally it parks and writes a `pending_approvals` row — that is
/// `a_remote_triage_escalation_parks_with_an_audit_row_rather_than_an_unknown_denial`
/// above. With `auto_approve_all` on it is allowed immediately and writes
/// no row, which means for those users #5634 moved this path from
/// `Unknown` → hard Deny to Allow-with-no-audit-trail.
///
/// The gate owner accepted that rather than carving out an exception:
/// https://github.com/tinyhumansai/openhuman/issues/5634#issuecomment-5396604125
///
/// So this test exists to be *broken on purpose*. If a future change adds
/// this origin to the bypass exclusion list, this fails, and whoever is
/// making that change has to reopen the decision instead of discovering the
/// behaviour by accident. Deleting it to make a change pass is the one
/// wrong response.
#[tokio::test]
async fn auto_approve_all_allows_a_remote_triage_dispatch_without_an_audit_row() {
    use crate::openhuman::agent::triage::{remote_trigger_origin, TriggerEnvelope};

    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, dir) = test_gate();
    let policy = crate::openhuman::security::SecurityPolicy {
        auto_approve_all: true,
        ..crate::openhuman::security::SecurityPolicy::default()
    };
    let _policy_guard = crate::openhuman::security::live_policy::install_scoped(
        Arc::new(policy),
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
    );

    let envelope = TriggerEnvelope::from_composio(
        "gmail",
        "new_message",
        "ti_meta",
        "ti_bCCTKZlajKi4",
        serde_json::json!({ "subject": "hello" }),
    );

    let outcome = turn_origin::with_origin(
        remote_trigger_origin(&envelope),
        gate.intercept(
            "triage.escalate",
            "escalate to orchestrator",
            serde_json::json!({}),
        ),
    )
    .await;

    assert!(
        matches!(outcome, GateOutcome::Allow),
        "auto_approve_all opts into the bypass globally, including remote triage;              got {outcome:?}"
    );
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "the bypass short-circuits before the park, so no pending_approvals row is              written — this is the documented cost of the flag, not a defect"
    );
}

#[tokio::test]
async fn timeout_returns_deny() {
    let (gate, _dir) = test_gate(); // TTL = 500ms
    let gate = Arc::new(gate);
    let outcome = turn_origin::with_origin(
        web_origin(),
        APPROVAL_CHAT_CONTEXT.scope(
            chat_ctx(),
            gate.intercept("composio", "timed out", serde_json::json!({})),
        ),
    )
    .await;
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("timed out")),
        other => panic!("expected deny, got {other:?}"),
    }
}

/// T-M3 (flows `cancel_flow_run`): the gate has no special-casing per tool
/// name — any call intercepted under a chat origin/context with no
/// matching auto-allowlist entry parks and, absent a human decision,
/// times out to `Deny` rather than executing. This pins that
/// `cancel_flow_run` — now that `builder_tools::CancelFlowRunTool`
/// reports `external_effect() == true` (T-M3) so
/// `ApprovalSecurityMiddleware` routes it through exactly this call —
/// genuinely parks for a real approval decision instead of running
/// unapproved, mirroring `timeout_returns_deny` above.
#[tokio::test]
async fn cancel_flow_run_parks_for_approval_when_a_gate_is_present() {
    let (gate, _dir) = test_gate(); // TTL = 500ms
    let gate = Arc::new(gate);
    let outcome = turn_origin::with_origin(
        web_origin(),
        APPROVAL_CHAT_CONTEXT.scope(
            chat_ctx(),
            gate.intercept(
                "cancel_flow_run",
                "cancel run r-1 of flow f-1",
                serde_json::json!({ "flow_id": "f-1", "run_id": "r-1" }),
            ),
        ),
    )
    .await;
    // No decision ever arrives — the call must NOT auto-execute. It
    // parks until the gate's TTL elapses, then denies (never `Allow`).
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("timed out")),
        other => {
            panic!("expected the parked cancel_flow_run call to time out to Deny, got {other:?}")
        }
    }
}

#[tokio::test]
async fn decide_unknown_id_is_noop() {
    let (gate, _dir) = test_gate();
    let decided = gate
        .decide("does-not-exist", ApprovalDecision::ApproveOnce)
        .unwrap();
    assert!(decided.is_none());
}

/// TAURI-RUST-5EH: a `decide` miss must be classified — already-decided and
/// expired rows are benign (`AlreadyResolved`), while an id that was never
/// persisted is a genuine lost registration (`NeverRegistered`) that stays a
/// Sentry signal.
#[tokio::test]
async fn classify_decide_miss_distinguishes_resolved_from_unknown() {
    let (gate, _dir) = test_gate();

    // Never persisted → genuine loss, keep visible.
    assert_eq!(
        gate.classify_decide_miss("never-existed"),
        DecideMiss::NeverRegistered
    );

    // Persist + decide a row, then a second decide misses → already-decided.
    let pending = PendingApproval::new(
        "req-decided",
        "composio",
        "send email",
        serde_json::json!({}),
        Some(chrono::Utc::now() + chrono::Duration::minutes(10)),
    );
    store::insert_pending(&gate.config, &pending, &gate.session_id).unwrap();
    assert!(gate
        .decide("req-decided", ApprovalDecision::ApproveOnce)
        .unwrap()
        .is_some());
    // The conditional UPDATE now matches 0 rows (decided_at set).
    assert!(gate
        .decide("req-decided", ApprovalDecision::Deny)
        .unwrap()
        .is_none());
    assert_eq!(
        gate.classify_decide_miss("req-decided"),
        DecideMiss::AlreadyResolved
    );

    // A row past its expiry is lazily denied by `decide`'s expire pass, so
    // its decide miss is also benign (the persisted decision exists).
    let expired = PendingApproval::new(
        "req-expired",
        "composio",
        "send email",
        serde_json::json!({}),
        Some(chrono::Utc::now() - chrono::Duration::minutes(1)),
    );
    store::insert_pending(&gate.config, &expired, &gate.session_id).unwrap();
    assert!(gate
        .decide("req-expired", ApprovalDecision::ApproveOnce)
        .unwrap()
        .is_none());
    assert_eq!(
        gate.classify_decide_miss("req-expired"),
        DecideMiss::AlreadyResolved
    );
}

#[tokio::test]
async fn pending_for_thread_tracks_request_under_chat_context_and_clears() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // Run intercept inside a scoped chat context + matching WebChat
    // origin (as the web channel does in production).
    let g = gate.clone();
    let ctx = ApprovalChatContext {
        thread_id: "thread-42".into(),
        client_id: "client-1".into(),
    };
    let origin = AgentTurnOrigin::WebChat {
        thread_id: "thread-42".into(),
        client_id: "client-1".into(),
        request_id: Some("req-42".into()),
    };
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            origin,
            APPROVAL_CHAT_CONTEXT.scope(ctx, g.intercept("shell", "run ls", serde_json::json!({}))),
        )
        .await
    });

    // While parked, the thread → request mapping is queryable.
    let mut tries = 0;
    let request_id = loop {
        if let Some(r) = gate.pending_for_thread("thread-42") {
            break r;
        }
        tries += 1;
        assert!(tries < 50, "thread mapping never appeared");
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    // Decide via the mapped request_id (as the chat ingress router will).
    gate.decide(&request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    assert!(matches!(handle.await.unwrap(), GateOutcome::Allow));

    // Mapping is cleared once intercept returns.
    assert!(gate.pending_for_thread("thread-42").is_none());
}

/// Regression for #5499: an async-delegated sub-agent carries the `WebChat`
/// origin across the `tokio::spawn` boundary (`spawn_async_subagent` calls
/// `turn_origin::propagate`) but NOT the `APPROVAL_CHAT_CONTEXT` task-local.
/// Before the origin-routing fallback the gate parked with `thread_id:
/// None`, the web-channel surface dropped the `ApprovalRequested` event
/// ("thread/client absent — NOT surfacing"), and the park silently
/// TTL-denied — so a `cron_add` the user asked for in chat never completed.
/// The gate must instead route the park via the thread/client the `WebChat`
/// origin already carries, so the card can surface and be approved.
#[tokio::test]
async fn webchat_origin_routes_park_when_approval_chat_context_absent() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // WebChat origin scoped, but NO `APPROVAL_CHAT_CONTEXT` — exactly the
    // async sub-agent spawn state (origin propagated, approval context not).
    let g = gate.clone();
    let origin = AgentTurnOrigin::WebChat {
        thread_id: "thread-async".into(),
        client_id: "client-async".into(),
        request_id: Some("req-async".into()),
    };
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            origin,
            g.intercept("cron_add", "schedule daily reminder", serde_json::json!({})),
        )
        .await
    });

    // The park must be routable via the origin's thread even though the
    // approval task-local was never scoped. `thread_to_request` is inserted
    // only when `chat_thread_id` is `Some`, so this mapping appearing proves
    // the origin fallback supplied it.
    let mut tries = 0;
    let request_id = loop {
        if let Some(r) = gate.pending_for_thread("thread-async") {
            break r;
        }
        tries += 1;
        assert!(
            tries < 50,
            "park must be routable via the WebChat origin's thread when \
             APPROVAL_CHAT_CONTEXT is absent (#5499)"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    // A decision on that mapped request resolves the park (the card can
    // surface and be approved), instead of silently TTL-denying.
    gate.decide(&request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    assert!(matches!(handle.await.unwrap(), GateOutcome::Allow));
    assert!(gate.pending_for_thread("thread-async").is_none());
}

#[tokio::test]
async fn waiter_future_dropped_mid_park_evicts_waiter_clears_routing_and_denies_row() {
    // #4774: once a turn future can be torn down *externally* (the #4746
    // harness wall-clock backstop / #4751 outer web backstop firing while a
    // tool call is parked), dropping the intercept future must not leak the
    // waiter, the thread→request routing mapping, or the still-open pending
    // row. The `WaiterGuard` Drop impl runs the cleanup the timeout match
    // arms would otherwise own.
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // Build the parked future with the WebChat origin + chat context scoped,
    // exactly like the production web channel caller — but drive it locally
    // so we can drop it mid-park instead of resolving it.
    let g = gate.clone();
    // `Box::pin` (not `tokio::pin!`) so `drop(fut)` below drops the *future
    // itself* — and thus the `WaiterGuard` saved in its async state — rather
    // than just a `Pin<&mut _>` reference.
    let mut fut = Box::pin(turn_origin::with_origin(
        web_origin(),
        APPROVAL_CHAT_CONTEXT.scope(
            chat_ctx(),
            g.intercept("shell", "run rm", serde_json::json!({})),
        ),
    ));

    // Poll it just long enough to register the waiter, persist the pending
    // row, and park on the TTL timeout. Nothing resolves it, so the outer
    // timeout must elapse with the future still pending.
    let parked = tokio::time::timeout(Duration::from_millis(200), &mut fut).await;
    assert!(
        parked.is_err(),
        "future should still be parked, not resolved"
    );

    // Capture the request_id from the routing mapping while parked, and
    // confirm the waiter + pending row exist before teardown.
    let request_id = gate
        .pending_for_thread("t-test")
        .expect("thread→request mapping must exist while parked");
    assert!(
        gate.waiters.lock().contains_key(&request_id),
        "waiter must be registered while parked"
    );
    assert!(
        matches!(store::get_decision(&gate.config, &request_id), Ok(None)),
        "pending row must be open (undecided) while parked"
    );

    // External teardown: the wall-clock backstop tears the turn future down
    // mid-park. This skips the timeout match arms entirely.
    drop(fut);

    // The RAII guard must have run the cleanup on drop.
    assert!(
        !gate.waiters.lock().contains_key(&request_id),
        "waiter must be evicted when the parked future is dropped"
    );
    assert!(
        gate.pending_for_thread("t-test").is_none(),
        "thread→request routing must be cleared on external teardown"
    );
    assert!(
        matches!(
            store::get_decision(&gate.config, &request_id),
            Ok(Some(ApprovalDecision::Deny))
        ),
        "pending row must be denied when the parked future is dropped"
    );
}

// ── caller park bound (issue #4756) ──────────────────────────────
//
// A caller (composio_connect) can cap the park via
// `intercept_audited_bounded`. When the bound elapses before the gate's own
// TTL the gate must abandon the park cancellation-safely: return `None`,
// clear the thread→request routing so a later reply is not mis-routed (the
// codex concern), yet LEAVE the `pending_approvals` row open so a later
// card-click still resolves it in the DB.
#[tokio::test]
async fn intercept_audited_bounded_abandons_park_and_leaves_row_pending() {
    let (gate, _dir) = test_gate(); // boot-time TTL = 2s
    let gate = Arc::new(gate);

    let g = gate.clone();
    let ctx = ApprovalChatContext {
        thread_id: "thread-bound".into(),
        client_id: "client-1".into(),
    };
    let origin = AgentTurnOrigin::WebChat {
        thread_id: "thread-bound".into(),
        client_id: "client-1".into(),
        request_id: Some("req-bound".into()),
    };
    // 100ms caller bound — far below the 2s gate TTL — so the bound is what
    // elapses, not the gate's own timeout.
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            origin,
            APPROVAL_CHAT_CONTEXT.scope(
                ctx,
                g.intercept_audited_bounded(
                    "shell",
                    "run ls",
                    serde_json::json!({}),
                    Some(Duration::from_millis(100)),
                ),
            ),
        )
        .await
    });

    // While parked, the thread → request mapping is queryable.
    let mut tries = 0;
    let request_id = loop {
        if let Some(r) = gate.pending_for_thread("thread-bound") {
            break r;
        }
        tries += 1;
        assert!(tries < 50, "thread mapping never appeared");
        tokio::time::sleep(Duration::from_millis(5)).await;
    };

    // The bound elapses → `None`, so the caller renders its own fast path
    // instead of the park resolving to a Deny.
    let resolved = handle.await.unwrap();
    assert!(
        resolved.is_none(),
        "caller park bound must surface as None, not a resolved outcome"
    );

    // Routing is cleared so a later reply is not mis-routed to the abandoned
    // request (the codex #4756 concern).
    assert!(
        gate.pending_for_thread("thread-bound").is_none(),
        "thread → request mapping must be cleared on caller-bound abandon"
    );

    // The row is LEFT open — a later human card-click still resolves it.
    let decided = gate
        .decide(&request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    assert!(
        decided.is_some(),
        "pending row must survive the abandon so a later card-click resolves it"
    );
}

/// Tests for `effective_ttl` env-override parsing.
///
/// These run serially (they mutate the process env) via the shared
/// `TEST_ENV_LOCK`; the lock is the same one used by `auto_approve_tool_skips_prompt`
/// and the live_policy tests so they cannot clobber each other in parallel.
///
/// Guarded on `debug_assertions`: the override is compiled out of release
/// builds, so this assertion only holds under `cargo test` (debug). The
/// fallback tests below hold in either build.
#[cfg(debug_assertions)]
#[test]
fn effective_ttl_uses_env_override_when_valid() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, _dir) = test_gate(); // boot-time TTL = 2s
    unsafe { std::env::set_var("OPENHUMAN_APPROVAL_TTL_SECS", "42") };
    assert_eq!(
        gate.effective_ttl(),
        Duration::from_secs(42),
        "valid OPENHUMAN_APPROVAL_TTL_SECS must override boot-time TTL"
    );
    unsafe { std::env::remove_var("OPENHUMAN_APPROVAL_TTL_SECS") };
}

#[test]
fn effective_ttl_falls_back_to_boot_ttl_for_garbage_value() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, _dir) = test_gate(); // boot-time TTL = 2s
    unsafe { std::env::set_var("OPENHUMAN_APPROVAL_TTL_SECS", "not-a-number") };
    assert_eq!(
        gate.effective_ttl(),
        Duration::from_secs(2),
        "garbage OPENHUMAN_APPROVAL_TTL_SECS must fall back to boot-time TTL"
    );
    unsafe { std::env::remove_var("OPENHUMAN_APPROVAL_TTL_SECS") };
}

#[test]
fn effective_ttl_falls_back_to_boot_ttl_when_unset() {
    let _env = crate::openhuman::config::TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (gate, _dir) = test_gate(); // boot-time TTL = 2s
    unsafe { std::env::remove_var("OPENHUMAN_APPROVAL_TTL_SECS") };
    assert_eq!(
        gate.effective_ttl(),
        Duration::from_secs(2),
        "unset OPENHUMAN_APPROVAL_TTL_SECS must fall back to boot-time TTL"
    );
}

/// Tests for `resolve_park_ttl` — the pure clamp-selection helper behind
/// the copilot-streaming TTL shortening (fix/flows-copilot-approval-ttl).
/// Exercised directly (rather than by actually parking + waiting out a
/// multi-minute TTL) so the assertions stay fast and deterministic.
mod resolve_park_ttl_tests {
    use super::*;

    #[test]
    fn default_park_keeps_the_full_ttl() {
        let default_ttl = DEFAULT_APPROVAL_TTL;
        assert_eq!(
            ApprovalGate::resolve_park_ttl(default_ttl, false),
            default_ttl,
            "a plain park (no copilot stream) must not be clamped"
        );
    }

    #[test]
    fn copilot_stream_shortens_a_default_ten_minute_park() {
        let default_ttl = DEFAULT_APPROVAL_TTL;
        assert_eq!(
            ApprovalGate::resolve_park_ttl(default_ttl, true),
            COPILOT_APPROVAL_TTL,
            "a flows_build copilot-streaming park must clamp to COPILOT_APPROVAL_TTL"
        );
        assert!(
            COPILOT_APPROVAL_TTL < DEFAULT_APPROVAL_TTL,
            "the copilot clamp must actually be shorter than the default TTL"
        );
    }

    #[test]
    fn a_clamp_never_extends_a_shorter_boot_time_ttl() {
        // Mirrors production's env-override guard: a clamp may only
        // narrow, never widen, the gate's own effective TTL (e.g. a
        // debug-only `OPENHUMAN_APPROVAL_TTL_SECS=60` override that is
        // already shorter than either clamp).
        let short_ttl = Duration::from_secs(60);
        assert_eq!(
            ApprovalGate::resolve_park_ttl(short_ttl, true),
            short_ttl,
            "copilot clamp must not extend a boot-time TTL that is already shorter"
        );
    }
}

/// Integration regression test for the streaming-to-gate contract
/// (CodeRabbit review on PR #5112): `resolve_park_ttl` is covered directly
/// above, but that alone doesn't prove `intercept_audited_inner` actually
/// persists the clamped TTL when the copilot-streaming context is scoped.
/// Builds a gate with the full `DEFAULT_APPROVAL_TTL` boot TTL (unlike
/// `test_gate()`'s 2s, which is already shorter than either clamp and
/// would make this assertion vacuous), scopes
/// `APPROVAL_COPILOT_STREAM_CONTEXT` alongside the chat context + WebChat
/// origin the way `flows::ops::flows_build` does in production, and
/// inspects the persisted `expires_at` on the pending row.
#[tokio::test]
async fn copilot_streaming_park_persists_the_clamped_expiry() {
    let dir = TempDir::new().unwrap();
    let config = Config {
        workspace_dir: dir.path().to_path_buf(),
        ..Config::default()
    };
    let session = format!("session-{}", uuid::Uuid::new_v4());
    let gate = ApprovalGate::new(config, session, DEFAULT_APPROVAL_TTL);
    let gate = Arc::new(gate);

    let before = chrono::Utc::now();
    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                APPROVAL_COPILOT_STREAM_CONTEXT.scope(
                    (),
                    g.intercept("composio", "send slack", serde_json::json!({})),
                ),
            ),
        )
        .await
    });

    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::task::yield_now().await;
    };

    let expires_at = pending
        .expires_at
        .expect("a parked approval always sets expires_at");
    let ttl_persisted = expires_at - before;
    assert!(
        ttl_persisted
            <= chrono::Duration::from_std(COPILOT_APPROVAL_TTL).unwrap()
                + chrono::Duration::seconds(5),
        "copilot-streaming park must persist an expires_at clamped to COPILOT_APPROVAL_TTL \
         (180s), not the gate's full {:?} boot TTL — got a {ttl_persisted} window",
        DEFAULT_APPROVAL_TTL
    );
    assert!(
        ttl_persisted < chrono::Duration::from_std(DEFAULT_APPROVAL_TTL).unwrap(),
        "sanity: the persisted expiry must be shorter than the unclamped default TTL"
    );

    gate.decide(&pending.request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
}

#[test]
fn parse_approval_reply_maps_yes_no_and_rejects_other() {
    for y in ["yes", "Y", " OK ", "approve", "Allow", "okay"] {
        assert_eq!(
            super::parse_approval_reply(y),
            Some(ApprovalDecision::ApproveOnce),
            "{y}"
        );
    }
    for n in ["no", "N", "deny", "Denied"] {
        assert_eq!(
            super::parse_approval_reply(n),
            Some(ApprovalDecision::Deny),
            "{n}"
        );
    }
    // Anything else is NOT an answer → caller cancels + redirects.
    for other in [
        "maybe",
        "actually do Y instead",
        "",
        "yep nope",
        "sure thing",
    ] {
        assert_eq!(super::parse_approval_reply(other), None, "{other}");
    }
}

/// openhuman#5634: the six triage dispatch sites scoped no origin, so every
/// proactive escalation reached this gate as `Unknown` and was refused —
/// `intercept_with_unknown_origin_denies` below is that behaviour.
///
/// A remote trigger now carries
/// `TrustedAutomation { Workflow { require_approval: true } }`, which parks
/// and persists the `pending_approvals` row instead. This asserts the park
/// and the row, not a successful escalation: with no surface able to decide
/// a background park these still TTL-deny (openhuman#5746). The gain is the
/// audit trail, not restored function.
#[tokio::test]
async fn a_remote_triage_escalation_parks_with_an_audit_row_rather_than_an_unknown_denial() {
    use crate::openhuman::agent::triage::{remote_trigger_origin, TriggerEnvelope};

    let (gate, _dir) = test_gate();
    let envelope = TriggerEnvelope::from_composio(
        "gmail",
        "new_message",
        "ti_meta",
        "ti_bCCTKZlajKi4",
        serde_json::json!({ "subject": "hello" }),
    );

    // `Box::pin` + a short timeout drives the future into the park without
    // waiting out the TTL; nothing decides it, so it must still be pending.
    let mut fut = Box::pin(turn_origin::with_origin(
        remote_trigger_origin(&envelope),
        gate.intercept(
            "triage.escalate",
            "escalate to orchestrator",
            serde_json::json!({}),
        ),
    ));
    let parked = tokio::time::timeout(Duration::from_millis(300), &mut fut).await;
    assert!(
        parked.is_err(),
        "a remote escalation must park for a decision, not resolve immediately \
         (an immediate Deny here is the `Unknown` regression this pins)"
    );

    let pending = gate.list_pending().unwrap();
    assert_eq!(
        pending.len(),
        1,
        "the park must persist exactly one pending_approvals row, got {pending:?}"
    );
    assert_eq!(pending[0].tool_name, "triage.escalate");
}

/// The counterpart: a locally initiated triage dispatch keeps the authority
/// its caller already had, so it is allowed without a prompt and writes no
/// row. Pinned alongside the remote case because the security decision on
/// openhuman#5634 is that these two are *different*, and a later
/// simplification to one blanket label would have to break one of them.
#[tokio::test]
async fn a_local_triage_escalation_is_allowed_without_a_prompt() {
    use crate::openhuman::agent::triage::local_trigger_origin;

    let (gate, _dir) = test_gate();
    let outcome = turn_origin::with_origin(
        local_trigger_origin(),
        gate.intercept(
            "triage.escalate",
            "escalate to orchestrator",
            serde_json::json!({}),
        ),
    )
    .await;

    assert!(
        matches!(outcome, GateOutcome::Allow),
        "a locally initiated escalation must not be gated, got {outcome:?}"
    );
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "a trust-root origin persists no pending row"
    );
}

#[tokio::test]
async fn intercept_with_unknown_origin_denies() {
    // Unlabelled call site (no origin scope) maps to `Unknown` and is
    // rejected. This replaces the previous "no chat context → Allow"
    // legacy behaviour: the gate now refuses to execute external_effect
    // tools from unlabelled call sites.
    let (gate, _dir) = test_gate();
    let outcome = gate
        .intercept("shell", "run ls", serde_json::json!({}))
        .await;
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("origin label")),
        other => panic!("expected deny, got {other:?}"),
    }
    assert!(gate.pending_for_thread("thread-42").is_none());
}

#[tokio::test]
async fn intercept_with_trusted_cron_origin_allows_without_prompt() {
    // Cron jobs the user explicitly authorized run trusted automation;
    // the gate allows without prompt and does not persist a row.
    let (gate, _dir) = test_gate();
    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "cron-42".into(),
        source: TrustedAutomationSource::Cron,
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("shell", "run ls", serde_json::json!({})),
    )
    .await;
    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "trusted cron must not persist a pending row"
    );
}

#[tokio::test]
async fn intercept_with_workflow_origin_trust_root_allows_without_prompt() {
    // A saved+enabled flow's pre-declared tool/HTTP action (trust root,
    // `require_approval: false`) is allowed without a prompt.
    let (gate, _dir) = test_gate();
    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "flow-1".into(),
        source: TrustedAutomationSource::Workflow {
            require_approval: false,
        },
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("composio", "post to slack", serde_json::json!({})),
    )
    .await;
    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "a trusted workflow action must not persist a pending row"
    );
}

#[tokio::test]
async fn intercept_with_workflow_require_approval_persists_and_ttl_denies() {
    // A per-flow `require_approval: true` toggle forces every external
    // action through the HITL gate even though the origin carries a
    // trust root — same conservative park-and-audit shape as
    // `GoalContinuation` / `ExternalChannel`, since there is no flow
    // review surface to route the prompt to yet (B3).
    let (gate, _dir) = test_gate(); // 2s TTL
    let gate = Arc::new(gate);
    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "flow-2".into(),
        source: TrustedAutomationSource::Workflow {
            require_approval: true,
        },
    };

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            origin,
            g.intercept("composio", "post to slack", serde_json::json!({})),
        )
        .await
    });

    let mut tries = 0;
    loop {
        if !gate.list_pending().unwrap().is_empty() {
            break;
        }
        tries += 1;
        assert!(
            tries < 50,
            "audit row never appeared for require_approval workflow origin"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let outcome = handle.await.unwrap();
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("timed out")),
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn intercept_with_trusted_subconscious_origin_allows_without_prompt() {
    // Subconscious ticks on internal-only memory are trusted automation
    // and run unprompted (preserves pre-PR behavior for the safe case).
    let (gate, _dir) = test_gate();
    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "subconscious-tick".into(),
        source: TrustedAutomationSource::Subconscious,
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("shell", "run ls", serde_json::json!({})),
    )
    .await;
    assert!(matches!(outcome, GateOutcome::Allow));
}

#[tokio::test]
async fn intercept_with_subconscious_tainted_origin_denies() {
    // A subconscious tick whose memory context contains external-sync
    // chunks is rejected for external_effect tools — external text in
    // memory could otherwise steer the tick into a tool call.
    let (gate, _dir) = test_gate();
    let origin = AgentTurnOrigin::TrustedAutomation {
        job_id: "subconscious-tainted".into(),
        source: TrustedAutomationSource::SubconsciousTainted,
    };
    let outcome = turn_origin::with_origin(
        origin,
        gate.intercept("send_email", "send", serde_json::json!({})),
    )
    .await;
    match outcome {
        GateOutcome::Deny { reason } => {
            assert!(reason.contains("external-sync"), "reason was: {reason}")
        }
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn intercept_with_cli_origin_allows_without_prompt() {
    // CLI / one-off internal callers (sub-agent invocations, scripts)
    // are allowed through unprompted — there is no chat surface to
    // park on, and the legacy CLI workflow assumes the operator
    // authorized the invocation.
    let (gate, _dir) = test_gate();
    let outcome = turn_origin::with_origin(
        AgentTurnOrigin::Cli,
        gate.intercept("shell", "run ls", serde_json::json!({})),
    )
    .await;
    assert!(matches!(outcome, GateOutcome::Allow));
}

/// Regression for #5508 / #5499: an external-effect scheduling tool
/// (`cron_add`) that runs on a freshly-spawned, turn-less task — the exact
/// shape of a hosted effect executor, which
/// fires the local sub-agent from a bare `tokio::spawn` with no agent turn on
/// the stack — must NOT be `Unknown`-denied once the spawn site scopes an
/// explicit `AgentTurnOrigin::Cli` (the residual site PR #5465 did not cover).
///
/// Both halves run inside a `tokio::spawn` so the assertion exercises the real
/// task boundary the fix crosses: `AGENT_TURN_ORIGIN` is a `tokio::task_local`
/// that does not survive `spawn`, so the origin the gate reads is whatever the
/// spawned future scopes for itself — nothing, or the fix's explicit label.
#[tokio::test]
async fn cron_add_on_a_turnless_spawn_resolves_to_a_real_origin_not_unknown_denied() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // Precondition — mirrors the bug before the fix: a bare `tokio::spawn`
    // with no ambient origin (capture() would yield None) reaches the gate as
    // `Unknown`, and the scheduling tool is refused as "no origin label".
    let g = gate.clone();
    let denied = tokio::spawn(async move {
        g.intercept("cron_add", "schedule a job", serde_json::json!({}))
            .await
    })
    .await
    .expect("spawned task panicked");
    match denied {
        GateOutcome::Deny { reason } => {
            assert!(reason.contains("origin label"), "reason was: {reason}")
        }
        other => panic!("unlabelled turn-less spawn must fail closed, got {other:?}"),
    }

    // With the fix: `run_local_agent` scopes an explicit `Cli` origin around
    // the spawned sub-agent work, so the same `cron_add` call now resolves to
    // a real origin and is allowed (device-tool automation past the
    // Master-chat gate) instead of being denied as unlabelled.
    let g = gate.clone();
    let allowed = tokio::spawn(turn_origin::with_origin(AgentTurnOrigin::Cli, async move {
        g.intercept("cron_add", "schedule a job", serde_json::json!({}))
            .await
    }))
    .await
    .expect("spawned task panicked");
    assert!(
        matches!(allowed, GateOutcome::Allow),
        "an explicit Cli origin scoped across the spawn must resolve cron_add \
         to a real origin and allow it, got {allowed:?}"
    );
}

#[tokio::test]
async fn intercept_with_external_channel_origin_persists_and_ttl_denies() {
    // Non-web channel inbound (Telegram / Discord / Slack / etc.):
    // persist an audit row but TTL-deny — there is no channel-routed
    // approval surface yet, and the input is remote-attacker text.
    let (gate, _dir) = test_gate(); // 2s TTL
    let gate = Arc::new(gate);
    let origin = AgentTurnOrigin::ExternalChannel {
        channel: "telegram".into(),
        sender: Some("tg-user-1".into()),
        reply_target: "tg-chat-1".into(),
        message_id: "msg-1".into(),
    };

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            origin,
            g.intercept("shell", "run ls", serde_json::json!({})),
        )
        .await
    });

    // The audit row appears while the future is parked.
    let mut tries = 0;
    loop {
        if !gate.list_pending().unwrap().is_empty() {
            break;
        }
        tries += 1;
        assert!(tries < 50, "audit row never appeared for external channel");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Without a routable channel approval surface, the parked future
    // TTL-denies (2s — matches the test_gate fixture).
    let outcome = handle.await.unwrap();
    match outcome {
        GateOutcome::Deny { reason } => assert!(reason.contains("timed out")),
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn intercept_audited_returns_request_id_only_when_allowed_and_persisted() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // Allow path: the audited variant must hand back the
    // request_id so the caller can record_execution later
    // (issue #2135).
    let g = gate.clone();
    let handle = tokio::spawn(async move {
        // Scope a chat context + matching WebChat origin *inside* the
        // spawned task — task-locals don't cross `tokio::spawn`, and
        // `intercept` only parks (creates a pending row) for a chat
        // turn whose origin labels it as web-routable.
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept_audited("composio", "send slack", serde_json::json!({})),
            ),
        )
        .await
    });
    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    gate.decide(&pending.request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    let (outcome, id) = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
    assert_eq!(
        id.as_deref(),
        Some(pending.request_id.as_str()),
        "allowed call must return its persisted request id"
    );

    // Now record execution against that id. Round-trip via a
    // fresh gate to prove the row landed in durable storage.
    gate.record_execution(&pending.request_id, ExecutionOutcome::Success, None);
}

#[tokio::test]
async fn intercept_audited_id_is_none_for_denied_some_for_approved() {
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    // Deny path → no id (nothing to record afterward).
    let g = gate.clone();
    let denied = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept_audited("composio", "send slack", serde_json::json!({})),
            ),
        )
        .await
    });
    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    gate.decide(&pending.request_id, ApprovalDecision::Deny)
        .unwrap();
    let (outcome, id) = denied.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Deny { .. }));
    assert!(id.is_none(), "denied calls have nothing to record");

    // Allowlist-shortcut path → also no id (no row was created).
    let g = gate.clone();
    let first = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept_audited("pushover", "first send", serde_json::json!({})),
            ),
        )
        .await
    });
    let pending = loop {
        if let Some(p) = gate
            .list_pending()
            .unwrap()
            .into_iter()
            .find(|p| p.tool_name == "pushover")
        {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    // `ApproveAlwaysForTool` resolves the parked prompt to Allow and, because
    // the prompt persisted a row, returns its id. (Persisting the tool onto
    // the `auto_approve` allowlist for *future* calls is the RPC handler's
    // job — see `approval::rpc::approval_decide` — and the gate's allowlist
    // short-circuit is covered by `auto_approve_tool_skips_prompt`.)
    gate.decide(&pending.request_id, ApprovalDecision::ApproveAlwaysForTool)
        .unwrap();
    let (first_outcome, first_id) = first.await.unwrap();
    assert!(matches!(first_outcome, GateOutcome::Allow));
    assert!(
        first_id.is_some(),
        "the prompting call still persists a row"
    );
}

// ── flow-approval-surface (source_context, flow_tool_trust, surfacing) ──

/// A `Workflow`-origin turn for the flow-correlation tests below.
fn flow_origin(flow_id: &str, require_approval: bool) -> AgentTurnOrigin {
    AgentTurnOrigin::TrustedAutomation {
        job_id: flow_id.to_string(),
        source: TrustedAutomationSource::Workflow { require_approval },
    }
}

#[tokio::test]
async fn flow_origin_park_populates_source_context_with_flow_and_run_id() {
    // A `require_approval: true` flow still parks (same shape as before
    // this change) but the persisted row must now carry the flow/run
    // correlation the `APPROVAL_FLOW_RUN_CONTEXT` task-local supplies —
    // the origin alone only carries `flow_id`, not `run_id`.
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            flow_origin("flow-1", true),
            APPROVAL_FLOW_RUN_CONTEXT.scope(
                FlowRunContext {
                    flow_id: "flow-1".to_string(),
                    run_id: "run-1".to_string(),
                },
                g.intercept_audited("composio", "post to slack", serde_json::json!({})),
            ),
        )
        .await
    });

    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    match &pending.source_context {
        Some(super::super::types::ApprovalSourceContext::Flow {
            flow_id,
            run_id,
            node_id,
        }) => {
            assert_eq!(flow_id, "flow-1");
            assert_eq!(run_id, "run-1");
            assert!(
                node_id.is_none(),
                "node_id is not yet threaded down to the gate"
            );
        }
        other => panic!("expected Flow source_context, got {other:?}"),
    }

    gate.decide(&pending.request_id, ApprovalDecision::Deny)
        .unwrap();
    let _ = handle.await.unwrap();
}

#[tokio::test]
async fn chat_origin_park_has_no_source_context() {
    // Regression guard: the plain chat-routed path (unaffected by this
    // change) must never gain a `source_context` — only Workflow-origin
    // parks populate it.
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            web_origin(),
            APPROVAL_CHAT_CONTEXT.scope(
                chat_ctx(),
                g.intercept_audited("composio", "send slack", serde_json::json!({})),
            ),
        )
        .await
    });

    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(
        pending.source_context.is_none(),
        "chat-origin parks must not carry a source_context"
    );

    gate.decide(&pending.request_id, ApprovalDecision::ApproveOnce)
        .unwrap();
    let (outcome, _id) = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
}

#[tokio::test]
async fn flow_tool_trust_auto_allows_before_parking() {
    // A prior `ApproveAlwaysForFlow` grant for (flow_id, tool_name) must
    // short-circuit to `Allow` even for a `require_approval: true` flow —
    // that is the whole point of "approve always for this workflow": no
    // pending row is created and the call never parks.
    let (gate, _dir) = test_gate();
    store::insert_flow_trust(&gate.config, "flow-trusted", "composio").unwrap();

    let outcome = turn_origin::with_origin(
        flow_origin("flow-trusted", true),
        APPROVAL_FLOW_RUN_CONTEXT.scope(
            FlowRunContext {
                flow_id: "flow-trusted".to_string(),
                run_id: "run-1".to_string(),
            },
            gate.intercept("composio", "post to slack", serde_json::json!({})),
        ),
    )
    .await;

    assert!(matches!(outcome, GateOutcome::Allow));
    assert!(
        gate.list_pending().unwrap().is_empty(),
        "a trusted (flow, tool) pair must not persist a pending row"
    );

    // A different tool on the same trusted flow is unaffected — it still
    // parks (TTL-denies on the 2s test gate).
    let untrusted_outcome = turn_origin::with_origin(
        flow_origin("flow-trusted", true),
        APPROVAL_FLOW_RUN_CONTEXT.scope(
            FlowRunContext {
                flow_id: "flow-trusted".to_string(),
                run_id: "run-1".to_string(),
            },
            gate.intercept("pushover", "send push", serde_json::json!({})),
        ),
    )
    .await;
    assert!(
        matches!(untrusted_outcome, GateOutcome::Deny { .. }),
        "trust must be scoped to the exact tool granted, not the whole flow"
    );
}

#[tokio::test]
async fn decide_approve_always_for_flow_then_insert_flow_trust_composes_to_auto_allow() {
    // Exercises the two building blocks the `approval_decide` RPC handler
    // composes for `ApproveAlwaysForFlow` (see `approval::rpc`): the gate
    // resolves the parked call and returns the decided row (carrying
    // `source_context`), and the RPC layer then calls
    // `ApprovalGate::insert_flow_trust` using that row's flow id. This
    // test exercises both steps directly against a local (non-global)
    // gate — the RPC handler itself reads the process-wide
    // `ApprovalGate::try_global()` singleton, which tests must not touch
    // (it would leak state into every other test in this binary).
    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            flow_origin("flow-2", true),
            APPROVAL_FLOW_RUN_CONTEXT.scope(
                FlowRunContext {
                    flow_id: "flow-2".to_string(),
                    run_id: "run-2".to_string(),
                },
                g.intercept_audited("composio", "post to slack", serde_json::json!({})),
            ),
        )
        .await
    });

    let pending = loop {
        if let Some(p) = gate.list_pending().unwrap().into_iter().next() {
            break p;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    let decided = gate
        .decide(&pending.request_id, ApprovalDecision::ApproveAlwaysForFlow)
        .unwrap()
        .expect("decided row");

    assert!(!gate.is_flow_tool_trusted("flow-2", "composio").unwrap());

    match &decided.source_context {
        Some(super::super::types::ApprovalSourceContext::Flow { flow_id, .. }) => {
            gate.insert_flow_trust(flow_id, &decided.tool_name).unwrap();
        }
        other => panic!("expected Flow source_context, got {other:?}"),
    }

    assert!(gate.is_flow_tool_trusted("flow-2", "composio").unwrap());

    let (outcome, _id) = handle.await.unwrap();
    assert!(matches!(outcome, GateOutcome::Allow));
}

#[tokio::test]
async fn flow_origin_park_publishes_flow_approval_request_and_notification() {
    // The silent-deadlock bug this whole PR fixes: a flow-origin park has
    // no chat thread/client, so the generic `ApprovalRequested` event's
    // web-channel bridge silently drops it. This test asserts the two new
    // surfaces fire instead — the `flow_approval_request` DomainEvent
    // (bridged to a broadcast Socket.IO event by `core::socketio`) and
    // the `flow-gate-approval` CoreNotification with its three actions.
    crate::core::bus::init().await.expect("bus init");
    let mut event_rx = crate::core::bus::BUS
        .get()
        .expect("event bus initialized above")
        .receiver();
    let mut notif_rx =
        crate::openhuman::desktop::notifications::bus::subscribe_core_notifications();

    let (gate, _dir) = test_gate();
    let gate = Arc::new(gate);

    let g = gate.clone();
    let handle = tokio::spawn(async move {
        turn_origin::with_origin(
            flow_origin("flow-9", true),
            APPROVAL_FLOW_RUN_CONTEXT.scope(
                FlowRunContext {
                    flow_id: "flow-9".to_string(),
                    run_id: "run-9".to_string(),
                },
                g.intercept_audited("composio", "post to slack", serde_json::json!({})),
            ),
        )
        .await
    });

    let (request_id, run_id, tool_name) = tokio::time::timeout(
        Duration::from_secs(5),
        find_flow_approval_requested(&mut event_rx, "flow-9"),
    )
    .await
    .expect("timed out waiting for FlowApprovalRequested");
    assert_eq!(run_id, "run-9");
    assert_eq!(tool_name, "composio");

    let notif = tokio::time::timeout(
        Duration::from_secs(5),
        find_flow_gate_notification(&mut notif_rx, &request_id),
    )
    .await
    .expect("timed out waiting for the flow-gate-approval notification");
    assert_eq!(notif.id, format!("flow-gate-approval:{request_id}"));
    let actions = notif.actions.expect("notification must declare actions");
    let action_ids: Vec<_> = actions.iter().map(|a| a.action_id.as_str()).collect();
    assert_eq!(
        action_ids,
        vec!["approve_once", "approve_always_for_flow", "deny"]
    );

    gate.decide(&request_id, ApprovalDecision::Deny).unwrap();
    let _ = handle.await.unwrap();
}

/// Drain `rx` until a `FlowApprovalRequested` for `expected_flow_id`
/// arrives. The event bus is process-wide and other tests in this file
/// (and elsewhere) publish on it concurrently — including other
/// `FlowApprovalRequested` events for *different* flow ids — so this must
/// filter by flow id, not just by variant, and tolerate both unrelated
/// events and broadcast lag rather than returning the first match.
async fn find_flow_approval_requested(
    rx: &mut tinybus::events::EventReceiver<crate::core::events::DomainEvent>,
    expected_flow_id: &str,
) -> (String, String, String) {
    loop {
        match rx.recv().await {
            Some(crate::core::events::DomainEvent::FlowApprovalRequested {
                request_id,
                flow_id,
                run_id,
                tool_name,
                ..
            }) if flow_id == expected_flow_id => return (request_id, run_id, tool_name),
            Some(_) => continue,
            None => panic!("the bus closed before the expected event arrived"),
        }
    }
}

/// Drain `rx` until the `flow-gate-approval` notification for
/// `request_id` arrives — the notification bus is process-wide, so
/// unrelated notifications from other concurrently-running tests are
/// tolerated and skipped.
async fn find_flow_gate_notification(
    rx: &mut tokio::sync::broadcast::Receiver<
        crate::openhuman::desktop::notifications::types::CoreNotificationEvent,
    >,
    request_id: &str,
) -> crate::openhuman::desktop::notifications::types::CoreNotificationEvent {
    let expected_id = format!("flow-gate-approval:{request_id}");
    loop {
        match rx.recv().await {
            Ok(event) if event.id == expected_id => return event,
            Ok(_) => continue,
            Err(err) => panic!("the notification bus closed before the approval: {err}"),
        }
    }
}
