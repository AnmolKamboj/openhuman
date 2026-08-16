//! Langfuse export for `flows::` graph runs.
//!
//! After a `flows_run` / `flows_resume` settles, the `flows::` domain hands
//! this module the run's durable [`GraphObservation`] slice (captured by the
//! per-run in-memory journal that `tinyflows`' journaled entry points fill)
//! and it exports the run as one Langfuse trace via the backend's Langfuse
//! proxy route, `/telemetry/langfuse/ingestion`.
//!
//! The batch is built by `tinyagents`' [`GraphLangfuseExporter`], which turns
//! each superstep and node into a timed span and stamps the Langfuse **Agent
//! Graph view** keys (`langgraph_node` / `langgraph_step`) on node spans, so
//! the Langfuse UI can render the flow run as a graph. A host span-metadata
//! injector additionally stamps `flow_id` on every span for filtering.
//!
//! Transport mirrors the agent-turn tracing path
//! (`agent::progress_tracing::langfuse::push_spans`): the endpoint is derived
//! from the **current backend hostname** (`effective_backend_api_url`), auth
//! is the live OpenHuman session bearer (the backend injects the real
//! Langfuse keys server-side), the send is capped at 10s, `207 Multi-Status`
//! is tolerated, and every failure is logged and swallowed — exporting is
//! best-effort and never fails the run. Gated on
//! `observability.share_usage_data`.

use std::time::Duration;

use serde_json::{json, Map};
use tinyagents::{
    GraphLangfuseExporter, GraphObservation, LangfuseAuth, LangfuseClient, LangfuseTraceConfig,
};
use tinyflows::engine::GraphObservation as FlowObservation;

use crate::api::config::effective_backend_api_url;
use crate::openhuman::config::Config;
use crate::openhuman::flows::FlowRunTrigger;
use crate::openhuman::security::credentials::session_support::require_live_session_token;

const LOG_TARGET: &str = "flows::langfuse";
/// Backend proxy route for Langfuse ingestion (relative to the backend
/// origin). The backend authenticates the session JWT, injects the Langfuse
/// project keys, and forwards to Langfuse's real `/api/public/ingestion`.
const INGESTION_PATH: &str = "/telemetry/langfuse/ingestion";
/// Cap the push so a slow/hung Langfuse never stalls run teardown (same
/// posture as the agent-turn exporter).
const PUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// Resolve the Langfuse ingestion URL from the current backend host — the
/// exact base-server resolution every other backend call uses, so the host
/// always matches wherever the app's domain calls go (staging, prod, or a
/// custom `api_url` override).
fn ingestion_url(config: &Config) -> String {
    let base = effective_backend_api_url(&config.api_url);
    crate::api::config::api_url(&base, INGESTION_PATH)
}

/// The OpenHuman core crate version (e.g. `0.58.0`), stamped onto every flow
/// trace as the Langfuse `release` field plus an `app_version` metadata key so
/// traces can be correlated with the app build that produced them.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builds the [`LangfuseTraceConfig`] for one flow run: the trace id **and**
/// session id are the run's `thread_id` (`flow:{flow_id}:{uuid}`), the trace
/// is named `flow.run:{flow_name}`, run-type tags (`run:flow` +
/// `trigger:<kind>`) mark how the run started, and flow coordinates plus the
/// app version ride on the trace metadata. No content — ids, name, status,
/// trigger, and version only.
fn build_flow_trace_config(
    flow_name: &str,
    flow_id: &str,
    thread_id: &str,
    status: &str,
    trigger: FlowRunTrigger,
) -> LangfuseTraceConfig {
    LangfuseTraceConfig {
        trace_id: Some(thread_id.to_string()),
        name: Some(format!("flow.run:{flow_name}")),
        session_id: Some(thread_id.to_string()),
        release: Some(APP_VERSION.to_string()),
        tags: vec![
            "run:flow".to_string(),
            format!("trigger:{}", trigger.as_str()),
        ],
        metadata: json!({
            "flow_id": flow_id,
            "status": status,
            "source": "flows",
            "run_type": "flow",
            "trigger": trigger.as_str(),
            "app_version": APP_VERSION,
        }),
        ..Default::default()
    }
}

/// Builds the graph exporter for a flow run: the batch builder plus a host
/// span-metadata injector that stamps `flow_id` on every span (node spans
/// already carry `langgraph_node`/`langgraph_step` from `tinyagents`).
fn build_flow_exporter(client: LangfuseClient, flow_id: &str) -> GraphLangfuseExporter {
    let flow_id = flow_id.to_string();
    GraphLangfuseExporter::new(client).with_span_metadata_fn(move |_obs| {
        let mut extra = Map::new();
        extra.insert("flow_id".to_string(), json!(flow_id));
        Some(extra)
    })
}

