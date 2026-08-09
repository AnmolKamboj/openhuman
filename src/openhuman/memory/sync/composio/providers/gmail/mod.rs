// The Gmail post-processor moved to tinycortex (a pure Value transform, i.e.
// driver-side). Aliased under the old module name so the single call site in
// `provider.rs` stays unchanged.
use tinycortex::memory::sync::composio::providers::normalize::gmail_post_process as post_process;
mod provider;
#[cfg(test)]
mod tests;
pub mod tools;

pub use provider::GmailProvider;
pub use tools::GMAIL_CURATED;
