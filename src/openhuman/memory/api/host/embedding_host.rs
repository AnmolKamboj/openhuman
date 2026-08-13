//! [`EmbeddingHost`] — provider *construction*, which the host owns.
//!
//! [`super::EmbeddingProvider`] is the contract for a provider that already
//! exists. This trait is the other half: how one comes into being. Resolving an
//! API key from the credential store, knowing which managed cloud endpoint the
//! signed-in user is entitled to, knowing where the local Ollama server is
//! listening — all of that is host policy, and none of it belongs in a memory
//! engine.
//!
//! The core reaches this through a process-global installed at startup, for the
//! same reason [`super::MemoryEventSink`] is a global: the construction sites
//! sit deep inside retrieval and sealing call stacks that already thread a
//! config and a store handle.
//!
//! # Default is failure, not silence
//!
//! Unlike the event sink, an unwired [`EmbeddingHost`] must **not** degrade
//! quietly. A missing sink drops a notification about work that already
//! happened; a missing embedding provider means vectors would be written into
//! the wrong embedding space, or a query would silently return lexical-only
//! results. Both are data corruption with a delayed fuse, so the unwired
//! accessors return `Err`/`None` and every call site is written to propagate.

use std::sync::Arc;

use super::EmbeddingProvider;

/// Builds [`EmbeddingProvider`]s on the core's behalf.
///
/// Object-safe: the core holds one as `Arc<dyn EmbeddingHost>`.
pub trait EmbeddingHost: Send + Sync + std::fmt::Debug {
    /// The API key for `provider`, from the host's credential store.
    ///
    /// Returns `None` when the provider has no stored credential — which is not
    /// an error: a local provider needs none, and an unconfigured cloud one is
    /// a state the caller reports rather than a failure.
    fn resolve_api_key(&self, provider: &str) -> Option<String>;

    /// Base URL of the local Ollama server, honouring the host's env override
    /// and config before falling back to the default.
    fn ollama_base_url(&self) -> String;

    /// The host's default provider — the managed cloud embedder.
    ///
    /// Constructed lazily with respect to authentication: this may be called
    /// before login completes, and the first `embed()` is what fails if the
    /// user is unauthenticated.
    fn default_embedding_provider(&self) -> Arc<dyn EmbeddingProvider>;

    /// Builds a provider from an explicit provider/model/credential triple.
    ///
    /// # Errors
    ///
    /// Returns `Err` when `provider` is not one the host knows how to build, or
    /// when the supplied credentials are unusable for it.
    fn create_embedding_provider_with_credentials(
        &self,
        provider: &str,
        model: &str,
        dims: usize,
        api_key: &str,
        custom_endpoint: Option<&str>,
    ) -> Result<Box<dyn EmbeddingProvider>, String>;

    /// Whether `model` accepts a caller-chosen output dimensionality.
    ///
    /// Asking for dimensions a model does not support is rejected by the
    /// provider at request time, so the core checks first rather than writing a
    /// batch that will fail halfway.
    fn model_supports_dimensions(&self, model: &str) -> bool;

    /// The managed cloud embedder at an explicit model and dimensionality.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the host cannot reach its managed endpoint
    /// configuration.
    fn cloud_embedding_provider(
        &self,
        model: &str,
        dims: usize,
    ) -> Result<Box<dyn EmbeddingProvider>, String>;

    /// The default model id the managed cloud embedder uses.
    fn default_cloud_embedding_model(&self) -> &str;

    /// The dimensionality [`Self::default_cloud_embedding_model`] emits.
    fn default_cloud_embedding_dimensions(&self) -> usize;

    /// An Ollama-backed provider at `base_url`.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the host cannot construct one for `model`.
    fn ollama_embedding_provider(
        &self,
        base_url: &str,
        model: &str,
        dims: usize,
    ) -> Result<Box<dyn EmbeddingProvider>, String>;
}