/// Re-types the engine's observations for the exporter.
///
/// The engine now emits `tinyflows::engine::GraphObservation` — tinyflows
/// vendored its state-graph runtime in PR #43 — while the Langfuse batch is
/// still built by `tinyagents`' exporter, which names its own. The two types
/// are the same shape (`tinyagents` is where tinyflows' copy came from) down
/// to the `GraphEvent` payload, so this re-types through their shared serde
/// representation rather than duplicating a 20-field mapping that would then
/// have to be kept in step by hand.
///
/// An observation that fails to convert is **dropped, not fatal**: exporting
/// is best-effort throughout this module, and losing one span from a trace is
/// a better outcome than losing the trace. A drop is logged at `warn` because
/// the only way it can happen is the two types drifting apart, which is worth
/// hearing about — `re_typing_preserves_every_field` is the guard that should
/// catch it first.
fn to_exporter_observations(observations: &[FlowObservation]) -> Vec<GraphObservation> {
    observations
        .iter()
        .filter_map(|obs| {
            match serde_json::to_value(obs).and_then(serde_json::from_value::<GraphObservation>) {
                Ok(converted) => Some(converted),
                Err(err) => {
                    tracing::warn!(
                        target: LOG_TARGET,
                        error = %err,
                        "[flows] dropped an observation the exporter could not read — \
                         tinyflows and tinyagents GraphObservation have drifted"
                    );
                    None
                }
            }
        })
        .collect()
}

