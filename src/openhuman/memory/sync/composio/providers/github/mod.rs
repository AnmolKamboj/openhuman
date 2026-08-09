//! GitHub Composio provider — incremental Memory Tree ingest for issues and
//! pull requests involving the connected user.
//!
//! Mirrors the [`crate::openhuman::memory::sync::composio::providers::clickup`] layout so
//! anyone familiar with ClickUp/Notion ingestion can read this without
//! re-learning a new shape:
//!
//! - `provider.rs` — `impl ComposioProvider for GitHubProvider`
//! - `normalization`  — payload-shape helpers, now `tinycortex::…::normalize::github`
//! - `tools.rs`    — `GITHUB_CURATED` whitelist of Composio actions
//! - `tests.rs`    — unit tests for the helpers + trait metadata
//!
//! Issue: #2408.

// The payload normalisers moved to tinycortex (they are pure Value
// transforms, i.e. driver-side). Aliased under the old module name so
// every `normalization::extract_*` call site below stays unchanged.
use tinycortex::memory::sync::composio::providers::normalize::github as normalization;
mod provider;
#[cfg(test)]
mod tests;
pub mod tools;

pub use provider::GitHubProvider;
pub use tools::GITHUB_CURATED;
