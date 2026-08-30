//! RPC handlers for the embeddings domain.

use std::collections::HashMap;

use crate::openhuman::config::Config;
use crate::openhuman::security::credentials::AuthService;
use crate::rpc::RpcOutcome;

use super::catalog;
use super::factory::{create_embedding_provider_with_config, model_supports_dimensions};

const LOG_PREFIX: &str = "[embeddings::rpc]";

/// Slug naming the embedder ingestion will actually use, resolved host-side
/// from the `Config` fields the resolution ladder reads.
///
/// Mirrors `tinymemory_core::tree::score::embed::effective_embedder_slug` so
/// `get_settings` no longer calls `tinymemory_core::` directly (#5560).
///
/// `MemoryScoring::embedder_slug()` is not used here for two reasons:
/// (1) `get_settings` is a synchronous config-reading RPC handler and cannot
/// await an async bus call; (2) this function answers "what slug will ingestion
/// use?" — a config-derived prediction that must work even when the module is
/// not loaded. The bus call would give the same answer when the module is
/// running, but would fail gracefully when it is not, offering no benefit over
/// reading the config directly. Keep both implementations in sync whenever the
/// engine's resolution ladder changes.
///
/// Resolution order (matches the engine factory's ladder):
/// 1. Explicit Ollama override — `memory_tree.embedding_endpoint` +
///    `memory_tree.embedding_model` both `Some` and non-empty → `"ollama"`.
/// 2. Deliberate opt-out — `embeddings_provider` trimmed equals `"none"` → `"none"`.
/// 3. Local Ollama via unified workload setting — `workload_local_model("embeddings")`
///    is `Some` → `"ollama"`.
/// 4. User OpenAI-compatible endpoint — `memory.embedding_provider` is
///    `"openai"`, `"custom"`, or starts with `"custom:"` → `"custom"`.
/// 5. Managed cloud session — `auth-profiles.json` exists next to the config
///    file → `"cloud"`.
/// 6. Nothing usable → `"unconfigured"`.
fn effective_embedder_slug_from_config(config: &Config) -> &'static str {
    // 1. Explicit Ollama override.
    if let (Some(ep), Some(model)) = (
        config.memory_tree.embedding_endpoint.as_deref(),
        config.memory_tree.embedding_model.as_deref(),
    ) {
        if !ep.trim().is_empty() && !model.trim().is_empty() {
            return "ollama";
        }
    }
    // 2. Deliberate opt-out.
    if config
        .embeddings_provider
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| s == "none")
    {
        return "none";
    }
    // 3. Local Ollama via unified workload setting.
    if config.workload_local_model("embeddings").is_some() {
        return "ollama";
    }
    // 4. User OpenAI-compatible endpoint.
    let picker = config.memory.embedding_provider.trim();
    if picker == "openai" || picker == "custom" || picker.starts_with("custom:") {
        return "custom";
    }
    // 5. Managed cloud session.
    let session_exists = config
        .config_path
        .parent()
        .map(|dir| dir.join("auth-profiles.json").exists())
        .unwrap_or(false);
    if session_exists {
        return "cloud";
    }
    "unconfigured"
}

/// Dimension to run a Custom (OpenAI-compatible) verification probe at.
///
/// The user-entered `dimensions` field is a guess: for any model outside the
/// `text-embedding-3-*` family we never send the OpenAI `dimensions` request
/// param (see [`model_supports_dimensions`]), so the endpoint returns its own
/// native vector length. Forcing the probe to enforce the guessed length makes
/// every reachable, valid embedding endpoint fail verification whenever the
/// guess (default 1024) differs from the native size — the root cause of
/// issue #4056.
///
/// So we probe a `text-embedding-3-*` model at the configured size (the server
/// honours the param and returns exactly that), but probe every other model at
/// `0`, which disables both the request param and the post-response length
/// guard in `OpenAiEmbedding::embed` — the probe then only has to prove the
/// endpoint can embed, and we learn the real dimension from the returned
/// vector (see [`final_probe_dims`]).
fn probe_dims_for(model: &str, configured: usize) -> usize {
    if model_supports_dimensions(model) {
        configured
    } else {
        0
    }
}

/// Dimension to persist after a successful Custom verification probe.
///
/// For a `text-embedding-3-*` model the endpoint honoured the requested size,
/// so keep the user's `configured` value (Matryoshka). For every other model we
/// probed dimension-agnostically, so adopt the endpoint's actual returned
/// length (`actual`) — the user can't be expected to know it, and storing the
/// real size is what lets the live embed path's length guard pass afterwards.
/// Falls back to `configured` if the probe somehow reported a zero-length
/// vector (defensive — `classify_embed_probe` already rejects empty vectors).
fn final_probe_dims(model: &str, configured: usize, actual: usize) -> usize {
    if model_supports_dimensions(model) || actual == 0 {
        configured
    } else {
        actual
    }
}

