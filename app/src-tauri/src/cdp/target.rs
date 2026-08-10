//! CDP target discovery + per-attach helpers.
//!
//! Each CEF webview is its own browser instance with its own DevTools
//! channel (see [`super::in_process`]), so the multi-target multiplexer
//! that used to live in this module has been simplified — there is no
//! HTTP `/json/version` discovery and no remote attach. The remaining
//! helpers (`Target.getTargets` walk, `Target.attachToTarget`
//! flatten-attach, detach) still apply because the page itself may
//! contain iframes / workers that the scanners care about.

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Runtime};

use super::{in_process::CdpRegistry, CdpConn};

#[derive(Debug, Clone)]
pub struct CdpTarget {
    pub id: String,
    pub kind: String,
    pub url: String,
}

/// Parse the response of a `Target.getTargets` CDP call into a list of
/// targets. Public so scanners using the lower-level [`CdpConn::call`]
/// can interpret target lists.
pub fn parse_targets(v: &Value) -> Vec<CdpTarget> {
    v.get("targetInfos")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(CdpTarget {
                        id: t.get("targetId")?.as_str()?.to_string(),
                        kind: t.get("type")?.as_str()?.to_string(),
                        url: t
                            .get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get a [`CdpConn`] for a webview keyed by its concrete label
/// (e.g. `"meet-call-<request_id>"`).
///
/// Falls back to [`super::in_process::install_for_label`] on a cache
/// miss so a transient install race at window creation doesn't
/// permanently lock the surface out of CDP.
pub fn conn_for_label<R: Runtime>(app: &AppHandle<R>, label: &str) -> Result<CdpConn, String> {
    let registry = app
        .try_state::<CdpRegistry>()
        .ok_or_else(|| "CdpRegistry not managed by app".to_string())?;
    if let Some(transport) = registry.by_label(label) {
        return Ok(CdpConn::new(transport));
    }
    let transport = super::in_process::install_for_label(label)
        .map_err(|e| format!("no cdp transport for label {label} (install retry: {e})"))?;
    Ok(CdpConn::new(transport))
}

/// Full short-lived attach sequence keyed by the webview's concrete
/// label: look up the [`CdpRegistry`] transport, find the matching page
/// target via `Target.getTargets`, then attach with `flatten: true`.
/// Used by the Meet call window (label `meet-call-{request_id}`).
pub async fn connect_and_attach_matching_in_process_by_label<R, F>(
    app: &AppHandle<R>,
    label: &str,
    pred: F,
) -> Result<(CdpConn, String), String>
where
    R: Runtime,
    F: Fn(&CdpTarget) -> bool,
{
    let cdp = conn_for_label(app, label)?;
    attach_matching_on_conn(cdp, pred).await
}

async fn attach_matching_on_conn<F>(mut cdp: CdpConn, pred: F) -> Result<(CdpConn, String), String>
where
    F: Fn(&CdpTarget) -> bool,
{
    let target = find_page_target_where(&mut cdp, pred).await?;
    let attach = cdp
        .call(
            "Target.attachToTarget",
            json!({ "targetId": target.id, "flatten": true }),
            None,
        )
        .await?;
    let session = attach
        .get("sessionId")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "attach missing sessionId".to_string())?
        .to_string();
    Ok((cdp, session))
}

/// Generalised target search — caller supplies the predicate
/// (url-hash marker, title marker, etc).
pub async fn find_page_target_where<F>(cdp: &mut CdpConn, pred: F) -> Result<CdpTarget, String>
where
    F: Fn(&CdpTarget) -> bool,
{
    let targets_v = cdp.call("Target.getTargets", json!({}), None).await?;
    let targets = parse_targets(&targets_v);
    targets
        .into_iter()
        .find(|t| t.kind == "page" && pred(t))
        .ok_or_else(|| "no matching page target".to_string())
}
