//! Implementation of process-local inference overrides supplied by the standalone CLI.
//!
//! These values deliberately live outside the serialized [`Config`]. A CLI
//! launch may choose a provider/model without changing what the desktop app or
//! the next invocation uses.

use std::sync::{OnceLock, RwLock};

use super::super::Config;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CliInferenceOverrides {
    provider: Option<String>,
    model: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct InferenceFields {
    default_model: Option<String>,
    chat_provider: Option<String>,
    reasoning_provider: Option<String>,
    agentic_provider: Option<String>,
    coding_provider: Option<String>,
}

impl InferenceFields {
    fn from_config(config: &Config) -> Self {
        Self {
            default_model: config.default_model.clone(),
            chat_provider: config.chat_provider.clone(),
            reasoning_provider: config.reasoning_provider.clone(),
            agentic_provider: config.agentic_provider.clone(),
            coding_provider: config.coding_provider.clone(),
        }
    }

    fn restore_matching(&self, config: &mut Config, applied: &Self) {
        restore_if_applied(
            &mut config.default_model,
            &applied.default_model,
            &self.default_model,
        );
        restore_if_applied(
            &mut config.chat_provider,
            &applied.chat_provider,
            &self.chat_provider,
        );
        restore_if_applied(
            &mut config.reasoning_provider,
            &applied.reasoning_provider,
            &self.reasoning_provider,
        );
        restore_if_applied(
            &mut config.agentic_provider,
            &applied.agentic_provider,
            &self.agentic_provider,
        );
        restore_if_applied(
            &mut config.coding_provider,
            &applied.coding_provider,
            &self.coding_provider,
        );
    }
}

fn restore_if_applied<T: Clone + PartialEq>(current: &mut T, applied: &T, baseline: &T) {
    if current == applied {
        *current = baseline.clone();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub struct AppliedInferenceOverride {
    baseline: InferenceFields,
    applied: InferenceFields,
}

#[derive(Debug, Default)]
struct CliInferenceState {
    overrides: CliInferenceOverrides,
}

static CLI_INFERENCE_STATE: OnceLock<RwLock<CliInferenceState>> = OnceLock::new();

fn state() -> &'static RwLock<CliInferenceState> {
    CLI_INFERENCE_STATE.get_or_init(|| RwLock::new(CliInferenceState::default()))
}

pub(crate) fn set_cli_inference_overrides(provider: Option<&str>, model: Option<&str>) {
    let overrides = CliInferenceOverrides {
        provider: nonempty(provider),
        model: nonempty(model),
    };
    let mut state = state()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.overrides = overrides;
}

pub(crate) fn apply_cli_inference_overrides(config: &mut Config) {
    let overrides = state()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .overrides
        .clone();
    let baseline = InferenceFields::from_config(config);
    apply_overrides(config, &overrides);
    if overrides.provider.is_some() || overrides.model.is_some() {
        config.cli_inference_snapshot = Some(AppliedInferenceOverride {
            baseline,
            applied: InferenceFields::from_config(config),
        });
    }
}

/// Remove process-local launch values from a clone immediately before it is
/// serialized. A handler mutation that replaced an overridden field is kept;
/// only values still equal to the transient overlay are restored.
pub(crate) fn restore_persisted_inference_fields(config: &mut Config) {
    if let Some(applied) = config.cli_inference_snapshot.take() {
        applied.baseline.restore_matching(config, &applied.applied);
    }
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

    if provider_key == "openhuman" {
        if let Some(model) = model.as_ref() {
            config.default_model = Some(model.clone());
        }
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
    use std::path::PathBuf;

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
        let managed_default = config.default_model.clone();

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
        assert_eq!(config.default_model, managed_default);
    }

    #[test]
    fn model_only_preserves_the_active_provider_and_allows_colons_in_model_ids() {
        let mut config = Config::default();
        config.chat_provider = Some("ollama:old:tag".into());
        let managed_default = config.default_model.clone();

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
        assert_eq!(config.default_model, managed_default);
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
    fn cloud_provider_resolves_to_primary_cloud_or_managed_fallback() {
        let mut config = Config::default();
        config.cloud_providers.push(provider());
        config.primary_cloud = Some("p_openai_123".into());

        assert_eq!(resolve_provider_key(&config, "cloud"), "openai-work");
        assert_eq!(resolve_provider_key(&config, ""), "openai-work");

        config.primary_cloud = Some("missing-provider".into());
        assert_eq!(resolve_provider_key(&config, "cloud"), "openhuman");
    }

    #[test]
    fn local_provider_only_uses_the_configured_chat_model() {
        let mut config = Config::default();
        config.local_ai.chat_model_id = "qwen3:8b".into();
        let managed_default = config.default_model.clone();

        apply_overrides(
            &mut config,
            &CliInferenceOverrides {
                provider: Some("ollama".into()),
                model: None,
            },
        );

        assert_eq!(config.chat_provider.as_deref(), Some("ollama:qwen3:8b"));
        assert_eq!(config.reasoning_provider, config.chat_provider);
        assert_eq!(config.agentic_provider, config.chat_provider);
        assert_eq!(config.coding_provider, config.chat_provider);
        assert_eq!(config.default_model, managed_default);
    }

    #[tokio::test]
    async fn reload_refreshes_the_snapshot_before_an_unrelated_save() {
        let root = tempfile::tempdir().expect("temporary config root");
        let config_path = root.path().join("config.toml");
        let workspace_dir = root.path().join("workspace");

        set_cli_inference_overrides(None, None);
        let mut initial = Config::default();
        initial.config_path = config_path.clone();
        initial.workspace_dir = workspace_dir.clone();
        initial.chat_provider = Some("openai:old".into());
        initial.save().await.expect("save initial config");

        set_cli_inference_overrides(Some("ollama"), Some("qwen3:8b"));
        let mut first = Config::load_from_config_path(&config_path, &workspace_dir)
            .await
            .expect("first load");
        first.chat_provider = Some("anthropic:explicit".into());
        first.save().await.expect("save explicit route change");

        let mut reloaded = Config::load_from_config_path(&config_path, &workspace_dir)
            .await
            .expect("reload after explicit change");
        reloaded.onboarding_completed = true;
        reloaded.save().await.expect("save unrelated change");

        set_cli_inference_overrides(None, None);
        let persisted = Config::load_from_config_path(&config_path, &workspace_dir)
            .await
            .expect("load persisted config without overrides");

        assert_eq!(
            persisted.chat_provider.as_deref(),
            Some("anthropic:explicit")
        );
        assert!(persisted.onboarding_completed);
    }

    #[test]
    fn persistence_restore_keeps_handler_changes_and_removes_untouched_overlay_fields() {
        let mut config = Config::default();
        config.config_path = PathBuf::from("/test/config.toml");
        config.chat_provider = Some("openai:before".into());
        let baseline = InferenceFields::from_config(&config);

        let overrides = CliInferenceOverrides {
            provider: Some("ollama".into()),
            model: Some("qwen3:8b".into()),
        };
        apply_overrides(&mut config, &overrides);
        let applied = InferenceFields::from_config(&config);
        config.coding_provider = Some("anthropic:explicit-change".into());

        baseline.restore_matching(&mut config, &applied);

        assert_eq!(config.default_model, baseline.default_model);
        assert_eq!(config.chat_provider, baseline.chat_provider);
        assert_eq!(config.reasoning_provider, baseline.reasoning_provider);
        assert_eq!(config.agentic_provider, baseline.agentic_provider);
        assert_eq!(
            config.coding_provider.as_deref(),
            Some("anthropic:explicit-change")
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