/// Returns the current embedding settings plus the provider catalog.
pub async fn get_settings(config: &Config) -> Result<RpcOutcome<serde_json::Value>, String> {
    let provider = &config.memory.embedding_provider;
    let model = &config.memory.embedding_model;
    let dimensions = config.memory.embedding_dimensions;
    let rate_limit = config.memory.embedding_rate_limit_per_min;

    let auth = AuthService::from_config(config);
    let providers: Vec<serde_json::Value> = catalog::all_providers()
        .iter()
        .map(|entry| {
            let has_key = if entry.requires_api_key {
                let cred_provider = format!("embeddings:{}", entry.slug);
                auth.get_provider_bearer_token(&cred_provider, None)
                    .ok()
                    .flatten()
                    .is_some()
            } else {
                false
            };
            serde_json::json!({
                "slug": entry.slug,
                "label": entry.label,
                "description": entry.description,
                "requires_api_key": entry.requires_api_key,
                "requires_endpoint": entry.requires_endpoint,
                "has_api_key": has_key,
                "models": entry.models,
            })
        })
        .collect();

    let vector_search_enabled = {
        let slug = if provider.starts_with("custom:") {
            "custom"
        } else {
            provider.as_str()
        };
        slug != "none"
    };

    // The embedder ingestion will *actually* use. `provider` above is the
    // per-section setting the picker writes; it is NOT authoritative for how
    // embeddings are funded, because the Local AI "Memory embeddings" toggle and
    // the `memory_tree.embedding_endpoint` override both route to local Ollama
    // without rewriting it. Additive field — callers that only need the picker
    // value are unaffected; callers asking "does this bill the managed budget?"
    // must read this one (#5402).
    let effective_provider = effective_embedder_slug_from_config(config);

    let payload = serde_json::json!({
        "provider": provider,
        "effective_provider": effective_provider,
        "model": model,
        "dimensions": dimensions,
        "rate_limit_per_min": rate_limit,
        "providers": providers,
        "vector_search_enabled": vector_search_enabled,
    });

    tracing::debug!(
        provider = provider.as_str(),
        effective_provider,
        model = model.as_str(),
        dimensions,
        vector_search_enabled,
        "{LOG_PREFIX} get_settings"
    );

    Ok(RpcOutcome::new(
        payload,
        vec!["embeddings settings loaded".into()],
    ))
}

