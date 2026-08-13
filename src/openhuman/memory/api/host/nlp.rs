//! spaCy extraction results — the wire shape of the host's Python NLP server.
//!
//! Moved here from the host's `runtime::python_server::spacy` because the
//! summary tree's query-entity extractor consumes them directly, canonicalising
//! each entity into the same `<kind>:<value>` namespace the indexed chunks use.
//! Inert serde data.
//!
//! Provisioning the runtime (`ensure_spacy`, `spacy_provisioned`, the model id)
//! deliberately stayed in the host: downloading and launching a Python server
//! is not something a memory engine should do.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacyEntity {
    pub text: String,
    pub label: String,
    #[serde(default)]
    pub start: u32,
    #[serde(default)]
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacyResponse {
    #[serde(default)]
    pub entities: Vec<SpacyEntity>,
    #[serde(default)]
    pub nouns: Vec<String>,
}
