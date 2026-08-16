//! [`TestHostConfig`] — a concrete, `Default`-able [`MemoryHostConfig`] for
//! tests.
//!
//! `crate::openhuman::memory::core_impl::Config` is `dyn MemoryHostConfig`, which cannot be
//! `Default::default()`ed. The extracted test suites build a config, tweak two
//! or three fields, and pass `&config` into the code under test — a pattern
//! that needs a real struct. This is that struct.
//!
//! It is behind the `test-support` feature and enabled from
//! `tinymemory-core`'s dev-dependencies, so it never enters a shipped build.
//! It is deliberately *not* a mock: the fields are the real config sections
//! with their real serde defaults, so a test that asserts on default behaviour
//! is asserting on the same values production loads.

use std::path::PathBuf;

use super::cloud_providers::CloudProviderCreds;
use super::config::{ComposioMode, MemoryHostConfig};
use super::local_ai::LocalAiConfig;
use super::scheduler_gate::SchedulerGateConfig;
use super::storage_memory::{MemoryConfig, MemoryTreeConfig};

/// A concrete host config for tests. Fields are public — mutate them directly
/// rather than reaching for a builder.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TestHostConfig {
    /// See [`MemoryHostConfig::workspace_dir`].
    pub workspace_dir: PathBuf,
    /// See [`MemoryHostConfig::config_path`].
    pub config_path: PathBuf,
    /// See [`MemoryHostConfig::memory`].
    pub memory: MemoryConfig,
    /// See [`MemoryHostConfig::session_token`]. `None` is signed-out.
    pub session_token: Option<String>,
    /// See [`MemoryHostConfig::memory_tree`].
    pub memory_tree: MemoryTreeConfig,
    /// See [`MemoryHostConfig::scheduler_gate`].
    pub scheduler_gate: SchedulerGateConfig,
    /// See [`MemoryHostConfig::local_ai`].
    pub local_ai: LocalAiConfig,
    /// See [`MemoryHostConfig::cloud_providers`].
    pub cloud_providers: Vec<CloudProviderCreds>,
    /// See [`MemoryHostConfig::embeddings_provider`].
    pub embeddings_provider: Option<String>,
    /// See [`MemoryHostConfig::memory_provider`].
    pub memory_provider: Option<String>,
    /// See [`MemoryHostConfig::api_url`].
    pub api_url: Option<String>,
    /// See [`MemoryHostConfig::default_model`].
    pub default_model: Option<String>,
    /// See [`MemoryHostConfig::default_temperature`].
    pub default_temperature: f64,
    /// See [`MemoryHostConfig::output_language`].
    pub output_language: Option<String>,
    /// See [`MemoryHostConfig::memory_sync_interval_secs`].
    pub memory_sync_interval_secs: Option<u64>,
    /// See [`MemoryHostConfig::onboarding_completed`].
    pub onboarding_completed: bool,
    /// See [`MemoryHostConfig::secrets_encrypt`].
    pub secrets_encrypt: bool,
    /// See [`MemoryHostConfig::composio`].
    pub composio: ComposioMode,
    /// See [`MemoryHostConfig::memory_sources_json`]. Defaults to an empty
    /// array so a test that never touches sources behaves like a fresh install.
    pub memory_sources: Option<serde_json::Value>,
    /// See [`MemoryHostConfig::composio_source_caps_migration_version`].
    pub composio_source_caps_migration_version: u32,
}

#[async_trait::async_trait]
impl MemoryHostConfig for TestHostConfig {
    fn workspace_dir(&self) -> &PathBuf {
        &self.workspace_dir
    }

    fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    fn memory_tree_content_root(&self) -> PathBuf {
        self.memory_tree
            .content_dir
            .clone()
            .unwrap_or_else(|| self.workspace_dir.join("memory_tree").join("content"))
    }

    fn memory(&self) -> &MemoryConfig {
        &self.memory
    }

    fn memory_tree(&self) -> &MemoryTreeConfig {
        &self.memory_tree
    }

    fn scheduler_gate(&self) -> &SchedulerGateConfig {
        &self.scheduler_gate
    }

    fn local_ai(&self) -> &LocalAiConfig {
        &self.local_ai
    }

    fn cloud_providers(&self) -> &Vec<CloudProviderCreds> {
        &self.cloud_providers
    }

    fn embeddings_provider(&self) -> Option<&str> {
        self.embeddings_provider.as_deref()
    }

    fn memory_provider(&self) -> Option<&str> {
        self.memory_provider.as_deref()
    }

    fn workload_local_model(&self, workload: &str) -> Option<String> {
        let raw = match workload {
            "memory" => self.memory_provider.as_deref(),
            "embeddings" => self.embeddings_provider.as_deref(),
            _ => None,
        }?;
        let model = raw.trim().strip_prefix("ollama:")?.trim();
        if model.is_empty() {
            None
        } else {
            Some(model.to_string())
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_arc(&self) -> std::sync::Arc<dyn MemoryHostConfig> {
        std::sync::Arc::new(self.clone())
    }

    fn api_url(&self) -> Option<&str> {
        self.api_url.as_deref()
    }

    fn effective_backend_api_url(&self) -> String {
        // No resolution to do: a test config states its backend URL outright,
        // and the host's env/default ladder is not something to reimplement
        // here.
        self.api_url.clone().unwrap_or_default()
    }

    fn session_token(&self) -> Result<Option<String>, String> {
        // `Ok(None)` — "read fine, not signed in" — rather than `Err`, so a
        // test that never sets a token exercises the signed-out path instead of
        // a credential-store failure.
        Ok(self.session_token.clone())
    }

    fn default_model(&self) -> Option<&str> {
        self.default_model.as_deref()
    }

    fn default_temperature(&self) -> f64 {
        self.default_temperature
    }

    fn output_language(&self) -> Option<&str> {
        self.output_language.as_deref()
    }

    fn memory_sync_interval_secs(&self) -> Option<u64> {
        self.memory_sync_interval_secs
    }

    fn onboarding_completed(&self) -> bool {
        self.onboarding_completed
    }

    fn secrets_encrypt(&self) -> bool {
        self.secrets_encrypt
    }

    fn composio(&self) -> ComposioMode {
        self.composio.clone()
    }

    fn memory_sources_json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .memory_sources
            .clone()
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new())))
    }

    fn set_memory_sources_json(&mut self, value: serde_json::Value) -> anyhow::Result<()> {
        self.memory_sources = Some(value);
        Ok(())
    }

    fn composio_source_caps_migration_version(&self) -> u32 {
        self.composio_source_caps_migration_version
    }

    fn set_composio_source_caps_migration_version(&mut self, version: u32) {
        self.composio_source_caps_migration_version = version;
    }

    fn apply_env_overrides(&mut self) {}

    async fn save(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