/// Updates embedding provider/model/dimensions. If the embedding signature
/// changes, requires `confirm_wipe = true` and wipes memory.
pub async fn update_settings(
    provider: Option<String>,
    model: Option<String>,
    dimensions: Option<usize>,
    custom_endpoint: Option<String>,
    rate_limit_per_min: Option<u32>,
    confirm_wipe: bool,
) -> Result<RpcOutcome<serde_json::Value>, String> {
    use crate::openhuman::config::ops as config_rpc;
    use crate::openhuman::inference::embeddings::format_embedding_signature;

    let mut config = config_rpc::load_config_with_timeout().await?;

    let old_sig = format_embedding_signature(
        &config.memory.embedding_provider,
        &config.memory.embedding_model,
        config.memory.embedding_dimensions,
    );

    let new_provider = provider
        .clone()
        .unwrap_or_else(|| config.memory.embedding_provider.clone());
    let new_model = model
        .clone()
        .unwrap_or_else(|| config.memory.embedding_model.clone());
    // `new_dims`/`new_sig`/`dims_changed` are recomputed after the Custom
    // verification probe auto-detects the endpoint's real vector length
    // (issue #4056), so they must be mutable.
    let mut new_dims = dimensions.unwrap_or(config.memory.embedding_dimensions);
    let mut new_sig = format_embedding_signature(&new_provider, &new_model, new_dims);

    let old_dims = config.memory.embedding_dimensions;
    let mut dims_changed = new_dims != old_dims;
    let mut sig_changed = new_sig != old_sig;

    // Setup-time verification gate (TAURI-RUST-5JR / 4P4): a Custom
    // (OpenAI-compatible) embeddings endpoint — e.g. LM Studio — must prove it
    // can actually embed *before* we accept it. We run one live test embed and
    // only persist the config if it succeeds; any failure (no `/embeddings`
    // route, no model loaded, timeout, 5xx, empty/zero-dim vector) rejects the
    // save so a config that can't embed is never stored (and we never wipe
    // memory for one). Verifying at setup is the fix — we deliberately do NOT
    // try to classify-and-suppress the resulting embed flood in code; any
    // residual flood (e.g. the user unloads the model *after* a good save) is
    // handled on the Sentry side.
    //
    // Only custom endpoints are probed: named catalog providers are
    // embedding-capable by construction, and probing `managed`/`cloud`
    // pre-login would false-fail. Resolve the provider string exactly as it
    // will be stored so the probe targets the real endpoint.
    let effective_provider = match &custom_endpoint {
        Some(ep) if new_provider == "custom" || new_provider.starts_with("custom:") => {
            format!("custom:{ep}")
        }
        _ => new_provider.clone(),
    };
    if effective_provider.starts_with("custom:") {
        // Probe dimension-agnostically for non-`text-embedding-3-*` models so the
        // user's guessed `dimensions` can't fail an otherwise-valid endpoint; the
        // real length is detected from the returned vector below (issue #4056).
        let probe_dims = probe_dims_for(&new_model, new_dims);
        match build_embedder(&config, &effective_provider, &new_model, probe_dims) {
            Ok(embedder) => {
                // Time-box the probe so a black-hole host can't hang the RPC.
                tracing::debug!(
                    provider = effective_provider.as_str(),
                    probe_dims,
                    "{LOG_PREFIX} update_settings verifying embeddings endpoint with a test embed"
                );
                let probe = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    embedder.embed(&["connection test"]),
                )
                .await;
                // Normalize the timeout/result into one shape, then apply the
                // pure verification policy (`classify_embed_probe`, unit-tested).
                let outcome = match probe {
                    Ok(Ok(vectors)) => EmbedProbe::Returned(vectors),
                    Ok(Err(e)) => EmbedProbe::Failed(e.to_string()),
                    Err(_elapsed) => EmbedProbe::TimedOut,
                };
                // Peek the actual vector length before the policy consumes the
                // outcome — on a pass this is the endpoint's real dimension.
                let probe_actual_dims = match &outcome {
                    EmbedProbe::Returned(vectors) => vectors.first().map(|v| v.len()).unwrap_or(0),
                    _ => 0,
                };
                if let Some(reject) = classify_embed_probe(outcome) {
                    // Log the classified error code (never the raw detail — it can
                    // carry endpoint response bodies) so support can distinguish
                    // auth vs wrong-model vs unreachable failures (issue #5017).
                    let reject_code = reject
                        .value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("EMBEDDINGS_VERIFICATION_FAILED");
                    tracing::warn!(
                        provider = effective_provider.as_str(),
                        reject_code,
                        "{LOG_PREFIX} update_settings rejected — embeddings endpoint failed verification"
                    );
                    // Right-feedback (issue #3761): the probe failed. If the
                    // endpoint lists its served models and the requested id
                    // isn't among them, the cause is almost certainly a name
                    // mismatch (e.g. the user entered `bge-m3` but LM Studio
                    // serves `text-embedding-bge-m3`). Replace the generic
                    // failure with an actionable message naming the available
                    // models and the suggested match. Best-effort and only on
                    // the failure path, so a passing config is never blocked by
                    // an endpoint that doesn't expose `/models`. Derive the
                    // endpoint from the payload OR the already-stored
                    // `custom:<url>` provider, so a model-only update to an
                    // existing custom endpoint still gets the guidance.
                    let listed_endpoint = custom_endpoint
                        .as_deref()
                        .or_else(|| effective_provider.strip_prefix("custom:"));
                    if let Some(ep) = listed_endpoint {
                        let api_key = resolve_api_key(&config, "custom");
                        tracing::debug!(
                            provider = effective_provider.as_str(),
                            requested = new_model.as_str(),
                            "{LOG_PREFIX} update_settings: probing endpoint /models for served-id guidance"
                        );
                        match fetch_served_model_ids(ep, &api_key).await {
                            Ok(served) => match check_requested_model_served(&new_model, &served) {
                                Some(better) => {
                                    tracing::warn!(
                                        provider = effective_provider.as_str(),
                                        requested = new_model.as_str(),
                                        served = served.len(),
                                        "{LOG_PREFIX} update_settings: model not in served list — returning name-mismatch guidance"
                                    );
                                    return Ok(better);
                                }
                                None => {
                                    tracing::debug!(
                                        provider = effective_provider.as_str(),
                                        served = served.len(),
                                        "{LOG_PREFIX} update_settings: requested model is served (or list empty) — keeping generic verification error"
                                    );
                                }
                            },
                            Err(e) => {
                                tracing::debug!(
                                    provider = effective_provider.as_str(),
                                    error = %e,
                                    "{LOG_PREFIX} update_settings: /models lookup failed — keeping generic verification error"
                                );
                            }
                        }
                    }
                    return Ok(reject);
                }
                // Passed. Adopt the endpoint's real vector length for every model
                // we probed dimension-agnostically — the user can't be expected to
                // know it, and storing the actual size is what keeps the live embed
                // path's length guard from rejecting future embeds (issue #4056).
                // `text-embedding-3-*` keeps the requested size (server honoured it).
                let detected_dims = final_probe_dims(&new_model, new_dims, probe_actual_dims);
                if detected_dims != new_dims {
                    tracing::info!(
                        provider = effective_provider.as_str(),
                        model = new_model.as_str(),
                        requested = new_dims,
                        detected = detected_dims,
                        "{LOG_PREFIX} update_settings auto-detected custom embedding dimension from probe"
                    );
                    new_dims = detected_dims;
                    new_sig = format_embedding_signature(&new_provider, &new_model, new_dims);
                    dims_changed = new_dims != old_dims;
                    sig_changed = new_sig != old_sig;
                }
                tracing::debug!(
                    provider = effective_provider.as_str(),
                    new_dims,
                    "{LOG_PREFIX} update_settings test embed passed — accepting config"
                );
            }
            Err(e) => {
                // Construction failure (unknown slug / bad config) — surface it
                // rather than persisting a config that can never embed.
                return Err(format!("invalid embedding provider configuration: {e}"));
            }
        }
    }

    // Only require a wipe when dimensions actually change — switching
    // provider/model at the same dimensionality keeps vectors comparable.
    if dims_changed && !confirm_wipe {
        let payload = serde_json::json!({
            "error": "EMBEDDINGS_DIMENSION_CHANGE_REQUIRES_WIPE",
            "old_dimensions": old_dims,
            "new_dimensions": new_dims,
            "old_signature": old_sig,
            "new_signature": new_sig,
            "message": "Changing embedding dimensions invalidates all stored vectors. \
                        Pass confirm_wipe=true to wipe memory and apply.",
        });
        return Ok(RpcOutcome::new(
            payload,
            vec!["embedding dimension change requires wipe confirmation".into()],
        ));
    }

    if dims_changed {
        tracing::warn!(
            old_dims,
            new_dims,
            "{LOG_PREFIX} embedding dimensions changing — wiping memory"
        );
        crate::openhuman::memory::read_rpc::wipe_all_rpc(&config)
            .await
            .map_err(|e| format!("memory wipe failed: {e}"))?;
    }

    // Apply provider
    if let Some(p) = &provider {
        config.memory.embedding_provider = p.clone();
        // Also update the workload routing to keep them in sync
        config.embeddings_provider = Some(match p.as_str() {
            "managed" | "cloud" => "openhuman".to_string(),
            "ollama" => format!("ollama:{new_model}"),
            other => other.to_string(),
        });
    }
    if let Some(m) = &model {
        config.memory.embedding_model = m.clone();
    }
    // Persist `new_dims`, not the raw `dimensions` arg: the Custom verification
    // probe may have auto-detected the endpoint's real length (issue #4056), and
    // `new_dims` already defaults to the stored value when neither a new arg nor
    // detection changed it — so this is a no-op for the unchanged case.
    config.memory.embedding_dimensions = new_dims;
    if let Some(rl) = rate_limit_per_min {
        config.memory.embedding_rate_limit_per_min = rl;
    }
    // Store custom endpoint in a convention field if provided
    if let Some(ep) = &custom_endpoint {
        if new_provider == "custom" || new_provider.starts_with("custom:") {
            config.memory.embedding_provider = format!("custom:{ep}");
        }
    }

    config.save().await.map_err(|e| e.to_string())?;

    if sig_changed {
        crate::openhuman::memory::ops::maintenance::reembed_best_effort(
            &config,
            "embedding settings",
        )
        .await;
    }

    // #5324: this is the exact screen the "embedding budget reached" alert
    // deep-links to, so a provider/endpoint save here is the user completing
    // the remediation. Un-park the jobs that failed under the old
    // (budget-exhausted / misconfigured) provider so memory resumes growing
    // without the user also having to find "Retry failed" in Memory Tree
    // settings.
    //
    // Gated on an actual provider/endpoint/signature touch — NOT unconditional:
    // a save that only nudges `rate_limit_per_min` does not remediate the
    // embedder, so it must leave terminally-failed jobs parked. `provider`
    // covers re-selecting the *same* provider after fixing the account behind
    // it (a legitimate remediation even when the signature is unchanged).
    let is_embedding_remediation = sig_changed || provider.is_some() || custom_endpoint.is_some();
    // #5324: the settings save has already succeeded. A failed un-park must not
    // fail the RPC, but it must be surfaced (not reported as `0`) so a queue
    // that stayed parked isn't presented as remediated.
    let requeue_result = if is_embedding_remediation {
        crate::openhuman::memory::ops::maintenance::retry_failed(&config).await
    } else {
        Ok(0)
    };
    let requeued_count = *requeue_result.as_ref().unwrap_or(&0);
    let requeue_error = requeue_result.as_ref().err().cloned();
    let requeued_note = match &requeue_error {
        None => requeued_count.to_string(),
        Some(e) => format!("error ({e})"),
    };

    tracing::info!(
        provider = config.memory.embedding_provider.as_str(),
        model = config.memory.embedding_model.as_str(),
        dimensions = config.memory.embedding_dimensions,
        sig_changed,
        requeued = requeued_count,
        requeue_error = requeue_error.as_deref().unwrap_or(""),
        "{LOG_PREFIX} update_settings applied"
    );

    let payload = serde_json::json!({
        "provider": config.memory.embedding_provider,
        "model": config.memory.embedding_model,
        "dimensions": config.memory.embedding_dimensions,
        "signature_changed": sig_changed,
        "new_signature": new_sig,
        "requeued_failed_jobs": requeued_count,
        "requeue_error": requeue_error,
    });

    Ok(RpcOutcome::new(
        payload,
        vec![format!(
            "embeddings settings updated (sig_changed={sig_changed} requeued_failed={requeued_note})"
        )],
    ))
}

