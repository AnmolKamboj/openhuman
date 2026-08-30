//! Composio connection identity resolution.
//!
//! Single source of truth for "what is the username on this Composio
//! connection?". Used by the skill preflight gate (`[github]
//! identity_match = "strict"`) and by any future caller that needs to
//! compare the connected account against another subsystem (e.g. local
//! `git config user.name`).
//!
//! The lookup goes through the per-toolkit
//! [`ComposioProvider::fetch_user_profile`](crate::openhuman::integrations::composio::providers::ComposioProvider::fetch_user_profile)
//! call, which already knows the right Composio action slug for each
//! toolkit (`GITHUB_GET_THE_AUTHENTICATED_USER`,
//! `GMAIL_GET_PROFILE`, …) and the JSON field that holds the username.
//!
//! ## Failure surface
//!
//! Everything in this module is best-effort and returns `Option`:
//!
//! - toolkit not registered → `None`
//! - user not signed in / no active connection for the toolkit → `None`
//! - Composio call fails / returns no username → `None`
//!
//! Callers MUST treat `None` as "couldn't resolve" rather than
//! "username is empty". The preflight gate uses this contract to map
//! `None` into a clear "GitHub identity not resolved — reconnect via
//! `composio_authorize github`" error.

use std::sync::Arc;

use crate::openhuman::config::Config;

use super::ops::fetch_connected_integrations;
use super::providers::{get_provider, ProviderContext};

/// Resolve the connected account's username for the given Composio
/// toolkit, going through the existing per-provider `fetch_user_profile`
/// path.
///
/// Returns `Some(username)` when:
///   1. The toolkit has a registered provider; AND
///   2. The toolkit is currently connected (per
///      [`fetch_connected_integrations`]); AND
///   3. The provider's `fetch_user_profile` call succeeds AND yields a
///      non-empty `username`.
///
/// Returns `None` for any other case. See module docs for the failure
/// contract.
pub async fn connection_identity(config: &Config, toolkit: &str) -> Option<String> {
    let toolkit_norm = toolkit.trim().to_ascii_lowercase();
    if toolkit_norm.is_empty() {
        tracing::debug!("[composio:identity] connection_identity: empty toolkit slug");
        return None;
    }

    // (1) Provider must exist for this toolkit.
    let provider = match get_provider(&toolkit_norm) {
        Some(p) => p,
        None => {
            tracing::debug!(
                toolkit = %toolkit_norm,
                "[composio:identity] no provider registered for toolkit"
            );
            return None;
        }
    };

    // (2) Toolkit must be in the active integrations set. This is the
    //     same source of truth Connections uses.
    let connections = fetch_connected_integrations(config).await;
    let matching = connections
        .iter()
        .find(|c| c.toolkit.eq_ignore_ascii_case(&toolkit_norm));
    if matching.is_none() {
        tracing::debug!(
            toolkit = %toolkit_norm,
            "[composio:identity] toolkit not in active integrations"
        );
        return None;
    }

    // (3) Build a provider context and call fetch_user_profile.
    //     `ProviderContext::from_config` probes the Composio factory and
    //     returns `None` when the user isn't signed in at all — same
    //     short-circuit other consumers rely on.
    let ctx = ProviderContext::from_config(Arc::new(config.clone()), &toolkit_norm, None)?;
    match provider.fetch_user_profile(&ctx).await {
        Ok(profile) => {
            let username = profile.username.as_deref().map(str::trim).unwrap_or("");
            if username.is_empty() {
                tracing::debug!(
                    toolkit = %toolkit_norm,
                    "[composio:identity] provider returned empty username"
                );
                None
            } else {
                tracing::debug!(
                    toolkit = %toolkit_norm,
                    resolved = true,
                    "[composio:identity] resolved username"
                );
                Some(username.to_string())
            }
        }
        Err(e) => {
            tracing::debug!(
                toolkit = %toolkit_norm,
                error = %e,
                "[composio:identity] fetch_user_profile failed"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
