//! Process-local inference overrides supplied by the standalone CLI.
//!
//! These values deliberately live outside the serialized [`Config`]. A CLI
//! launch may choose a provider/model without changing what the desktop app or
//! the next invocation uses.

use std::sync::{OnceLock, RwLock};

use super::Config;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CliInferenceOverrides {
    provider: Option<String>,
    model: Option<String>,
}

static CLI_INFERENCE_OVERRIDES: OnceLock<RwLock<CliInferenceOverrides>> = OnceLock::new();

pub(crate) fn set_cli_inference_overrides(provider: Option<&str>, model: Option<&str>) {
    let overrides = CliInferenceOverrides {
        provider: nonempty(provider),
        model: nonempty(model),
    };
    *CLI_INFERENCE_OVERRIDES
        .get_or_init(|| RwLock::new(CliInferenceOverrides::default()))
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = overrides;
}

pub(crate) fn apply_cli_inference_overrides(config: &mut Config) {
    let overrides = CLI_INFERENCE_OVERRIDES
        .get_or_init(|| RwLock::new(CliInferenceOverrides::default()))
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    apply_overrides(config, &overrides);
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn apply_overrides(config: &mut Config, overrides: &CliInferenceOverrides) {
    if overrides.provider.is_none() && overrides.model.is_none() {
        return;
    }

    let current_chat = config.chat_provider.as_deref().unwrap_or("").trim();
    let requested_provider = overrides.provider.as_deref().unwrap_or(current_chat);
    let (requested_key, embedded_model) = split_provider_route(requested_provider);
    let provider_key = resolve_provider_key(config, requested_key);

    let current_model = if overrides.provider.is_none()
        || resolve_provider_key(config, split_provider_route(current_chat).0) == provider_key
    {
        split_provider_route(current_chat).1
    } else {
        None
    };

    let model = overrides
        .model
        .clone()
        .or_else(|| embedded_model.map(str::to_string))
        .or_else(|| current_model.map(str::to_string))
        .or_else(|| provider_default_model(config, &provider_key).map(str::to_string));

    let is_managed = provider_key == "openhuman";
    if is_managed {
        if let Some(model) = model.as_ref() {
            config.default_model = Some(model.clone());
        }
    } else if let Some(model) = overrides.model.as_ref() {
        config.default_model = Some(model.clone());
    }

    let route = match (provider_key.as_str(), model.as_deref()) {
        ("" | "openhuman", _) => "openhuman".to_string(),
        (provider, Some(model)) => format!("{provider}:{model}"),
        (provider, None) => provider.to_string(),
    };

    config.chat_provider = Some(route.clone());
    config.reasoning_provider = Some(route.clone());
    config.agentic_provider = Some(route.clone());
    config.coding_provider = Some(route);
}

fn split_provider_route(route: &str) -> (&str, Option<&str>) {
    let route = route.trim();
    match route.split_once(':') {
        Some((provider, model)) if !model.trim().is_empty() => {
            (provider.trim(), Some(model.trim()))
        }
        Some((provider, _)) => (provider.trim(), None),
        None => (route, None),
    }
}

/// Accept either the user-facing provider slug or the opaque provider id
/// stored in config. The routing factory consumes slugs, so ids are resolved
/// before the transient route is installed.
fn resolve_provider_key(config: &Config, requested: &str) -> String {
    let requested = requested.trim();
    if requested.is_empty() || requested == "cloud" {
        return config
            .primary_cloud
            .as_deref()
            .and_then(|id| config.cloud_providers.iter().find(|entry| entry.id == id))
            .map(|entry| entry.slug.trim())
            .filter(|slug| !slug.is_empty())
            .unwrap_or("openhuman")
            .to_string();
    }

    let requested = match requested {
        "lm_studio" | "lm-studio" => "lmstudio",
        other => other,
    };

    config
        .cloud_providers
        .iter()
        .find(|entry| entry.id == requested || entry.slug == requested)
        .map(|entry| entry.slug.trim())
        .filter(|slug| !slug.is_empty())
        .unwrap_or(requested)
        .to_string()
}

fn provider_default_model<'a>(config: &'a Config, provider: &str) -> Option<&'a str> {
    if matches!(provider, "ollama" | "lmstudio" | "omlx") {
        return nonempty_str(&config.local_ai.chat_model_id);
    }
    if matches!(provider, "openhuman" | "cloud") {
        return config.default_model.as_deref().and_then(nonempty_str);
    }
    config
        .cloud_providers
        .iter()
        .find(|entry| entry.slug == provider || entry.id == provider)
        .and_then(|entry| entry.default_model.as_deref())
        .and_then(nonempty_str)
}

fn nonempty_str(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::config::schema::{AuthStyle, CloudProviderCreds};

    fn provider() -> CloudProviderCreds {
        CloudProviderCreds {
            id: "p_openai_123".into(),
            slug: "openai-work".into(),
            label: "OpenAI work".into(),
            endpoint: "https://api.openai.com/v1".into(),
            auth_style: AuthStyle::Bearer,
            default_model: Some("gpt-default".into()),
            ..Default::default()
        }
    }

    #[test]
    fn provider_id_and_model_override_all_interactive_workloads() {
        let mut config = Config::default();
        config.cloud_providers.push(provider());

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: Some("p_openai_123".into()),
                model: Some("gpt-custom".into()),
            },
        );

        for route in [
            &config.chat_provider,
            &config.reasoning_provider,
            &config.agentic_provider,
            &config.coding_provider,
        ] {
            assert_eq!(route.as_deref(), Some("openai-work:gpt-custom"));
        }
        assert_eq!(config.default_model.as_deref(), Some("gpt-custom"));
    }

    #[test]
    fn model_only_preserves_the_active_provider_and_allows_colons_in_model_ids() {
        let mut config = Config::default();
        config.chat_provider = Some("ollama:old:tag".into());

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: None,
                model: Some("qwen3:8b".into()),
            },
        );

        assert_eq!(config.chat_provider.as_deref(), Some("ollama:qwen3:8b"));
        assert_eq!(config.reasoning_provider, config.chat_provider);
        assert_eq!(config.agentic_provider, config.chat_provider);
        assert_eq!(config.coding_provider, config.chat_provider);
    }

    #[test]
    fn provider_only_uses_its_configured_default_model() {
        let mut config = Config::default();
        config.cloud_providers.push(provider());

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: Some("openai-work".into()),
                model: None,
            },
        );

        assert_eq!(
            config.chat_provider.as_deref(),
            Some("openai-work:gpt-default")
        );
    }

    #[test]
    fn managed_provider_keeps_its_sentinel_route_and_uses_default_model() {
        let mut config = Config::default();

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: Some("openhuman".into()),
                model: Some("reasoning-v1".into()),
            },
        );

        assert_eq!(config.chat_provider.as_deref(), Some("openhuman"));
        assert_eq!(config.reasoning_provider, config.chat_provider);
        assert_eq!(config.default_model.as_deref(), Some("reasoning-v1"));
    }

    #[test]
    fn lm_studio_alias_is_normalized_to_factory_route() {
        let mut config = Config::default();

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: Some("lm_studio".into()),
                model: Some("local-model".into()),
            },
        );

        assert_eq!(
            config.chat_provider.as_deref(),
            Some("lmstudio:local-model")
        );
    }
}
