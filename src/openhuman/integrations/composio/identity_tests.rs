use super::*;
use crate::openhuman::integrations::composio::providers::{
    register_provider, ComposioProvider, ProviderArc, ProviderUserProfile,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Test provider that returns a fixed username (or fails, when
/// `fail` is set). We don't go through Composio at all — the
/// preflight gate just needs the provider's `username` field.
struct StubProvider {
    slug: &'static str,
    username: Option<&'static str>,
    fail: bool,
    calls: AtomicUsize,
}

impl StubProvider {
    fn new(slug: &'static str, username: Option<&'static str>) -> Self {
        Self {
            slug,
            username,
            fail: false,
            calls: AtomicUsize::new(0),
        }
    }
    fn failing(slug: &'static str) -> Self {
        Self {
            slug,
            username: None,
            fail: true,
            calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ComposioProvider for StubProvider {
    fn toolkit_slug(&self) -> &'static str {
        self.slug
    }

    async fn fetch_user_profile(
        &self,
        _ctx: &ProviderContext,
    ) -> Result<ProviderUserProfile, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err("stub provider: forced failure".to_string());
        }
        Ok(ProviderUserProfile {
            toolkit: self.slug.to_string(),
            username: self.username.map(|s| s.to_string()),
            ..Default::default()
        })
    }
}

fn fresh_config_in_workspace(tmp: &std::path::Path) -> Config {
    let mut config = Config::default();
    config.config_path = tmp.join("config.toml");
    config.workspace_dir = tmp.join("workspace");
    config.secrets.encrypt = false;
    config
}

#[tokio::test]
async fn empty_toolkit_short_circuits_to_none() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = fresh_config_in_workspace(tmp.path());
    assert!(connection_identity(&config, "").await.is_none());
    assert!(connection_identity(&config, "   ").await.is_none());
}

#[tokio::test]
async fn unknown_toolkit_returns_none_without_provider_call() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = fresh_config_in_workspace(tmp.path());
    // Toolkit slug that has no registered provider.
    assert!(connection_identity(&config, "not-a-real-toolkit-xyz")
        .await
        .is_none());
}

#[tokio::test]
async fn no_active_connection_short_circuits_before_provider_call() {
    // Register a provider but no connections exist for the toolkit
    // → identity helper should return None without calling
    // fetch_user_profile.
    let stub: ProviderArc = Arc::new(StubProvider::new(
        "stub-no-active",
        Some("would-not-be-returned"),
    ));
    register_provider(stub.clone());

    let tmp = tempfile::tempdir().expect("tempdir");
    let config = fresh_config_in_workspace(tmp.path());
    // Default config has no Composio auth → fetch_connected_integrations
    // returns an empty vec, so the toolkit is not "in active".
    let username = connection_identity(&config, "stub-no-active").await;
    assert!(username.is_none(), "must short-circuit when not connected");
}
