//! [`EmbeddingProvider`] — text → vector, supplied by the host.
//!
//! The memory subsystem embeds chunks, summaries and queries, but it does not
//! decide *how*: which provider, which credentials, which rate limit and which
//! fallback are host policy. So the core takes an `Arc<dyn EmbeddingProvider>`
//! and never constructs one.
//!
//! This trait deliberately lives in the contract crate rather than in
//! `tinymemory-core`, so that a host implementing it does not have to depend on
//! the engine. It carries nothing heavier than `async-trait` and `anyhow`.

use async_trait::async_trait;

/// Formats the canonical embedding-space signature string.
///
/// This is the **single source of truth** for the signature format. Both the
/// live-provider [`EmbeddingProvider::signature`] and any config-derived
/// signature must route through here, so a signature computed from
/// configuration is byte-identical to one computed from an instantiated
/// provider. Drift between the two silently splits one embedding space into
/// two, and every vector written on the wrong side of the split becomes
/// unsearchable without a re-embed.
#[must_use]
pub fn format_embedding_signature(name: &str, model_id: &str, dims: usize) -> String {
    format!(
        "provider={}:{};model={}:{};dims={dims}",
        name.len(),
        name,
        model_id.len(),
        model_id
    )
}

#[cfg(test)]
mod tests {
    use super::format_embedding_signature;

    #[test]
    fn delimiter_characters_cannot_make_distinct_spaces_collide() {
        let first = format_embedding_signature("a;model=b", "c", 3);
        let second = format_embedding_signature("a", "b;model=c", 3);
        assert_ne!(first, second);
    }
}

/// Converts text into numerical vectors.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider name, e.g. `"ollama"`, `"openai"`.
    fn name(&self) -> &str;

    /// Stable model identifier used to generate embeddings.
    fn model_id(&self) -> &str;

    /// Number of dimensions in the generated embeddings.
    fn dimensions(&self) -> usize;

    /// Stable signature for the embedding space.
    ///
    /// Changing any component means existing vectors are no longer comparable
    /// with newly generated ones and must be stored and queried separately
    /// until a migration re-embeds them.
    fn signature(&self) -> String {
        format_embedding_signature(self.name(), self.model_id(), self.dimensions())
    }

    /// Generates embeddings for a batch of strings.
    ///
    /// # Errors
    /// Propagates transport, authentication and quota failures from the
    /// underlying provider.
    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>>;

    /// Generates an embedding for a single string.
    ///
    /// # Errors
    /// As [`Self::embed`], plus an error when the provider returns no vector.
    async fn embed_one(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut results = self.embed(&[text]).await?;
        results
            .pop()
            .ok_or_else(|| anyhow::anyhow!("Empty embedding result"))
    }
}

/// The inert provider bound when semantic search is switched off or no
/// embedding backend is configured. Reports zero dimensions and returns one
/// empty vector per input, so keyword-only retrieval keeps working while
/// vector rerank degrades to a no-op rather than an error.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopEmbedding;

#[async_trait]
impl EmbeddingProvider for NoopEmbedding {
    fn name(&self) -> &str {
        "none"
    }

    fn model_id(&self) -> &str {
        "none"
    }

    fn dimensions(&self) -> usize {
        0
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(vec![Vec::new(); texts.len()])
    }
}