/// Exports one settled flow run to Langfuse as a single trace. Best-effort:
/// every failure path logs a `[flows]`-prefixed warning and returns — a
/// Langfuse outage can never fail or delay-fail the run. No-op when
/// `observability.share_usage_data` is off or there is nothing to send.
pub async fn export_flow_run_trace(
    config: &Config,
    flow_name: &str,
    flow_id: &str,
    thread_id: &str,
    status: &str,
    trigger: FlowRunTrigger,
    journal_observations: &[FlowObservation],
) {
    if !config.observability.share_usage_data {
        tracing::debug!(
            target: LOG_TARGET,
            flow_id = %flow_id,
            "[flows] langfuse export skipped: observability.share_usage_data is off"
        );
        return;
    }
    if journal_observations.is_empty() {
        tracing::debug!(
            target: LOG_TARGET,
            flow_id = %flow_id,
            thread_id = %thread_id,
            "[flows] langfuse export skipped: run journal is empty"
        );
        return;
    }

    let url = ingestion_url(config);
    if !url.starts_with("http") {
        tracing::warn!(
            target: LOG_TARGET,
            flow_id = %flow_id,
            "[flows] langfuse export skipped: could not resolve ingestion URL from backend host (got {url:?})"
        );
        return;
    }
    let token = match require_live_session_token(config) {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!(
                target: LOG_TARGET,
                flow_id = %flow_id,
                error = %err,
                "[flows] langfuse export skipped: no live session token"
            );
            return;
        }
    };
    let client = match LangfuseClient::new(url.clone(), LangfuseAuth::Bearer { token }) {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(
                target: LOG_TARGET,
                flow_id = %flow_id,
                error = %err,
                "[flows] langfuse export skipped: could not build client for {url}"
            );
            return;
        }
    };

    let observations = to_exporter_observations(journal_observations);
    if observations.is_empty() {
        tracing::warn!(
            target: LOG_TARGET,
            flow_id = %flow_id,
            thread_id = %thread_id,
            "[flows] langfuse export skipped: no observation survived re-typing"
        );
        return;
    }

    let exporter = build_flow_exporter(client, flow_id);
    let trace = build_flow_trace_config(flow_name, flow_id, thread_id, status, trigger);
    let observation_count = observations.len();
    tracing::debug!(
        target: LOG_TARGET,
        flow_id = %flow_id,
        thread_id = %thread_id,
        status = %status,
        trigger = %trigger.as_str(),
        observation_count,
        endpoint = %exporter.endpoint(),
        "[flows] pushing flow run trace to Langfuse"
    );

    // `send_observations` already tolerates 207 Multi-Status; the outer
    // timeout caps a hung connection the same way the agent exporter does.
    match tokio::time::timeout(
        PUSH_TIMEOUT,
        exporter.send_observations(trace, &observations),
    )
    .await
    {
        Ok(Ok(_)) => {
            tracing::debug!(
                target: LOG_TARGET,
                flow_id = %flow_id,
                thread_id = %thread_id,
                observation_count,
                "[flows] pushed flow run trace to Langfuse"
            );
        }
        Ok(Err(err)) => {
            tracing::warn!(
                target: LOG_TARGET,
                flow_id = %flow_id,
                thread_id = %thread_id,
                error = %err,
                "[flows] langfuse export failed (run unaffected)"
            );
        }
        Err(_elapsed) => {
            tracing::warn!(
                target: LOG_TARGET,
                flow_id = %flow_id,
                thread_id = %thread_id,
                timeout_secs = PUSH_TIMEOUT.as_secs(),
                "[flows] langfuse export timed out (run unaffected)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tinyflows::graph::ids;
    use tinyflows::graph::GraphEvent;

    /// Builds a minimal observation stream for one node under run/thread ids
    /// shaped like a real `flows_run` (`thread_id = flow:{id}:{uuid}`).
    ///
    /// Built as the ENGINE's observation type, which is what the `flows::`
    /// domain actually hands this module — so every test below goes through
    /// the same re-typing hop production does.
    fn sample_observations(thread_id: &str) -> Vec<FlowObservation> {
        let node = ids::NodeId::new("fetch");
        let mk = |offset: u64, step: usize, ts_ms: u64, event: GraphEvent| FlowObservation {
            event_id: ids::EventId::new(format!("evt-{offset}")),
            run_id: ids::RunId::new("run-9"),
            root_run_id: ids::RunId::new("run-9"),
            parent_run_id: None,
            thread_id: Some(ids::ThreadId::new(thread_id)),
            graph_id: ids::GraphId::new("workflow"),
            checkpoint_id: None,
            namespace: Vec::new(),
            step,
            offset,
            ts_ms,
            event,
        };
        vec![
            mk(
                0,
                0,
                1_000,
                GraphEvent::RunStarted {
                    run_id: ids::RunId::new("run-9"),
                },
            ),
            mk(
                1,
                1,
                1_010,
                GraphEvent::StepStarted {
                    step: 1,
                    active: vec![node.clone()],
                },
            ),
            mk(
                2,
                1,
                1_020,
                GraphEvent::NodeStarted {
                    node: node.clone(),
                    step: 1,
                },
            ),
            mk(
                3,
                1,
                1_050,
                GraphEvent::NodeCompleted {
                    node: node.clone(),
                    step: 1,
                },
            ),
            mk(4, 1, 1_060, GraphEvent::StepCompleted { step: 1 }),
        ]
    }

    /// Finds the first span-create whose body id matches.
    fn find_span<'a>(batch: &'a [Value], id: &str) -> Option<&'a Value> {
        batch
            .iter()
            .find(|e| e["type"] == "span-create" && e["body"]["id"] == id)
    }

    #[test]
    fn ingestion_url_targets_backend_proxy_route() {
        let mut config = Config::default();
        config.api_url = Some("https://staging-api.tinyhumans.ai/api/v1".to_string());
        assert_eq!(
            ingestion_url(&config),
            "https://staging-api.tinyhumans.ai/telemetry/langfuse/ingestion"
        );
    }

    #[test]
    fn flow_trace_config_uses_thread_id_and_flow_coordinates() {
        let trace = build_flow_trace_config(
            "Daily digest",
            "flow-1",
            "flow:flow-1:uuid-1",
            "completed",
            FlowRunTrigger::Schedule,
        );
        assert_eq!(trace.trace_id.as_deref(), Some("flow:flow-1:uuid-1"));
        assert_eq!(trace.session_id.as_deref(), Some("flow:flow-1:uuid-1"));
        assert_eq!(trace.name.as_deref(), Some("flow.run:Daily digest"));
        assert_eq!(trace.tags, vec!["run:flow", "trigger:schedule"]);
        assert_eq!(trace.metadata["flow_id"], "flow-1");
        assert_eq!(trace.metadata["status"], "completed");
        assert_eq!(trace.metadata["source"], "flows");
        assert_eq!(trace.metadata["run_type"], "flow");
        assert_eq!(trace.metadata["trigger"], "schedule");
        assert_eq!(trace.release.as_deref(), Some(APP_VERSION));
        assert_eq!(trace.metadata["app_version"], APP_VERSION);
        assert!(!APP_VERSION.is_empty(), "crate version must be baked in");
    }

    #[test]
    fn batch_carries_flow_trace_and_langgraph_keys_on_node_spans() {
        let thread_id = "flow:flow-1:uuid-1";
        let observations = sample_observations(thread_id);
        let client = LangfuseClient::new(
            "https://backend.test/telemetry/langfuse/ingestion",
            LangfuseAuth::Bearer {
                token: "tok".to_string(),
            },
        )
        .expect("client");
        let exporter = build_flow_exporter(client, "flow-1");
        let trace = build_flow_trace_config(
            "Daily digest",
            "flow-1",
            thread_id,
            "completed",
            FlowRunTrigger::Rpc,
        );
        let payload = exporter
            .build_ingestion_batch(trace, &to_exporter_observations(&observations))
            .expect("batch");
        let batch = payload["batch"].as_array().expect("batch array");

        // Trace: id + sessionId are the run thread id; name and flow
        // coordinates as configured.
        let trace_event = &batch[0];
        assert_eq!(trace_event["type"], "trace-create");
        assert_eq!(trace_event["body"]["id"], thread_id);
        assert_eq!(trace_event["body"]["sessionId"], thread_id);
        assert_eq!(trace_event["body"]["name"], "flow.run:Daily digest");
        assert_eq!(trace_event["body"]["metadata"]["flow_id"], "flow-1");
        assert_eq!(trace_event["body"]["metadata"]["status"], "completed");
        assert_eq!(trace_event["body"]["metadata"]["source"], "flows");
        assert_eq!(trace_event["body"]["metadata"]["run_type"], "flow");
        assert_eq!(trace_event["body"]["metadata"]["trigger"], "rpc");
        assert_eq!(
            trace_event["body"]["tags"],
            json!(["run:flow", "trigger:rpc"]),
            "run-type tags must ride on the trace-create"
        );
        assert_eq!(
            trace_event["body"]["release"], APP_VERSION,
            "app version must ride on the trace-create as release"
        );
        assert_eq!(trace_event["body"]["metadata"]["app_version"], APP_VERSION);

        // Node span: Agent-Graph-view keys + the injected flow_id, under the
        // overridden trace id.
        let node = find_span(batch, &format!("{thread_id}:node:fetch:1")).expect("node span");
        assert_eq!(node["body"]["traceId"], thread_id);
        assert_eq!(node["body"]["metadata"]["langgraph_node"], "fetch");
        assert_eq!(node["body"]["metadata"]["langgraph_step"], 1);
        assert_eq!(node["body"]["metadata"]["flow_id"], "flow-1");

        // Step span: superstep index + injected flow_id.
        let step = find_span(batch, &format!("{thread_id}:step:1")).expect("step span");
        assert_eq!(step["body"]["metadata"]["langgraph_step"], 1);
        assert_eq!(step["body"]["metadata"]["flow_id"], "flow-1");
    }

    #[tokio::test]
    async fn export_is_a_noop_when_share_usage_data_is_off() {
        let mut config = Config::default();
        config.observability.share_usage_data = false;
        // Must return without any host/token resolution or network.
        export_flow_run_trace(
            &config,
            "Daily digest",
            "flow-1",
            "flow:flow-1:uuid-1",
            "completed",
            FlowRunTrigger::Rpc,
            &sample_observations("flow:flow-1:uuid-1"),
        )
        .await;
    }

    #[tokio::test]
    async fn export_with_empty_observations_is_a_noop() {
        let config = Config::default();
        export_flow_run_trace(
            &config,
            "Daily digest",
            "flow-1",
            "flow:flow-1:uuid-1",
            "completed",
            FlowRunTrigger::AppEvent,
            &[],
        )
        .await;
    }

    /// The re-typing hop is a serde round-trip between two independently
    /// declared types, so nothing but a test notices when one of them grows,
    /// renames, or re-tags a field: `to_exporter_observations` would start
    /// silently dropping observations and the trace would quietly lose spans.
    /// This asserts the whole sample survives AND that the fields the exporter
    /// keys spans on come through with their values intact.
    #[test]
    fn re_typing_preserves_every_field() {
        let thread_id = "flow:flow-1:uuid-1";
        let engine = sample_observations(thread_id);
        let converted = to_exporter_observations(&engine);

        assert_eq!(
            converted.len(),
            engine.len(),
            "an observation was dropped — the two GraphObservation types have drifted"
        );
        for (before, after) in engine.iter().zip(converted.iter()) {
            assert_eq!(
                serde_json::to_value(before).unwrap(),
                serde_json::to_value(after).unwrap(),
                "re-typing changed the observation's serialized form"
            );
        }

        // Spot-check the fields the exporter reads directly, so a drift that
        // happened to stay serde-compatible (a renamed field with a matching
        // `#[serde(rename)]`, say) still fails here rather than producing a
        // batch full of empty spans.
        let first = &converted[0];
        assert_eq!(
            first.thread_id.as_ref().map(|t| t.as_str()),
            Some(thread_id)
        );
        assert_eq!(first.run_id.as_str(), "run-9");
        assert_eq!(first.graph_id.as_str(), "workflow");
        assert_eq!(first.offset, 0);
        assert_eq!(converted[2].step, 1);
        assert_eq!(converted[2].ts_ms, 1_020);
    }
}
