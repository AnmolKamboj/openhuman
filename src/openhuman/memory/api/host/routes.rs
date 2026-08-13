//! [`EmbeddingRouteConfig`] — a per-workload embedding provider override.
//!
//! Moved here from the host's `config::schema::routes` because the memory
//! store's factory reads its fields directly when resolving which embedder
//! backs a workload. Inert serde data; **its serde form is persisted** in
//! users' `config.toml`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingRouteConfig {
    pub hint: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub dimensions: Option<usize>,
}
