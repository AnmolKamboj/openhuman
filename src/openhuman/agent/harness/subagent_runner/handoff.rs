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
//! that backdoor for its own test harnesses —
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
    let effective_threshold = std::env::var("OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(HANDOFF_OVERSIZE_THRESHOLD_TOKENS);
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
    use std::sync::{Mutex, OnceLock};

    /// `OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS` is process-global and this
    /// suite runs ~11.6k tests in one process, so serialize the two tests in
    /// this module that touch it and restore the prior value on drop.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, val: &str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: caller holds `env_lock()` for the duration of the test.
            unsafe { std::env::set_var(key, val) };
            Self { key, prev }
        }

        fn remove(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: caller holds `env_lock()` for the duration of the test.
            unsafe { std::env::remove_var(key) };
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                // SAFETY: caller holds `env_lock()` for the duration of the test.
                Some(val) => unsafe { std::env::set_var(self.key, val) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn apply_handoff_uses_the_test_env_threshold_when_set() {
        let _guard = env_lock();
        let _env = EnvGuard::set("OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS", "5");
        let cache = ResultHandoffCache::new();

        // 40 bytes / 4 => 10 estimated tokens, comfortably above the
        // env-lowered threshold of 5 but nowhere near the real default of
        // 50_000 — this only exercises the handoff path because the env var
        // was actually read and honoured.
        let oversized = "x".repeat(40);
        let out = apply_handoff(&cache, "some_tool", "task-1", "agent-1", oversized.clone());

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
    fn apply_handoff_falls_back_to_the_default_threshold_when_env_unset() {
        let _guard = env_lock();
        let _env = EnvGuard::remove("OPENHUMAN_TEST_HANDOFF_THRESHOLD_TOKENS");
        let cache = ResultHandoffCache::new();

        // Comfortably below HANDOFF_OVERSIZE_THRESHOLD_TOKENS (50_000 tokens
        // / 200_000 bytes), so with the env var unset (falling back to the
        // real default) the text passes through unchanged.
        let small = "hello world".to_string();
        let out = apply_handoff(&cache, "some_tool", "task-1", "agent-1", small.clone());

        assert_eq!(
            out, small,
            "a small result under the default threshold must pass through unchanged"
        );
    }
}
