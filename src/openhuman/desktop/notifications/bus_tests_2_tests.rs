use super::*;

#[test]
fn cron_completed_produces_agents_notification() {
    let ev = DomainEvent::CronJobCompleted {
        job_id: "job-1".into(),
        success: true,
        output: "done".into(),
    };
    let n = event_to_notification(&ev).expect("should produce notification");
    assert_eq!(n.category, CoreNotificationCategory::Agents);
    assert_eq!(n.title, "Cron job completed");
    assert!(n.body.contains("job-1"));
}

#[test]
fn provider_api_key_rejected_produces_system_notification() {
    let ev = DomainEvent::ProviderApiKeyRejected {
        provider: "openrouter".into(),
        message: "openrouter rejected the API key (HTTP 401). Update your openrouter \
                  API key in Connections → API keys → LLM to restore it."
            .into(),
    };
    let n = event_to_notification(&ev).expect("should produce notification");
    assert_eq!(n.category, CoreNotificationCategory::System);
    assert_eq!(n.title, "API key rejected");
    assert!(n.body.contains("openrouter"));
    assert!(n.body.contains("Connections"));
    assert_eq!(n.deep_link.as_deref(), Some("/connections?tab=llm"));
    assert!(n.id.starts_with("provider-key-rejected:openrouter:"));
}

#[test]
fn cron_failed_uses_failure_title() {
    let ev = DomainEvent::CronJobCompleted {
        job_id: "job-1".into(),
        success: false,
        output: "error".into(),
    };
    let n = event_to_notification(&ev).unwrap();
    assert_eq!(n.title, "Cron job failed");
}

#[test]
fn successful_webhook_is_silent() {
    let ev = DomainEvent::WebhookProcessed {
        tunnel_id: "t".into(),
        skill_id: "s".into(),
        method: "POST".into(),
        path: "/p".into(),
        correlation_id: "c".into(),
        status_code: 200,
        elapsed_ms: 5,
        error: None,
    };
    assert!(event_to_notification(&ev).is_none());
}

#[test]
fn failed_webhook_produces_system_notification() {
    let ev = DomainEvent::WebhookProcessed {
        tunnel_id: "t".into(),
        skill_id: "skill-x".into(),
        method: "POST".into(),
        path: "/p".into(),
        correlation_id: "c".into(),
        status_code: 500,
        elapsed_ms: 12,
        error: Some("boom".into()),
    };
    let n = event_to_notification(&ev).unwrap();
    assert_eq!(n.category, CoreNotificationCategory::System);
    assert!(n.body.contains("skill-x"));
    assert!(n.body.contains("boom"));
}

#[test]
fn subagent_completed_produces_agents_notification() {
    let ev = DomainEvent::SubagentCompleted {
        parent_session: "p".into(),
        task_id: "t".into(),
        agent_id: "researcher".into(),
        elapsed_ms: 100,
        output_chars: 500,
        iterations: 3,
    };
    let n = event_to_notification(&ev).unwrap();
    assert_eq!(n.category, CoreNotificationCategory::Agents);
    assert!(n.body.contains("researcher"));
    assert!(n.body.contains("500"));
}

#[test]
fn subagent_failed_produces_agents_notification() {
    let ev = DomainEvent::SubagentFailed {
        parent_session: "p".into(),
        task_id: "t".into(),
        agent_id: "researcher".into(),
        error: "context window exceeded".into(),
    };
    let n = event_to_notification(&ev).unwrap();
    assert_eq!(n.category, CoreNotificationCategory::Agents);
    assert_eq!(n.title, "Sub-agent failed");
    assert!(n.body.contains("researcher"));
    assert!(n.body.contains("context window exceeded"));
}

#[test]
fn unrelated_events_return_none() {
    let ev = DomainEvent::AgentTurnCompleted {
        session_id: "s".into(),
        text_chars: 1,
        iterations: 1,
    };
    assert!(event_to_notification(&ev).is_none());
}

