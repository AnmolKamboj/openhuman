//! Hosting provider configuration (`[hosting]`).
//!
//! Where the credential for a hosting provider comes from, and whether the
//! `hosting_*` agent tools are offered at all. Read by
//! [`crate::openhuman::hosting::credentials`] when it resolves an account, and
//! by the tool registry when it decides whether to register the tools.
//!
//! The key may be left empty here: an empty key falls back to the provider's own
//! environment variables, which is what a self-hosted single-tenant deployment
//! wants. A multi-tenant host (OpenCompany) puts each user's key in this section
//! instead, and nothing else in the process can see it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HostingConfig {
    /// Master switch. When `false`, no hosting tool is registered, whatever
    /// credentials exist.
    #[serde(default)]
    pub enabled: bool,

    /// The provider slug to act on — today, `vercel`.
    #[serde(default = "default_provider")]
    pub provider: String,

    /// The provider API key. Empty means "read it from the environment", which
    /// is [`tinyhosts::ProviderKind::credentials_from_env`]'s search order.
    #[serde(default)]
    pub api_key: String,

    /// The team, organization, or account scope to act as. Empty means the
    /// personal account.
    #[serde(default)]
    pub team: String,
}

impl std::fmt::Debug for HostingConfig {
    /// Redacts `api_key`. `Config` derives `Debug` and embeds `HostingConfig`,
    /// so a nested `{:?}` must never render the raw credential.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostingConfig")
            .field("enabled", &self.enabled)
            .field("provider", &self.provider)
            .field(
                "api_key",
                &if self.has_api_key() { "<redacted>" } else { "" },
            )
            .field("team", &self.team)
            .finish()
    }
}

fn default_provider() -> String {
    "vercel".to_string()
}

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_provider(),
            api_key: String::new(),
            team: String::new(),
        }
    }
}

impl HostingConfig {
    /// Whether a credential was configured here, rather than left to the
    /// environment.
    pub fn has_api_key(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    /// The configured team, or `None` when the field was left blank.
    pub fn team(&self) -> Option<&str> {
        let team = self.team.trim();
        (!team.is_empty()).then_some(team)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosting_is_off_and_points_at_vercel_by_default() {
        let config = HostingConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.provider, "vercel");
        assert!(!config.has_api_key());
        assert_eq!(config.team(), None);
    }

    #[test]
    fn a_blank_key_or_team_is_the_same_as_none() {
        let config = HostingConfig {
            api_key: "   ".to_string(),
            team: "  ".to_string(),
            ..HostingConfig::default()
        };

        assert!(!config.has_api_key());
        assert_eq!(config.team(), None);
    }

    #[test]
    fn a_configured_key_and_team_are_reported() {
        let config = HostingConfig {
            enabled: true,
            api_key: "token".to_string(),
            team: " team_abc ".to_string(),
            ..HostingConfig::default()
        };

        assert!(config.has_api_key());
        assert_eq!(config.team(), Some("team_abc"));
    }

    #[test]
    fn the_section_round_trips_through_toml() {
        let config: HostingConfig =
            toml::from_str("enabled = true\napi_key = \"token\"\n").expect("parses");

        assert!(config.enabled);
        assert_eq!(config.provider, "vercel");
        assert_eq!(config.api_key, "token");
    }

    #[test]
    fn the_team_scope_round_trips_through_toml() {
        let config: HostingConfig =
            toml::from_str("enabled = true\nteam = \"team_abc\"\n").expect("parses");

        assert_eq!(config.team(), Some("team_abc"));
    }

    #[test]
    fn debug_formatting_never_renders_the_raw_key() {
        let config = HostingConfig {
            api_key: "super-secret-token".to_string(),
            ..HostingConfig::default()
        };

        let rendered = format!("{config:?}");

        assert!(!rendered.contains("super-secret-token"));
        assert!(rendered.contains("<redacted>"));
    }
}
