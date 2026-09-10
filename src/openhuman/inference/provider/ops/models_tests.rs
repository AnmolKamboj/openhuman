use super::{resolve_local_runtime_key, url_is_credential_safe};
use crate::openhuman::config::Config;

#[test]
fn omlx_key_falls_back_to_local_ai_api_key() {
    let mut config = Config::default();
    config.local_ai.api_key = Some("  sk-omlx-list  ".into());
    assert_eq!(
        resolve_local_runtime_key("omlx", String::new(), &config),
        "sk-omlx-list"
    );
}

#[test]
fn looked_up_key_wins_over_local_ai() {
    let mut config = Config::default();
    config.local_ai.api_key = Some("sk-local".into());
    assert_eq!(
        resolve_local_runtime_key("omlx", "from-profiles".into(), &config),
        "from-profiles"
    );
}

#[test]
fn non_omlx_slug_does_not_fall_back() {
    let mut config = Config::default();
    config.local_ai.api_key = Some("sk-local".into());
    assert_eq!(
        resolve_local_runtime_key("ollama", String::new(), &config),
        ""
    );
}

/// A bearer credential must never ride a plaintext connection to a remote host.
/// Loopback stays allowed so a locally-hosted backend still authenticates in
/// development.
#[test]
fn credentials_ride_https_or_loopback_only() {
    for url in [
        "https://api.tinyhumans.ai/openai/v1/models",
        "https://staging-api.tinyhumans.ai/openai/v1/models?catalog=openrouter",
        "http://localhost:5005/openai/v1/models",
        "http://127.0.0.1:5005/openai/v1/models",
    ] {
        assert!(url_is_credential_safe(url), "{url}");
    }
}

#[test]
fn credentials_are_withheld_from_remote_plaintext_and_junk_urls() {
    for url in [
        "http://api.tinyhumans.ai/openai/v1/models",
        "http://192.168.1.10:5005/openai/v1/models",
        "not a url",
        "",
    ] {
        assert!(!url_is_credential_safe(url), "{url}");
    }
}