/// Stores an API key for a specific embedding provider.
pub async fn set_api_key(
    config: &Config,
    provider_slug: &str,
    api_key: &str,
) -> Result<RpcOutcome<serde_json::Value>, String> {
    if provider_slug.is_empty() {
        return Err("provider slug is required".into());
    }
    if api_key.trim().is_empty() {
        return Err("api_key cannot be empty".into());
    }

    let cred_provider = format!("embeddings:{provider_slug}");
    let auth = AuthService::from_config(config);
    auth.store_provider_token(&cred_provider, "default", api_key, HashMap::new(), true)
        .map_err(|e| format!("failed to store embedding API key: {e}"))?;

    // #5324: supplying a BYO key does NOT change the embedding signature, so
    // `ensure_reembed_backfill` has nothing to enqueue — but it is precisely
    // the action that unblocks jobs parked on `budget_exhausted` /
    // `auth_missing`. Requeue them here or they stay dead until the user
    // separately discovers the "Retry failed" button. A store failure is
    // surfaced (not reported as `0`) so the key-stored response can't imply the
    // parked queue was recovered when it wasn't.
    let requeue_result = crate::openhuman::memory::ops::maintenance::retry_failed(config).await;
    let requeued_count = *requeue_result.as_ref().unwrap_or(&0);
    let requeue_error = requeue_result.as_ref().err().cloned();
    let requeued_note = match &requeue_error {
        None => requeued_count.to_string(),
        Some(e) => format!("error ({e})"),
    };

    tracing::info!(
        provider = provider_slug,
        requeued = requeued_count,
        requeue_error = requeue_error.as_deref().unwrap_or(""),
        "{LOG_PREFIX} set_api_key stored"
    );

    Ok(RpcOutcome::new(
        serde_json::json!({ "stored": true, "provider": provider_slug, "requeued_failed_jobs": requeued_count, "requeue_error": requeue_error }),
        vec![format!(
            "embedding API key stored for {provider_slug} (requeued_failed={requeued_note})"
        )],
    ))
}

