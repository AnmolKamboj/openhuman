//! [`UsageInfo`] — token accounting returned by an inference provider.
//!
//! Lives in the contract crate because both sides name it: the host's chat
//! providers produce it, and the memory subsystem's summariser threads it back
//! out so callers can attribute cost to a summarisation run. It is inert data
//! with no dependencies, so it costs the contract crate nothing.

/// Token usage information returned by the provider after an inference call.
#[derive(Debug, Clone, Default)]
pub struct UsageInfo {
    /// Number of tokens in the input/prompt.
    pub input_tokens: u64,
    /// Number of tokens in the output/completion.
    pub output_tokens: u64,
    /// Total context window size for the model (0 if unknown).
    pub context_window: u64,
    /// Number of input tokens that were served from the KV cache
    /// (returned by backends that support prompt caching, e.g. via
    /// `openhuman.usage.cached_input_tokens` or
    /// `prompt_tokens_details.cached_tokens`).
    pub cached_input_tokens: u64,
    /// Number of input tokens written into a provider prompt/KV cache on this
    /// request (cache-creation / cache-write tokens). Distinct from
    /// `cached_input_tokens` (cache reads). Zero when the provider does not
    /// report a cache-write breakdown.
    pub cache_creation_tokens: u64,
    /// Number of reasoning/thinking output tokens when the provider exposes
    /// them separately from `output_tokens`. Zero when unavailable.
    pub reasoning_tokens: u64,
    /// Amount billed for this request in USD (from
    /// `openhuman.billing.charged_amount_usd`). Zero when unavailable.
    pub charged_amount_usd: f64,
}
