//! NVIDIA NIM embeddings (OpenAI-compatible `/v1/embeddings` plus NIM extras).
//!
//! Retrieval models on `integrate.api.nvidia.com` need `input_type` (`query` vs
//! `passage`). The stock OpenAI embedder never sends it, so recall quality
//! collapses even when the HTTP call succeeds. `nvidia/nemotron-3-embed-1b`
//! is also native-2048-only: requesting 1024 returns HTTP 400, and the memory
//! tree hard-requires 1024 (`EMBEDDING_DIM`). This client:
//!
//! - always sends `input_type` (inferred from batch length, or from a
//!   `-query` / `-passage` model suffix);
//! - only sends `dimensions` for models that actually honour Matryoshka;
//! - refuses to silently truncate non-MRL 2048 vectors down to 1024.

use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use super::EmbeddingProvider;

pub(crate) fn is_nvidia_endpoint(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("nvidia.com") || lower.contains("nvcf")
}

/// Strip a NIM `-query` / `-passage` suffix. The remaining id is what the
/// catalog expects in the `model` field when `input_type` is sent separately.
pub(crate) fn split_nvidia_model(model: &str) -> (String, Option<&'static str>) {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(base) = lower.strip_suffix("-query") {
        (trimmed[..base.len()].to_string(), Some("query"))
    } else if let Some(base) = lower.strip_suffix("-passage") {
        (trimmed[..base.len()].to_string(), Some("passage"))
    } else {
        (trimmed.to_string(), None)
    }
}

/// Short batches are recall queries; longer text is vault/index passages.
pub(crate) fn infer_input_type(texts: &[&str]) -> &'static str {
    const QUERY_MAX_CHARS: usize = 256;
    if !texts.is_empty() && texts.iter().all(|t| t.chars().count() <= QUERY_MAX_CHARS) {
        "query"
    } else {
        "passage"
    }
}

pub(crate) struct NvidiaNimEmbedding {
    client: reqwest::Client,
    embeddings_url: String,
    api_key: String,
    /// Wire model id (suffix already stripped).
    model: String,
    /// Original configured id, for `model_id()` / signature.
    configured_model: String,
    dims: usize,
    send_dimensions: bool,
    /// Forced `input_type` from a `-query`/`-passage` suffix.
    pinned_input_type: Option<&'static str>,
}

impl NvidiaNimEmbedding {
    pub(crate) fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
        dims: usize,
        send_dimensions: bool,
    ) -> Self {
        let base = base_url.trim().trim_end_matches('/');
        let embeddings_url = if base.ends_with("/embeddings") {
            base.to_string()
        } else {
            format!("{base}/embeddings")
        };
        let (wire_model, pinned_input_type) = split_nvidia_model(model);
        Self {
            client: reqwest::Client::new(),
            embeddings_url,
            api_key: api_key.to_string(),
            model: wire_model,
            configured_model: model.to_string(),
            dims,
            send_dimensions,
            pinned_input_type,
        }
    }
}

fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn align_vector(
    model: &str,
    allow_mrl_truncate: bool,
    mut values: Vec<f32>,
    target: usize,
) -> anyhow::Result<Vec<f32>> {
    if target == 0 || values.len() == target {
        return Ok(values);
    }
    if values.len() > target && allow_mrl_truncate {
        values.truncate(target);
        l2_normalize(&mut values);
        return Ok(values);
    }
    anyhow::bail!(
        "NVIDIA embedding for `{model}` returned {} dims; memory tree needs {target}. \
         nvidia/nemotron-3-embed-1b is 2048-only (requesting 1024 is HTTP 400). \
         Use nvidia/nv-embedqa-e5-v5 (native 1024) or nvidia/llama-nemotron-embed-1b-v2 \
         with dimensions=1024.",
        values.len()
    )
}

#[async_trait]
impl EmbeddingProvider for NvidiaNimEmbedding {
    fn name(&self) -> &str {
        "nvidia"
    }

    fn model_id(&self) -> &str {
        &self.configured_model
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let egress = crate::openhuman::security::egress::EgressDescriptor::embedding(
            "nvidia",
            &self.configured_model,
        );
        crate::openhuman::security::egress::enforce_egress(&egress)?;
        crate::openhuman::security::egress::emit_external_transfer(egress);

        let input_type = self
            .pinned_input_type
            .unwrap_or_else(|| infer_input_type(texts));

        let mut body = serde_json::json!({
            "input": texts,
            "model": self.model,
            "encoding_format": "float",
            "input_type": input_type,
        });
        if self.send_dimensions && self.dims > 0 {
            body["dimensions"] = serde_json::json!(self.dims);
        }

        crate::openhuman::inference::embeddings::rate_limit::acquire_embedding_slot(
            &self.embeddings_url,
        )
        .await;

        let mut request = self
            .client
            .post(&self.embeddings_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body);
        if !self.api_key.trim().is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", self.api_key.trim()));
        }

        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            let detail = String::from_utf8_lossy(&bytes);
            anyhow::bail!(
                "NVIDIA embeddings HTTP {status} from {}: {detail}",
                self.embeddings_url
            );
        }

        let parsed: serde_json::Value = serde_json::from_slice(&bytes)?;
        let data = parsed
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("NVIDIA embeddings response missing data[]"))?;

        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("NVIDIA embeddings item missing embedding"))?;
            let values: Vec<f32> = embedding
                .iter()
                .map(|n| n.as_f64().unwrap_or(0.0) as f32)
                .collect();
            vectors.push(align_vector(
                &self.model,
                self.send_dimensions,
                values,
                self.dims,
            )?);
        }
        Ok(vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nvidia_hosts() {
        assert!(is_nvidia_endpoint("https://integrate.api.nvidia.com/v1"));
        assert!(is_nvidia_endpoint("https://api.nvcf.nvidia.com/v2"));
        assert!(!is_nvidia_endpoint("https://api.openai.com/v1"));
        assert!(!is_nvidia_endpoint("http://127.0.0.1:11434/v1"));
    }

    #[test]
    fn splits_query_passage_suffix() {
        assert_eq!(
            split_nvidia_model("nvidia/nv-embedqa-e5-v5-query"),
            ("nvidia/nv-embedqa-e5-v5".into(), Some("query"))
        );
        assert_eq!(
            split_nvidia_model("nvidia/nv-embedqa-e5-v5-passage"),
            ("nvidia/nv-embedqa-e5-v5".into(), Some("passage"))
        );
        assert_eq!(
            split_nvidia_model("nvidia/nv-embedqa-e5-v5"),
            ("nvidia/nv-embedqa-e5-v5".into(), None)
        );
    }

    #[test]
    fn infers_query_vs_passage_from_length() {
        assert_eq!(infer_input_type(&["what do you know about me?"]), "query");
        let passage = "x".repeat(300);
        assert_eq!(infer_input_type(&[&passage]), "passage");
        assert_eq!(infer_input_type(&[]), "passage");
    }

    #[test]
    fn align_refuses_non_mrl_truncate() {
        let err = align_vector(
            "nvidia/nemotron-3-embed-1b",
            false,
            vec![0.1; 2048],
            1024,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("nv-embedqa-e5-v5"),
            "got: {err}"
        );
    }

    #[test]
    fn align_truncates_mrl_models() {
        let mut source = vec![0.0_f32; 2048];
        source[0] = 3.0;
        source[1] = 4.0;
        let out = align_vector("nvidia/llama-nemotron-embed-1b-v2", true, source, 1024).unwrap();
        assert_eq!(out.len(), 1024);
        let norm = (out[0] * out[0] + out[1] * out[1]).sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "expected L2-normalised slice, got {norm}");
    }
}
