//! Shared Chrome DevTools Protocol client for the Meet call window.
//!
//! CDP traffic flows through the in-process transport in [`in_process`],
//! which is a permanent unavailable-error stub: upstream Tauri's Wry
//! runtime uses WKWebView (macOS) and WebKitGTK (Linux), neither of which
//! speaks CDP. See #5478 — this module and its remaining consumers are
//! being removed; nothing here can succeed at runtime.
//!
//! The per-account session opener and the DOM-snapshot parser were removed
//! alongside the webview-account surface they served.

// Transitional. With the account scanners gone the only remaining consumer
// is the Meet stack, which uses a narrow slice of the transport, so the rest
// of `CdpConn` / `WebviewCdpTransport` / `CdpRegistry` is unreferenced.
// Pruning it here would be churn: PR 3 of #5478 deletes this whole module.
#![allow(dead_code)]

pub mod conn;
pub mod in_process;
pub mod target;

pub use conn::CdpConn;
pub use in_process::{set_cef_app_handle, CdpRegistry};