/// Removes the API key for a specific embedding provider.
pub async fn clear_api_key(
    config: &Config,
    provider_slug: &str,
) -> Result<RpcOutcome<serde_json::Value>, String> {
    if provider_slug.is_empty() {
        return Err("provider slug is required".into());
    }

    let cred_provider = format!("embeddings:{provider_slug}");
    let auth = AuthService::from_config(config);
    let removed = auth
        .remove_profile(&cred_provider, "default")
        .map_err(|e| format!("failed to clear embedding API key: {e}"))?;

    tracing::info!(
        provider = provider_slug,
        removed,
        "{LOG_PREFIX} clear_api_key"
    );

    Ok(RpcOutcome::new(
        serde_json::json!({ "cleared": removed, "provider": provider_slug }),
        vec![format!("embedding API key cleared for {provider_slug}")],
    ))
}

/// Generates embeddings for the given input texts using the currently
/// configured provider.
pub async fn embed(
    config: &Config,
    inputs: &[String],
) -> Result<RpcOutcome<serde_json::Value>, String> {
    let provider_name = &config.memory.embedding_provider;
    let model = &config.memory.embedding_model;
    let dims = config.memory.embedding_dimensions;

    let api_key = resolve_api_key(config, provider_name);

    let custom_endpoint = if provider_name.starts_with("custom:") {
        provider_name
            .strip_prefix("custom:")
            .map(|s: &str| s.to_string())
    } else {
        None
    };

    let provider_slug = if provider_name.starts_with("custom:") {
        "custom"
    } else {
        provider_name.as_str()
    };

    let embedder = create_embedding_provider_with_config(
        config,
        provider_slug,
        model,
        dims,
        &api_key,
        custom_endpoint.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
    let vectors = embedder.embed(&refs).await.map_err(|e| e.to_string())?;

    let actual_dims = vectors.first().map(|v| v.len()).unwrap_or(0);

    tracing::debug!(
        provider = provider_slug,
        model,
        input_count = inputs.len(),
        vector_count = vectors.len(),
        dims = actual_dims,
        "{LOG_PREFIX} embed completed"
    );

    let payload = serde_json::json!({
        "vectors": vectors,
        "dimensions": actual_dims,
        "count": vectors.len(),
        "provider": provider_slug,
        "model": model,
    });

    Ok(RpcOutcome::new(payload, vec!["embedding completed".into()]))
}

/// Tests connectivity to the configured (or specified) embedding provider.
pub async fn test_connection(
    config: &Config,
    provider_slug: Option<&str>,
    model: Option<&str>,
    dims: Option<usize>,
) -> Result<RpcOutcome<serde_json::Value>, String> {
    let slug = provider_slug.unwrap_or(&config.memory.embedding_provider);
    let model = model.unwrap_or(&config.memory.embedding_model);
    let dims = dims.unwrap_or(config.memory.embedding_dimensions);

    let api_key = resolve_api_key(config, slug);

    let custom_endpoint = if slug.starts_with("custom:") {
        slug.strip_prefix("custom:").map(|s| s.to_string())
    } else {
        None
    };

    let provider_tag = if slug.starts_with("custom:") {
        "custom"
    } else {
        slug
    };

    // Probe a Custom endpoint dimension-agnostically (issue #4056): the user's
    // `dims` is a guess, so enforcing it here would make a valid endpoint fail
    // the Test-connection button whenever the guess differs from the native
    // size. Catalog providers keep their fixed `dims`. We still report the
    // requested vs actual dimensions in the payload below.
    let probe_dims = if provider_tag == "custom" {
        probe_dims_for(model, dims)
    } else {
        dims
    };

    let embedder = create_embedding_provider_with_config(
        config,
        provider_tag,
        model,
        probe_dims,
        &api_key,
        custom_endpoint.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    tracing::debug!(
        provider = provider_tag,
        model,
        dims,
        probe_dims,
        "{LOG_PREFIX} test_connection starting"
    );

    match embedder.embed(&["connection test"]).await {
        Ok(vectors) => {
            let actual_dims = vectors.first().map(|v| v.len()).unwrap_or(0);
            let payload = serde_json::json!({
                "success": true,
                "provider": provider_tag,
                "model": model,
                "requested_dimensions": dims,
                "actual_dimensions": actual_dims,
            });
            Ok(RpcOutcome::new(
                payload,
                vec!["connection test passed".into()],
            ))
        }
        Err(e) => {
            let payload = serde_json::json!({
                "success": false,
                "provider": provider_tag,
                "model": model,
                "error": e.to_string(),
            });
            Ok(RpcOutcome::new(
                payload,
                vec![format!("connection test failed: {e}")],
            ))
        }
    }
}

/// Build an embedding provider from the live config — the same construction
/// [`embed`] uses, exposed so other domains (e.g. `codegraph`) can obtain a
/// provider for `signature()` + direct embedding without a JSON-RPC round-trip.
pub fn provider_from_config(config: &Config) -> anyhow::Result<Box<dyn super::EmbeddingProvider>> {
    build_embedder(
        config,
        &config.memory.embedding_provider,
        &config.memory.embedding_model,
        config.memory.embedding_dimensions,
    )
}

/// Construct an embedding provider for an explicit `(provider_name, model,
/// dims)` triple, resolving the stored API key + inline `custom:<url>` endpoint
/// the same way [`embed`] / [`test_connection`] do. Single construction seam so
/// the save-time probe in [`update_settings`] and the live embed path can't
/// drift on slug-normalization / credential-lookup rules.
fn build_embedder(
    config: &Config,
    provider_name: &str,
    model: &str,
    dims: usize,
) -> anyhow::Result<Box<dyn super::EmbeddingProvider>> {
    let api_key = resolve_api_key(config, provider_name);
    let custom_endpoint = provider_name.strip_prefix("custom:").map(|s| s.to_string());
    let provider_slug = if provider_name.starts_with("custom:") {
        "custom"
    } else {
        provider_name
    };
    create_embedding_provider_with_config(
        config,
        provider_slug,
        model,
        dims,
        &api_key,
        custom_endpoint.as_deref(),
    )
}

/// Normalized result of the setup-time test embed in [`update_settings`].
/// Collapses the `Result<Result<_, _>, Elapsed>` timeout shape into one enum so
/// the verification policy can be expressed (and unit-tested) as a pure
/// function over it.
enum EmbedProbe {
    /// The endpoint returned vectors (may still be empty/zero-dim — checked).
    Returned(Vec<Vec<f32>>),
    /// The embed call returned an error; the string is the provider detail.
    Failed(String),
    /// The probe didn't complete within the time box.
    TimedOut,
}

/// Setup-time embeddings verification policy. Returns `None` when the endpoint
/// is verified (accept + persist the config) or `Some(reject)` — the
/// "not saved" RPC payload — otherwise.
///
/// The endpoint must prove it can embed before we accept it: only a non-empty
/// vector passes; every failure mode (no model loaded, no `/embeddings` route,
/// 5xx/auth/network, timeout, empty vector) rejects the save. We do NOT try to
/// classify-and-suppress the resulting embed flood in code — residual floods
/// (e.g. the user unloads the model after a good save) are handled Sentry-side.
/// The known shapes only get a friendlier remediation message.
fn classify_embed_probe(outcome: EmbedProbe) -> Option<RpcOutcome<serde_json::Value>> {
    let reject = |error: &str, message: &str, summary: &str, detail: Option<&str>| {
        let mut body = serde_json::json!({ "error": error, "message": message });
        if let Some(d) = detail {
            // The probe detail is the raw endpoint response body. It can carry the
            // API key (OpenAI's 401 echoes `Incorrect API key provided: sk-…`), and
            // the frontend appends `detail` to the surfaced message — so redact any
            // key/bearer material before it ever leaves the core, for both the UI
            // and logs (#5116). The clean classified `message` is the primary text;
            // the sanitized detail only adds a self-diagnosis hint.
            body["detail"] = serde_json::Value::String(redact_secrets(d));
        }
        Some(RpcOutcome::new(body, vec![summary.to_string()]))
    };

    match outcome {
        // Pass only when the endpoint returns a usable vector.
        EmbedProbe::Returned(vectors)
            if vectors.first().map(|v| !v.is_empty()).unwrap_or(false) =>
        {
            None
        }
        // Reachable but produced no usable vector — not a valid embedder.
        EmbedProbe::Returned(_) => reject(
            "EMBEDDINGS_VERIFICATION_FAILED",
            "The embeddings endpoint responded but returned no vector. Choose an \
             embeddings-capable provider or endpoint, then save again.",
            "test embed returned no vectors — not saved",
            None,
        ),
        EmbedProbe::Failed(detail) => {
            let lower = detail.to_ascii_lowercase();
            // The endpoint IS reachable and correctly shaped (POST /v1/embeddings
            // with the user's model + key — verified conformant by the mock-endpoint
            // regression test). The failures below are all *distinct causes*; issue
            // #5017 was that they collapsed into one generic "test embed failed"
            // message, so a user whose endpoint works for chat couldn't tell that
            // (e.g.) their chosen model isn't an embeddings model, their key was
            // rejected, or the host was unreachable. Order matters: check the
            // specific shapes before the generic fallback.
            if lower.contains("no models loaded") {
                // Reachable but no model loaded (e.g. LM Studio idle).
                reject(
                    "EMBEDDINGS_NO_MODEL_LOADED",
                    "Your local embeddings server (e.g. LM Studio) is running but has no \
                     model loaded. Load an embedding model — in LM Studio use the developer \
                     page or the `lms load` command — then save again.",
                    "embeddings server has no model loaded — not saved",
                    Some(&detail),
                )
            } else if crate::core::observability::is_embedding_endpoint_absent(&lower) {
                // Endpoint exposes no embeddings API (404/405).
                reject(
                    "EMBEDDINGS_ENDPOINT_NO_API",
                    "This endpoint has no embeddings API. Choose an embeddings-capable \
                     provider (Managed, Voyage, OpenAI, Cohere, Ollama) or a different \
                     custom endpoint.",
                    "embeddings endpoint has no embeddings API — not saved",
                    Some(&detail),
                )
            } else if is_embedding_dimension_mismatch(&lower) {
                // Endpoint embedded fine but returned a different vector length than
                // the (Matryoshka) size we requested — a `text-embedding-3-*` model
                // name pointed at a host that ignores the `dimensions` param.
                reject(
                    "EMBEDDINGS_DIMENSION_MISMATCH",
                    "The endpoint returned a vector with a different length than the \
                     dimensions you entered. Set dimensions to match the model's native \
                     output, then save again.",
                    "embeddings endpoint returned mismatched dimensions — not saved",
                    Some(&detail),
                )
            } else if is_embedding_model_incompatible(&lower) {
                // Reachable, authenticated embeddings API that rejected the model —
                // the user pasted a chat/reasoning model (e.g. `gpt-5-mini`) into the
                // embeddings model field. This is the #5017 reporter's exact case:
                // the same model works for chat but is not an embeddings model.
                reject(
                    "EMBEDDINGS_MODEL_INCOMPATIBLE",
                    "That model isn't an embeddings model on this endpoint. A chat model \
                     (the one that works in Chat settings) can't produce embeddings — \
                     enter an embeddings model id (e.g. text-embedding-3-small, bge-m3), \
                     then save again.",
                    "embeddings model is not an embeddings model — not saved",
                    Some(&detail),
                )
            } else if embed_error_mentions_status(&lower, 401)
                || embed_error_mentions_status(&lower, 403)
            {
                // Auth failure — key missing/wrong/lacking embeddings scope. The
                // embeddings key is stored separately from the chat BYOK key, so
                // "works for chat" does not imply the embeddings key is set.
                reject(
                    "EMBEDDINGS_AUTH_FAILED",
                    "The endpoint rejected the API key (401/403). Enter a valid key for \
                     this endpoint — note the embeddings key is stored separately from the \
                     Chat provider key — then save again.",
                    "embeddings endpoint rejected the API key — not saved",
                    Some(&detail),
                )
            } else if is_embedding_endpoint_unreachable(&lower) {
                // Transport-level failure — DNS, refused connection, TLS. The base
                // URL is wrong or the host is down.
                reject(
                    "EMBEDDINGS_ENDPOINT_UNREACHABLE",
                    "Couldn't reach the embeddings endpoint (network/DNS/connection \
                     error). Check the base URL and that the host is reachable, then save \
                     again.",
                    "embeddings endpoint unreachable — not saved",
                    Some(&detail),
                )
            } else {
                // Any other failure (5xx, unclassified) — didn't pass verification.
                reject(
                    "EMBEDDINGS_VERIFICATION_FAILED",
                    "Couldn't verify the embeddings endpoint — the test embed failed. Make \
                     sure the endpoint is reachable and serving an embedding model, then \
                     save again.",
                    "embeddings endpoint failed verification — not saved",
                    Some(&detail),
                )
            }
        }
        EmbedProbe::TimedOut => reject(
            "EMBEDDINGS_ENDPOINT_UNREACHABLE",
            "Couldn't verify the embeddings endpoint — the test embed timed out. Make sure \
             the endpoint is running and reachable, then save again.",
            "embeddings endpoint timed out during verification — not saved",
            None,
        ),
    }
}

/// Whether a lowercased embed-error detail names the given HTTP status, tolerant
/// of the wire shapes the embeddings stack emits:
///   `openai embeddings returned HTTP 401 Unauthorized: …` (tinyagents adapter)
///   `Embedding API error (401 Unauthorized): …`           (parenthesized host shape)
///   `Embedding API error 401 Unauthorized: …`             (bare-status host shape)
/// The bare-status `Embedding API error {code}` form is the one the observability
/// classifier in `src/core/observability.rs` covers; without it, setup-time
/// verification for those hosts fell through to the generic failure code (#5017).
fn embed_error_mentions_status(lower: &str, code: u16) -> bool {
    let code = code.to_string();
    lower.contains(&format!("http {code}"))
        || lower.contains(&format!("({code}"))
        || lower.contains(&format!("embedding api error {code}"))
}

/// A reachable, authenticated embeddings API that **rejected the model id** — the
/// user pointed the embeddings model field at a chat/reasoning model.
///
/// Two tiers of phrasing:
///
/// - **Strong, status-independent phrasings** unambiguously name a model that
///   can't embed. OpenAI returns *HTTP 403* "You are not allowed to generate
///   embeddings from this model" when a chat model (e.g. `gpt-4o-mini`) is used
///   as the embeddings model — a MODEL problem, not an auth problem. Because
///   `classify_embed_probe` checks this **before** the 401/403 auth branch, that
///   403 must be caught here or it falls through and misreports "enter a valid
///   key" (issue #5116). None of these phrases appear in a genuine auth rejection
///   (`Incorrect API key provided …`), so matching them ahead of auth is safe.
/// - **Weak phrasings** (a stray "does not exist" / odd model-name format) are
///   only unambiguous alongside a 400/422 bad-request, so a genuine 5xx or an
///   oversized-input 400 still falls through to the generic failure (issue #5017).
fn is_embedding_model_incompatible(lower: &str) -> bool {
    let strong_model_rejection = lower.contains("not allowed to generate embeddings")
        || lower.contains("does not support embeddings")
        || lower.contains("not an embedding model")
        || lower.contains("is not an embedding")
        || lower.contains("not supported for embeddings")
        || (lower.contains("unsupported") && lower.contains("embedding"));
    if strong_model_rejection {
        return true;
    }
    let bad_request =
        embed_error_mentions_status(lower, 400) || embed_error_mentions_status(lower, 422);
    bad_request
        && (lower.contains("does not exist") || lower.contains("unexpected model name format"))
}

/// Strip API-key / bearer-token material from any text before it reaches the UI
/// or logs. Matches OpenAI-style keys (`sk-…`, including the modern `sk-proj-…`
/// form with embedded hyphens/underscores) and `Bearer <token>` headers, and
/// replaces each **whole** match — the replacements deliberately contain no `sk-`
/// substring, so not even a key *prefix* can surface (#5116).
fn redact_secrets(input: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static SK_KEY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\bsk-[A-Za-z0-9_-]+").unwrap());
    static BEARER_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").unwrap());
    let redacted = SK_KEY_RE.replace_all(input, "[redacted-key]");
    BEARER_RE
        .replace_all(&redacted, "Bearer [redacted]")
        .into_owned()
}

/// The post-response length guard fired: the endpoint embedded but returned a
/// vector whose length differs from the requested (Matryoshka) `dimensions`.
/// Canonical shape from the tinyagents adapter:
/// `openai embed dimension mismatch: expected 1024, got 3072`.
fn is_embedding_dimension_mismatch(lower: &str) -> bool {
    lower.contains("dimension mismatch")
}

/// A transport-level failure (DNS, refused connection, TLS, connect timeout) —
/// the endpoint was never reached, so the base URL is wrong or the host is down.
/// The tinyagents adapter wraps these as
/// `openai embeddings request to <url> failed: <reqwest error>`.
fn is_embedding_endpoint_unreachable(lower: &str) -> bool {
    lower.contains("request to") && lower.contains("failed")
        || lower.contains("connection refused")
        || lower.contains("error sending request")
        || lower.contains("error trying to connect")
        || lower.contains("dns error")
        || lower.contains("failed to lookup address")
        || lower.contains("tcp connect error")
}

/// GET `{endpoint}/models` (OpenAI-compatible) and return the served model ids.
/// Time-boxed and best-effort — any failure returns `Err` and the caller falls
/// back to the live test-embed probe (issue #3761).
async fn fetch_served_model_ids(endpoint: &str, api_key: &str) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)]
    struct ModelEntry {
        id: String,
    }
    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        #[serde(default)]
        data: Vec<ModelEntry>,
    }

    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut req = client.get(&url).timeout(std::time::Duration::from_secs(5));
    if !api_key.trim().is_empty() {
        req = req.bearer_auth(api_key.trim());
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("models request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("models request returned status {}", resp.status()));
    }
    let parsed: ModelsResponse = resp
        .json()
        .await
        .map_err(|e| format!("models parse failed: {e}"))?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

/// Normalize an embedding model id for tolerant *suggestion* matching:
/// lowercase, drop a leading `text-embedding-`, drop a trailing `:tag`. Used
/// only to suggest the right served name — never to silently rewrite the id.
fn normalize_embed_model_id(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    let stripped = lower.strip_prefix("text-embedding-").unwrap_or(&lower);
    stripped.split(':').next().unwrap_or(stripped).to_string()
}

/// Decide whether the requested model is acceptable given the endpoint's served
/// list. Returns `Some(reject)` only when the endpoint reports a non-empty list
/// that does NOT contain the requested id — i.e. we have positive evidence the
/// model isn't loaded. An empty/unknown list returns `None` (defer to the live
/// test-embed probe) so we never block on a server that doesn't expose
/// `/models` (issue #3761).
fn check_requested_model_served(
    requested: &str,
    served: &[String],
) -> Option<RpcOutcome<serde_json::Value>> {
    if served.is_empty() || served.iter().any(|m| m == requested) {
        return None;
    }
    Some(reject_model_not_served(requested, served))
}

/// Build the "model not served" rejection: names what the endpoint actually
/// serves and, when a normalized match exists, suggests the exact name to pick
/// (e.g. `bge-m3` → `text-embedding-bge-m3`). Reuses the
/// `EMBEDDINGS_NO_MODEL_LOADED` error code so the existing Embeddings setup
/// dialog surfaces `message` and keeps the config unsaved (issue #3761).
fn reject_model_not_served(requested: &str, served: &[String]) -> RpcOutcome<serde_json::Value> {
    let want = normalize_embed_model_id(requested);
    let suggestion = served
        .iter()
        .find(|m| normalize_embed_model_id(m) == want)
        .cloned();
    let served_list = served.join(", ");
    let message = match suggestion.as_deref() {
        Some(s) => format!(
            "`{requested}` isn't loaded on this embeddings server — but the same model appears to be served as `{s}`. Select `{s}` (the exact name your server reports), then save again. Available models: {served_list}."
        ),
        None => format!(
            "`{requested}` isn't loaded on this embeddings server. Select one of the loaded models (the exact name your server reports), then save again. Available models: {served_list}."
        ),
    };
    let mut body = serde_json::json!({
        "error": "EMBEDDINGS_NO_MODEL_LOADED",
        "message": message,
        "requested_model": requested,
        "available_models": served,
    });
    if let Some(s) = suggestion {
        body["suggested_model"] = serde_json::Value::String(s);
    }
    RpcOutcome::new(
        body,
        vec!["embedding model not served by endpoint — not saved".to_string()],
    )
}

pub(crate) fn resolve_api_key(config: &Config, provider_name: &str) -> String {
    let slug = if provider_name.starts_with("custom:") {
        "custom"
    } else {
        provider_name
    };
    let cred_provider = format!("embeddings:{slug}");
    let auth = AuthService::from_config(config);
    auth.get_provider_bearer_token(&cred_provider, None)
        .ok()
        .flatten()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "rpc_tests.rs"]
mod tests;
