// The payload normalisers moved to tinycortex (they are pure Value
// transforms, i.e. driver-side). Aliased under the old module name so
// every `normalization::extract_*` call site below stays unchanged.
use tinycortex::memory::sync::composio::providers::normalize::notion as normalization;
mod provider;
#[cfg(test)]
mod tests;
pub mod tools;

pub use provider::NotionProvider;
pub use tools::NOTION_CURATED;
