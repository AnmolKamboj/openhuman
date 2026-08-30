//! Progressive-disclosure handoff cache for oversized tool results.
//!
//! ## Where the implementation lives
//!
//! The cache, placeholder renderer, and content-hygiene helpers are
//! **TinyAgents'** ([`tinyagents::harness::handoff`]): host-agnostic and
//! re-exported here under their historical OpenHuman names so call sites
//! and the RPC/tool surface are unchanged. See that module's docs for the
//! full picture — typed sub-agents (integrations_agent in particular)
//! regularly call tools that return megabyte-scale payloads
//! (`GMAIL_LIST_MESSAGES`, `NOTION_GET_PAGE`, `GOOGLEDRIVE_LIST_FILES`);
//! progressive disclosure stashes the full payload, replaces it in history
//! with a short placeholder (size + preview + `result_id` + how to query
//! it), and exposes an `extract_from_result` tool (see
//! [`super::extract_tool`]) that the sub-agent can call with a targeted
//! query.
//!
//! ## What stays here
//!
//! Resolving the effective oversize threshold is host policy: the crate
//! takes `threshold_tokens` as an explicit parameter (it deliberately
//! dropped an environment-variable backdoor), and this host still needs
//! that backdoor for its external test harnesses —
//! `OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS` lets a test lower the
//! threshold so the handoff path can be exercised on payloads that survive
//! tokenjuice's compaction cap (see e.g.
//! `tests/raw_coverage/agent_large_round25_raw_coverage_e2e.rs`). Never
//! consulted in production (the env var is absent) so there is zero
//! runtime cost.

// `CachedResult`, `HANDOFF_PREVIEW_CHARS`, `build_handoff_placeholder` and
// `clean_tool_output` have no current OpenHuman call site (the crate now
// owns the only callers, inside `apply_handoff`/`build_handoff_placeholder`
// itself) but are kept re-exported here for surface parity with the
// pre-migration module and in case a future caller needs them directly.
#[allow(unused_imports)]
pub(crate) use tinyagents::harness::handoff::{
    build_handoff_placeholder, chunk_content, clean_tool_output, CachedResult, ResultHandoffCache,
    HANDOFF_MAX_ENTRIES, HANDOFF_OVERSIZE_THRESHOLD_TOKENS, HANDOFF_PREVIEW_CHARS,
};

/// Apply the progressive-disclosure handoff to a tool result. Resolves the
/// effective oversize threshold from `OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS`
/// when set, falling back to [`HANDOFF_OVERSIZE_THRESHOLD_TOKENS`] otherwise,
/// then delegates the cache/placeholder logic to
/// [`tinyagents::harness::handoff::apply_handoff`].
pub(crate) fn apply_handoff(
    cache: &ResultHandoffCache,
    tool_name: &str,
    task_id: &str,
    agent_id: &str,
    result_text: String,
) -> String {
    let threshold_override = std::env::var("OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS").ok();
    let effective_threshold = resolve_handoff_threshold(threshold_override.as_deref());
    apply_handoff_with_threshold(
        cache,
        tool_name,
        task_id,
        agent_id,
        result_text,
        effective_threshold,
    )
}

fn resolve_handoff_threshold(threshold_override: Option<&str>) -> usize {
    threshold_override
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(HANDOFF_OVERSIZE_THRESHOLD_TOKENS)
}

fn apply_handoff_with_threshold(
    cache: &ResultHandoffCache,
    tool_name: &str,
    task_id: &str,
    agent_id: &str,
    result_text: String,
    effective_threshold: usize,
) -> String {
    tinyagents::harness::handoff::apply_handoff(
        cache,
        tool_name,
        task_id,
        agent_id,
        result_text,
        effective_threshold,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_handoff_uses_an_injected_threshold() {
        let cache = ResultHandoffCache::new();

        // 40 bytes / 4 => 10 estimated tokens, comfortably above the
        // injected threshold of 5 but nowhere near the real default of 50_000.
        let oversized = "x".repeat(40);
        let out = apply_handoff_with_threshold(
            &cache,
            "some_tool",
            "task-1",
            "agent-1",
            oversized.clone(),
            5,
        );

        assert_ne!(
            out, oversized,
            "result above the env-lowered threshold should be replaced with a placeholder"
        );
        assert!(
            out.contains("result_id"),
            "placeholder should mention how to retrieve the cached result: {out}"
        );
    }

    #[test]
    fn threshold_resolution_accepts_valid_overrides_and_rejects_invalid_ones() {
        assert_eq!(resolve_handoff_threshold(Some("5")), 5);
        assert_eq!(
            resolve_handoff_threshold(Some("not-a-number")),
            HANDOFF_OVERSIZE_THRESHOLD_TOKENS
        );
        assert_eq!(
            resolve_handoff_threshold(None),
            HANDOFF_OVERSIZE_THRESHOLD_TOKENS
        );
    }

    #[test]
    fn apply_handoff_uses_the_default_threshold_when_injected() {
        let cache = ResultHandoffCache::new();

        // Comfortably below HANDOFF_OVERSIZE_THRESHOLD_TOKENS (50_000 tokens
        // / 200_000 bytes), so with the env var unset (falling back to the
        // real default) the text passes through unchanged.
        let small = "hello world".to_string();
        let out = apply_handoff_with_threshold(
            &cache,
            "some_tool",
            "task-1",
            "agent-1",
            small.clone(),
            HANDOFF_OVERSIZE_THRESHOLD_TOKENS,
        );

        assert_eq!(
            out, small,
            "a small result under the default threshold must pass through unchanged"
        );
    }
}
