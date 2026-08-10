//! Compatibility surface for the remaining CDP consumers.
//!
//! Upstream Tauri's native WebView runtime intentionally does not expose the
//! Chromium DevTools Protocol transport used by the removed CEF runtime. Keep
//! scanner registration deterministic and return a clear unavailable error for
//! CDP-dependent operations until they are migrated to supported WebView APIs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::broadcast;

#[allow(dead_code)]
pub const CALL_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Clone, Debug)]
pub struct EventFrame {
    pub method: String,
    pub params: Value,
    pub session_id: String,
}

pub struct WebviewCdpTransport {
    label: String,
    events_tx: broadcast::Sender<EventFrame>,
}

impl WebviewCdpTransport {
    pub async fn call(
        self: &Arc<Self>,
        method: &str,
        _params: Value,
        _session_id: Option<&str>,
    ) -> Result<Value, String> {
        Err(format!(
            "{method}: CDP is unavailable with the upstream Tauri WebView runtime"
        ))
    }

    pub async fn call_with_timeout(
        self: &Arc<Self>,
        method: &str,
        params: Value,
        session_id: Option<&str>,
        _timeout: Duration,
    ) -> Result<Value, String> {
        self.call(method, params, session_id).await
    }

    pub fn subscribe_events(self: &Arc<Self>) -> broadcast::Receiver<EventFrame> {
        self.events_tx.subscribe()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Default)]
pub struct CdpRegistry {
    transports: Mutex<HashMap<String, Arc<WebviewCdpTransport>>>,
}

impl CdpRegistry {
    pub fn by_label(&self, label: &str) -> Option<Arc<WebviewCdpTransport>> {
        self.transports.lock().ok()?.get(label).cloned()
    }

    pub fn by_account(&self, account_id: &str) -> Option<Arc<WebviewCdpTransport>> {
        self.by_label(&format!("acct_{account_id}"))
    }

    pub fn forget_label(&self, label: &str) {
        if let Ok(mut transports) = self.transports.lock() {
            transports.remove(label);
        }
    }

    pub fn forget_account(&self, account_id: &str) {
        self.forget_label(&format!("acct_{account_id}"));
    }
}

pub fn set_cef_app_handle(_app: tauri::AppHandle<crate::AppRuntime>) {}

pub fn install_for_label(label: &str) -> Result<Arc<WebviewCdpTransport>, String> {
    Err(format!(
        "CDP is unavailable with the upstream Tauri WebView runtime (webview={label})"
    ))
}

#[allow(dead_code)]
pub fn install_for_webview(
    _registry: &CdpRegistry,
    webview: tauri::Webview<crate::AppRuntime>,
) -> Result<Arc<WebviewCdpTransport>, String> {
    install_for_label(webview.label())
}
