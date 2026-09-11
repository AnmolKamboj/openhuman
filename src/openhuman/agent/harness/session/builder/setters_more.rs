//! `AgentBuilder` fluent setters, continued.
//!
//! Split out of [`super::setters`] purely to stay under the repo's
//! per-file line limit (`scripts/ci/check-openhuman-rust-layout.mjs`) — same
//! module, same `impl AgentBuilder` surface, no behavioural distinction from
//! the setters in the sibling file.

use crate::openhuman::agent::harness::session::types::AgentBuilder;
use crate::openhuman::agent::harness::TriggerMemoryAgent;
use std::sync::Arc;

impl AgentBuilder {
    /// Wire an oversized-tool-result summarizer into the agent. The live
    /// TinyAgents turn path passes it to `ToolOutputMiddleware`, which calls
    /// [`crate::openhuman::agent::tinyagents::payload_summarizer::PayloadSummarizer::maybe_summarize_in_parent`]
    /// on successful tool output and replaces the raw payload with the
    /// compressed summary on success. Currently set only for the orchestrator
    /// session by [`Agent::build_session_agent_inner`].
    pub fn payload_summarizer(
        mut self,
        summarizer: Arc<
            dyn crate::openhuman::agent::tinyagents::payload_summarizer::PayloadSummarizer,
        >,
    ) -> Self {
        self.payload_summarizer = Some(summarizer);
        self
    }

    /// Forward the target agent definition's pre-turn memory policy.
    pub fn trigger_memory_agent(mut self, policy: TriggerMemoryAgent) -> Self {
        self.trigger_memory_agent = Some(policy);
        self
    }

    /// Installs pre-execution policy middleware for tool calls.
    ///
    /// The default policy allows all calls. Custom policies can deny a call
    /// before `Tool::execute_with_options` runs.
    pub fn tool_policy(
        mut self,
        policy: Arc<dyn crate::openhuman::agent::tool_policy::ToolPolicy>,
    ) -> Self {
        self.tool_policy = Some(policy);
        self
    }

    /// Attach the production [`ArchivistHook`] instance so the session
    /// turn loop can call [`ArchivistHook::flush_open_segment`] at
    /// session-wind-down time, guaranteeing the trailing open segment is
    /// always finalized with an LLM recap + embedding.
    ///
    /// Set from `build_session_agent_inner` when
    /// `config.learning.episodic_capture_enabled` is `true` and a
    /// SQLite connection is available. Callers that construct an `Agent`
    /// directly (tests, CLI) can leave this `None` — flush is a no-op
    /// when the hook is absent.
    pub fn archivist_hook(
        mut self,
        hook: Option<Arc<crate::openhuman::agent::harness::archivist::ArchivistHook>>,
    ) -> Self {
        self.archivist_hook = hook;
        self
    }

    /// Set the per-agent TokenJuice tool-output compression profile.
    pub fn tokenjuice_compression(
        mut self,
        profile: crate::openhuman::inference::tokenjuice::AgentTokenjuiceCompression,
    ) -> Self {
        self.tokenjuice_compression = profile;
        self
    }
}
