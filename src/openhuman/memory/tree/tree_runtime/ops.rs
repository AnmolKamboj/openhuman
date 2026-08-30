//! RPC operation wrappers for the tree summarizer.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::openhuman::config::Config;
use crate::openhuman::memory::tree::tree_runtime::{engine, store};
use crate::rpc::RpcOutcome;
use tinycortex::memory::tree::runtime::*;

/// Append raw content to the ingestion buffer.
pub async fn tree_summarizer_ingest(
    config: &Config,
    namespace: &str,
    content: &str,
    timestamp: Option<DateTime<Utc>>,
    metadata: Option<&Value>,
) -> Result<RpcOutcome<Value>, String> {
    store::validate_namespace(namespace)?;
    if content.trim().is_empty() {
        return Err("content must not be empty".to_string());
    }

    let ts = timestamp.unwrap_or_else(Utc::now);
    let path = store::buffer_write(config, namespace.trim(), content, &ts, metadata)
        .map_err(|e| format!("buffer write failed: {e}"))?;

    Ok(RpcOutcome::single_log(
        json!({
            "buffered": true,
            "namespace": namespace.trim(),
            "timestamp": ts.to_rfc3339(),
            "tokens": estimate_tokens(content),
            "path": path.display().to_string(),
            "has_metadata": metadata.is_some(),
        }),
        format!("content buffered for namespace '{}'", namespace.trim()),
    ))
}

/// Trigger the summarization job for a namespace (drain buffer + summarize + propagate).
pub async fn tree_summarizer_run(
    config: &Config,
    namespace: &str,
) -> Result<RpcOutcome<Value>, String> {
    store::validate_namespace(namespace)?;

    let (provider, _model) = create_provider(config)?;
    let ts = Utc::now();

    match engine::run_summarization(config, provider.as_ref(), namespace.trim(), ts).await {
        Ok(Some(node)) => Ok(RpcOutcome::single_log(
            serde_json::to_value(&node).map_err(|e| e.to_string())?,
            format!(
                "summarization completed for '{}': node {} ({} tokens)",
                namespace.trim(),
                node.node_id,
                node.token_count
            ),
        )),
        Ok(None) => Ok(RpcOutcome::single_log(
            json!({ "skipped": true, "reason": "no buffered data" }),
            format!(
                "summarization skipped for '{}': no buffered data",
                namespace.trim()
            ),
        )),
        Err(e) => Err(format!("summarization failed: {e:#}")),
    }
}

/// Query the tree at a specific node or level.
pub async fn tree_summarizer_query(
    config: &Config,
    namespace: &str,
    node_id: Option<&str>,
) -> Result<RpcOutcome<Value>, String> {
    store::validate_namespace(namespace)?;

    let target_id = node_id.unwrap_or("root");
    store::validate_node_id(target_id)?;

    let node = store::read_node(config, namespace.trim(), target_id)
        .map_err(|e| format!("read node: {e}"))?
        .ok_or_else(|| {
            format!(
                "node '{}' not found in namespace '{}'",
                target_id,
                namespace.trim()
            )
        })?;

    let children = store::read_children(config, namespace.trim(), target_id)
        .map_err(|e| format!("read children: {e}"))?;

    let result = QueryResult { node, children };
    Ok(RpcOutcome::single_log(
        serde_json::to_value(&result).map_err(|e| e.to_string())?,
        format!(
            "queried node '{}' in namespace '{}'",
            target_id,
            namespace.trim()
        ),
    ))
}

/// Get tree status/metadata for a namespace.
pub async fn tree_summarizer_status(
    config: &Config,
    namespace: &str,
) -> Result<RpcOutcome<Value>, String> {
    store::validate_namespace(namespace)?;

    let status =
        store::get_tree_status(config, namespace.trim()).map_err(|e| format!("get status: {e}"))?;

    Ok(RpcOutcome::single_log(
        serde_json::to_value(&status).map_err(|e| e.to_string())?,
        format!("tree status for namespace '{}'", namespace.trim()),
    ))
}

/// Rebuild the entire tree from hour leaves (background task).
pub async fn tree_summarizer_rebuild(
    config: &Config,
    namespace: &str,
) -> Result<RpcOutcome<Value>, String> {
    store::validate_namespace(namespace)?;

    let (provider, _model) = create_provider(config)?;

    let status = engine::rebuild_tree(config, provider.as_ref(), namespace.trim())
        .await
        .map_err(|e| format!("rebuild failed: {e:#}"))?;

    Ok(RpcOutcome::single_log(
        serde_json::to_value(&status).map_err(|e| e.to_string())?,
        format!(
            "tree rebuilt for '{}': {} nodes",
            namespace.trim(),
            status.total_nodes
        ),
    ))
}

