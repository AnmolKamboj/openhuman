use super::{resolve_local_runtime_key, synthesize_managed_entry, url_is_credential_safe};
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

/// The managed row is normally seeded by a migration, but a freshly created
/// profile has none — which is what the app falls back to when a stored session
/// is rejected. Managed is the product's own backend, so it must not depend on a
/// user-config row; without this, being signed out surfaced
/// "no cloud provider with id or slug 'openhuman' found".
#[test]
fn managed_entry_is_synthesized_when_the_config_row_is_missing() {
    use crate::openhuman::config::schema::cloud_providers::AuthStyle;
    let entry = synthesize_managed_entry("openhuman").expect("managed entry");
    assert_eq!(entry.slug, "openhuman");
    assert_eq!(entry.auth_style, AuthStyle::OpenhumanJwt);
    assert!(!entry.endpoint.is_empty());
}

/// Only the managed slug synthesizes: anything else must keep reporting an
/// unknown provider rather than being silently treated as managed.
#[test]
fn other_slugs_do_not_synthesize_a_managed_entry() {
    for slug in ["openai", "openrouter", "ollama", "", "openhuman-x"] {
        assert!(synthesize_managed_entry(slug).is_none(), "{slug}");
    }
}
