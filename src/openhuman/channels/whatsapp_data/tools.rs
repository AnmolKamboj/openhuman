//! LLM-callable wrappers for the WhatsApp data store (issue #1341).
//!
//! The store itself lives in the Tauri shell. Each tool dispatches its query
//! over the in-process native request bus
//! ([`crate::core::bus::BUS.native().request`]) to the shell-registered
//! handler, unwraps the typed response, and emits a compact JSON object that
//! includes a `"provider": "whatsapp"` provenance tag so replies can cite
//! WhatsApp as the source.
//!
//! **Graceful degradation.** In a headless / CLI / docker build there is no
//! desktop shell, so no handler is registered. In that case the native
//! dispatch returns [`NativeRequestError::NotInitialized`] or
//! [`NativeRequestError::UnregisteredHandler`]; the tools treat that as
//! "WhatsApp data unavailable (desktop only)" and return an empty, well-formed
//! result rather than surfacing an error to the agent. A genuine handler-side
//! failure ([`NativeRequestError::HandlerFailed`] / [`NativeRequestError::TypeMismatch`])
//! still propagates as a tool error.
//!
//! The write-path `whatsapp_data.ingest` is intentionally NOT wrapped here —
//! it is a scanner-only write path, dispatched by the Tauri shell scanner
//! directly. Exposing it as an agent tool would reopen the read-only boundary
//! this module exists to preserve.

mod list_chats;
mod list_messages;
mod search_messages;

pub use list_chats::WhatsAppDataListChatsTool;
pub use list_messages::WhatsAppDataListMessagesTool;
pub use search_messages::WhatsAppDataSearchMessagesTool;

use tinybus::NativeRequestError;

/// Note surfaced when the WhatsApp data store is unavailable because no desktop
/// shell handler is registered (headless / CLI / docker builds).
pub(crate) const UNAVAILABLE_NOTE: &str = "WhatsApp data unavailable (desktop only)";

/// True when `err` means "no shell handler is wired" — which maps to graceful
/// degradation (an empty result) rather than a tool error.
///
/// The old bus also had an `Init` case for "the registry itself was never
/// initialised". `tinybus::NativeRegistry` is created on first access, so that
/// state no longer exists: an absent handler is the only way to be absent.
pub(crate) fn is_handler_absent(err: &NativeRequestError) -> bool {
    matches!(err, NativeRequestError::UnregisteredHandler { .. })
}