// ── Helper ─────────────────────────────────────────────────────────────

/// Build the (provider, model) pair the summarizer runs on (#002 FR-007).
///
/// Historically this hard-required local AI ("private + offline"), which left
/// "Build Summary Trees" dead for cloud-only setups (Tencent/OpenRouter with
/// no local Ollama). It now falls back to the **configured cloud chat
/// provider** for the summarization role when local AI is off, returning that
/// provider's model id alongside it so the engine targets the right model
/// (the engine no longer assumes the local model id). The UI shows a
/// Resolve the summarization provider.
///
/// Priority:
/// 1. Local Ollama when `local_ai.runtime_enabled = true`.
/// 2. Cloud via `create_chat_provider` when
///    `memory_tree.cloud_summarization_opt_in = true` — the user has
///    explicitly acknowledged that memory summaries will be sent to an
///    external provider.
/// 3. Error otherwise — "Build Summary Trees" is local-only by default;
///    the user must opt in to cloud summarization via the
///    `memory_tree.cloud_summarization_opt_in` setting.
///
/// Visibility note: `pub(crate)` so the embedded memory driver's
/// [`MemoryTree`](tinycortex_api::provider::MemoryTree) `seal`/`cascade` reach
/// the **same** resolver the RPC path uses. Duplicating the local-AI /
/// cloud-opt-in precedence in the driver would be new policy logic, and the
/// `summarizer_available` doc below is explicit that this function is the
/// single source of truth.
pub(crate) fn create_provider(
    config: &Config,
) -> Result<
    (
        std::sync::Arc<dyn tinyinference::model::ChatModel<()>>,
        String,
    ),
    String,
> {
    // The summarizer applies its own temperature per request
    // (`SUMMARIZATION_TEMP` in `engine`), so the construction temperature here is
    // just a default the per-call value overrides.
    if config.local_ai.runtime_enabled {
        let model = config.local_ai.chat_model_id.clone();
        let provider_string = format!("ollama:{model}");
        tracing::debug!(
            model = %model,
            "[tree_summarizer] building crate-native local Ollama model"
        );
        return crate::openhuman::inference::provider::factory::create_local_chat_model_from_string(
            &provider_string,
            config,
        )
        .map_err(|e| format!("tree summarizer: failed to build local model: {e:#}"));
    }

    if !config.memory_tree.cloud_summarization_opt_in {
        return Err("no summarization provider — enable local AI, or opt in to \
             cloud summarization via the memory_tree.cloud_summarization_opt_in setting"
            .to_string());
    }

    // Cloud path — user has explicitly opted in. Build the configured
    // provider for the summarization role (`memory_provider` hint).
    crate::openhuman::inference::provider::create_chat_model_with_model_id(
        "summarization",
        config,
        config.default_temperature,
    )
    .map_err(|e| format!("tree summarizer: failed to build cloud provider: {e:#}"))
}

/// Whether a summarization provider can be resolved for "Build Summary Trees"
/// under the current config — the single source of truth the memory doctor
/// reuses so its `summary_tree` stage matches the runtime path (#002 FR-007).
///
/// Routes through [`create_provider`] (the SAME resolver the runtime uses):
/// - local AI enabled ⇒ available (local Ollama path).
/// - local AI off + `memory_tree.cloud_summarization_opt_in = true` ⇒
///   available iff the configured summarization-role provider resolves.
/// - local AI off + opt-in `false` (default) ⇒ unavailable — explicit
///   consent required before routing workspace memory summaries to a cloud
///   provider. Enable via the `memory_tree.cloud_summarization_opt_in` setting.
///
/// The provider built for the `Ok` check is dropped — construction is cheap
/// (no network) and confirming by build beats guessing.
pub fn summarizer_available(config: &Config) -> (bool, &'static str) {
    let local = config.local_ai.runtime_enabled;
    match create_provider(config) {
        Ok(_) if local => (
            true,
            "local AI enabled — Build Summary Trees runs on the local model",
        ),
        Ok(_) => (
            true,
            "local AI off — Build Summary Trees runs on the configured cloud provider",
        ),
        Err(_) => (
            false,
            "no summarization provider available — enable local AI, or opt in to cloud summarization (memory_tree.cloud_summarization_opt_in) with a provider set in Connections → API keys → LLM",
        ),
    }
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
