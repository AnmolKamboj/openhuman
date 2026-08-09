//! ClickUp Composio provider — incremental Memory Tree ingest for
//! tasks owned by (or assigned to) the connected user.
//!
//! Mirrors the [`crate::openhuman::memory::sync::composio::providers::notion`] layout
//! so anyone familiar with Notion/Slack ingestion can read this without
//! re-learning a new shape:
//!
//! - `provider.rs` — `impl ComposioProvider for ClickUpProvider`
//! - `normalization`  — payload-shape helpers, now `tinycortex::…::normalize::clickup`
//! - `ingest.rs`   — memory_tree document ingest (issue #2885)
//! - `tools.rs`    — `CLICKUP_CURATED` whitelist of Composio actions
//! - `tests.rs`    — unit tests for the helpers + trait metadata
//!
//! Issue: #2288 (introduction); #2885 (memory_tree migration).

// The payload normalisers moved to tinycortex (they are pure Value
// transforms, i.e. driver-side). Aliased under the old module name so
// every `normalization::extract_*` call site below stays unchanged.
use tinycortex::memory::sync::composio::providers::normalize::clickup as normalization;
mod provider;
#[cfg(test)]
mod tests;
pub mod tools;

pub use provider::ClickUpProvider;
pub use tools::CLICKUP_CURATED;
