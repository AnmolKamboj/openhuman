use anyhow::Result;
use async_trait::async_trait;

use crate::openhuman::memory::tree::score::embed::Embedder as HostEmbedder;

pub(super) struct EmbedderBridge<'a>(pub &'a dyn HostEmbedder);

#[async_trait]
impl tinycortex::memory::score::embed::Embedder for EmbedderBridge<'_> {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.0.embed(text).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Vec<Result<Vec<f32>>> {
        self.0.embed_batch(texts).await
    }
}