#[test]
fn notification_triaged_escalate_produces_agents_notification() {
    let ev = DomainEvent::NotificationTriaged {
        id: "n1".into(),
        provider: "slack".into(),
        action: "escalate".into(),
        importance_score: 0.9,
        latency_ms: 100,
        routed: true,
    };
    let n = event_to_notification(&ev).expect("should produce notification");
    assert_eq!(n.category, CoreNotificationCategory::Agents);
    assert!(n.body.contains("escalate"));
    assert!(n.deep_link.as_deref() == Some("/notifications"));
}

#[test]
fn notification_triaged_react_uses_follow_up_copy() {
    let ev = DomainEvent::NotificationTriaged {
        id: "n2".into(),
        provider: "discord".into(),
        action: "react".into(),
        importance_score: 0.7,
        latency_ms: 120,
        routed: true,
    };
    let n = event_to_notification(&ev).expect("should produce notification");
    assert_eq!(n.category, CoreNotificationCategory::Agents);
    assert!(n.body.contains("Routed for follow-up"));
}

#[test]
fn notification_triaged_drop_is_silent() {
    let ev = DomainEvent::NotificationTriaged {
        id: "n1".into(),
        provider: "gmail".into(),
        action: "drop".into(),
        importance_score: 0.1,
        latency_ms: 50,
        routed: false,
    };
    assert!(event_to_notification(&ev).is_none());
}

#[test]
fn notification_triaged_unrouted_escalate_is_silent() {
    let ev = DomainEvent::NotificationTriaged {
        id: "n1".into(),
        provider: "slack".into(),
        action: "escalate".into(),
        importance_score: 0.9,
        latency_ms: 100,
        routed: false,
    };
    assert!(event_to_notification(&ev).is_none());
}

// ── MCP reconnect supervisor (#5931) ────────────────────────────────────────

/// The workspace an MCP supervisor event is attributed to. The pure
/// translator ignores it — the bridge's `is_for_this_workspace` is what reads
/// it — so these cases all use the same one.
fn mcp_workspace() -> std::path::PathBuf {
    std::path::PathBuf::from("/tmp/openhuman-ws")
}

#[test]
fn mcp_first_failed_reconnect_tells_the_user_tools_are_unavailable() {
    let ev = DomainEvent::McpServerReconnectFailed {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        error: "mcp transport failure for `https://api.inference.sh`: connection reset".into(),
        failures: 1,
        retry_in_secs: 5,
        workspace_dir: mcp_workspace(),
    };
    let n = event_to_notification(&ev).expect("the first failure of an episode notifies");
    assert_eq!(n.category, CoreNotificationCategory::System);
    assert_eq!(n.title, "MCP server unavailable");
    assert!(n.body.contains("ac.inference.sh/mcp"), "{}", n.body);
    assert!(n.body.contains("retrying in 5s"), "{}", n.body);
    assert!(n.body.contains("connection reset"), "{}", n.body);
    assert_eq!(n.deep_link.as_deref(), Some("/connections?tab=mcp"));
    assert!(n.id.starts_with("mcp-unavailable:srv-1:"));
}

#[test]
fn mcp_later_failed_reconnects_stay_quiet() {
    // The backoff retries every few minutes for as long as the server is
    // down; the user heard about it once, on the first failure.
    let ev = DomainEvent::McpServerReconnectFailed {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        error: "connection refused".into(),
        failures: 2,
        retry_in_secs: 10,
        workspace_dir: mcp_workspace(),
    };
    assert!(event_to_notification(&ev).is_none());
}

