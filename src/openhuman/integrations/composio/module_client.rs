//! Calling the connector module, or explaining why it is not there.
//!
//! Every Composio operation now lives in the `tinyconnectors` module, which is
//! loaded through [`crate::openhuman::modules`] — and that is behind the
//! `modules` feature. A build with gates off has no loader, so it has no
//! connectors either.
//!
//! The `#[cfg]` lives here rather than on each handler. Twelve handlers with a
//! feature gate each is twelve chances to gate one of them differently, and the
//! gates-off build only fails on whichever is compiled first. One switch, one
//! message, and the handlers read the same either way.
//!
//! # What the member names do *not* depend on
//!
//! [`methods`] comes from `tinyconnectors_bus`, an ordinary dependency with no
//! feature gate. A gates-off build can still name a member and match on the
//! contract; it just cannot call one.

pub use tinyconnectors_bus::names::methods;

/// Whether a member failure is the module refusing an operation the live route
/// does not offer.
///
/// The two routes are not equivalent — direct mode has no per-user toolkit
/// allowlist, and no webhook endpoint for triggers — and the module says so by
/// name rather than by returning an empty result. A caller that wants to render
/// its own answer for that case has to tell the refusal apart from a real
/// failure, and getting it backwards would show "no curated allowlist" over an
/// outage, leaving the user unaware their integration had broken.
///
/// Matched on the message because that is what crosses the bus: `TinyBus`
/// carries an error name and a string, so the structure of the module's error
/// is flattened by the time it arrives.
#[must_use]
pub fn is_unsupported_by_route(error: &str) -> bool {
    error.contains("is not available over the") && error.contains("route")
}

/// The message a gates-off build answers every connector call with.
#[cfg(not(feature = "modules"))]
const WITHOUT_MODULES: &str =
    "composio is unavailable in this build: connectors run in the `tinyconnectors` module, \
     which needs the `modules` feature";

/// Call one member with an argument and decode its reply.
///
/// # Errors
///
/// Returns the member's failure as the module rendered it, or an explanation
/// when this build has no module loader.
///
/// Note what is *not* an error: a Composio action the provider refused comes
/// back as a successful reply carrying `successful: false`. A caller that
/// checks only for `Err` will report a failed send as a success.
#[cfg(feature = "modules")]
pub async fn call<Request, Reply>(
    config: &crate::openhuman::config::Config,
    member: &str,
    request: Request,
) -> Result<Reply, String>
where
    Request: serde::Serialize + Send,
    Reply: serde::de::DeserializeOwned,
{
    crate::openhuman::modules::connectors::call(config, member, request).await
}

/// Call one member with an argument. Always fails without the `modules` feature.
///
/// # Errors
///
/// Always, explaining that this build has no module loader.
#[cfg(not(feature = "modules"))]
pub async fn call<Request, Reply>(
    _config: &crate::openhuman::config::Config,
    member: &str,
    _request: Request,
) -> Result<Reply, String>
where
    Request: serde::Serialize + Send,
    Reply: serde::de::DeserializeOwned,
{
    Err(format!("{member}: {WITHOUT_MODULES}"))
}

/// Call a member that takes no arguments.
///
/// # Errors
///
/// As [`call`].
#[cfg(feature = "modules")]
pub async fn call_bare<Reply: serde::de::DeserializeOwned>(
    config: &crate::openhuman::config::Config,
    member: &str,
) -> Result<Reply, String> {
    crate::openhuman::modules::connectors::call_bare(config, member).await
}

/// Call a member that takes no arguments. Always fails without `modules`.
///
/// # Errors
///
/// Always, explaining that this build has no module loader.
#[cfg(not(feature = "modules"))]
pub async fn call_bare<Reply: serde::de::DeserializeOwned>(
    _config: &crate::openhuman::config::Config,
    member: &str,
) -> Result<Reply, String> {
    Err(format!("{member}: {WITHOUT_MODULES}"))
}

#[cfg(test)]
#[path = "module_client_tests.rs"]
mod tests;