#[test]
fn mcp_recovery_after_failures_is_announced() {
    let ev = DomainEvent::McpServerReconnected {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        tool_count: 25,
        after_failures: 2,
        workspace_dir: mcp_workspace(),
    };
    let n = event_to_notification(&ev).expect("a server that had stayed down coming back notifies");
    assert_eq!(n.category, CoreNotificationCategory::System);
    assert_eq!(n.title, "MCP server reconnected");
    assert!(n.body.contains("25 tools"), "{}", n.body);
    assert!(n.body.contains("2 failed attempt"), "{}", n.body);
    assert_eq!(n.deep_link.as_deref(), Some("/connections?tab=mcp"));
    assert!(n.id.starts_with("mcp-restored:srv-1:"));
}

#[test]
fn mcp_rebuild_within_the_same_tick_is_not_a_notification() {
    // The common field case: one request failed, the session was rebuilt a
    // second later, nobody noticed. Event Log only.
    let ev = DomainEvent::McpServerReconnected {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        tool_count: 25,
        after_failures: 0,
        workspace_dir: mcp_workspace(),
    };
    assert!(event_to_notification(&ev).is_none());
}

#[test]
fn mcp_parked_server_tells_the_user_how_to_recover() {
    let ev = DomainEvent::McpServerParked {
        server_id: "srv-1".into(),
        qualified_name: "@modelcontextprotocol/server-github".into(),
        error: "the `uvx` launcher is not installed".into(),
        workspace_dir: mcp_workspace(),
    };
    let n = event_to_notification(&ev).expect("a parked server notifies");
    assert_eq!(n.category, CoreNotificationCategory::System);
    assert_eq!(n.title, "MCP server can't start");
    assert!(
        n.body.contains("@modelcontextprotocol/server-github"),
        "{}",
        n.body
    );
    assert!(n.body.contains("uvx"), "{}", n.body);
    assert!(n.body.contains("disable and re-enable"), "{}", n.body);
    assert_eq!(n.deep_link.as_deref(), Some("/connections?tab=mcp"));
    assert!(n.id.starts_with("mcp-parked:srv-1:"));
}

#[test]
fn mcp_probe_timeouts_and_transport_drops_are_event_log_only() {
    let timed_out = DomainEvent::McpServerProbeTimedOut {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        probe_timeout_secs: 8,
        consecutive_timeouts: 1,
        teardown_after: 3,
        workspace_dir: mcp_workspace(),
    };
    let dropped = DomainEvent::McpServerTransportDropped {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        outcome: "broken".into(),
        detail: Some("connection reset".into()),
        elapsed_ms: Some(1961),
        consecutive_timeouts: 0,
        workspace_dir: mcp_workspace(),
    };
    assert!(event_to_notification(&timed_out).is_none());
    assert!(event_to_notification(&dropped).is_none());
}

// ── Workspace scoping (#5931) ───────────────────────────────────────────────
//
// One process supervises every workspace it has opened, and this bridge is
// registered once with the workspace that booted. A supervisor event from a
// workspace the user has switched away from must not be announced from — or
// persisted into — the current one.

fn bridge_for(workspace: &std::path::Path) -> NotificationBridgeSubscriber {
    let mut config = crate::openhuman::config::Config::default();
    config.workspace_dir = workspace.to_path_buf();
    NotificationBridgeSubscriber::new(config)
}

fn parked_in(workspace: &std::path::Path) -> DomainEvent {
    DomainEvent::McpServerParked {
        server_id: "srv-1".into(),
        qualified_name: "ac.inference.sh/mcp".into(),
        error: "the `uvx` launcher is not installed".into(),
        workspace_dir: workspace.to_path_buf(),
    }
}

#[test]
fn a_supervisor_event_from_this_workspace_is_this_bridge_s() {
    let workspace = std::path::Path::new("/tmp/openhuman-ws-a");
    assert!(bridge_for(workspace).is_for_this_workspace(&parked_in(workspace)));
}

#[test]
fn a_supervisor_event_from_a_switched_away_workspace_is_dropped() {
    let bridge = bridge_for(std::path::Path::new("/tmp/openhuman-ws-a"));
    let other = std::path::Path::new("/tmp/openhuman-ws-b");
    assert!(!bridge.is_for_this_workspace(&parked_in(other)));
    // The pure translator would still have produced one — the drop is the
    // bridge's decision, not the translator's.
    assert!(event_to_notification(&parked_in(other)).is_some());
}

#[test]
fn every_supervisor_variant_is_scoped_not_only_the_notifying_ones() {
    let bridge = bridge_for(std::path::Path::new("/tmp/openhuman-ws-a"));
    let other = mcp_workspace();
    let foreign = [
        DomainEvent::McpServerProbeTimedOut {
            server_id: "srv-1".into(),
            qualified_name: "ac.inference.sh/mcp".into(),
            probe_timeout_secs: 8,
            consecutive_timeouts: 1,
            teardown_after: 3,
            workspace_dir: other.clone(),
        },
        DomainEvent::McpServerTransportDropped {
            server_id: "srv-1".into(),
            qualified_name: "ac.inference.sh/mcp".into(),
            outcome: "broken".into(),
            detail: None,
            elapsed_ms: None,
            consecutive_timeouts: 0,
            workspace_dir: other.clone(),
        },
        DomainEvent::McpServerReconnected {
            server_id: "srv-1".into(),
            qualified_name: "ac.inference.sh/mcp".into(),
            tool_count: 25,
            after_failures: 2,
            workspace_dir: other.clone(),
        },
        DomainEvent::McpServerReconnectFailed {
            server_id: "srv-1".into(),
            qualified_name: "ac.inference.sh/mcp".into(),
            error: "connection refused".into(),
            failures: 1,
            retry_in_secs: 5,
            workspace_dir: other.clone(),
        },
        parked_in(&other),
    ];
    for event in &foreign {
        assert!(
            !bridge.is_for_this_workspace(event),
            "{} should be scoped out",
            event.variant_name()
        );
    }
}

#[test]
fn an_event_that_names_no_workspace_is_never_scoped_out() {
    // Every variant this bridge handled before #5931 — a cron job, a webhook,
    // a rejected API key — is process-wide and must keep reaching it.
    let bridge = bridge_for(std::path::Path::new("/tmp/openhuman-ws-a"));
    assert!(
        bridge.is_for_this_workspace(&DomainEvent::CronJobCompleted {
            job_id: "job-1".into(),
            success: true,
            output: "done".into(),
        })
    );
}

#[test]
fn a_bridge_with_no_config_scopes_nothing_out() {
    // `config: None` is the unit-test-only shape; with no workspace binding
    // there is nothing to compare against, so nothing is dropped.
    let bridge = NotificationBridgeSubscriber::default();
    assert!(bridge.is_for_this_workspace(&parked_in(&mcp_workspace())));
}

/// End to end through `handle`: a foreign workspace's outage is neither
/// persisted into this workspace's store nor broadcast, while this
/// workspace's own is.
///
/// Asserted on the store rather than on `NOTIFICATION_BUS`: that broadcast
/// channel is a process-wide static every test in this binary publishes to,
/// so a receiver here would also see other tests' notifications.
#[tokio::test]
async fn a_switched_away_workspace_s_outage_is_neither_stored_nor_announced() {
    use crate::openhuman::desktop::notifications::store;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let mut config = crate::openhuman::config::Config::default();
    config.workspace_dir = dir.path().to_path_buf();
    let bridge = NotificationBridgeSubscriber::new(config.clone());

    bridge
        .handle(&parked_in(std::path::Path::new("/tmp/openhuman-ws-b")))
        .await;
    assert!(
        store::list_core_notifications(&config, true, 50)
            .unwrap()
            .is_empty(),
        "a foreign workspace's parked server must not land in this store"
    );

    bridge.handle(&parked_in(dir.path())).await;
    let items = store::list_core_notifications(&config, true, 50).unwrap();
    assert_eq!(items.len(), 1, "this workspace's own outage still notifies");
    assert!(items[0].id.starts_with("mcp-parked:srv-1:"));
}
