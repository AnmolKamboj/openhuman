// Fixtures for the retrieval family's scored answers. Included into
// `test_support.rs` after the provider parts, so the imports there are in scope.

/// A [`NamespaceSummary`] saying `namespace` holds `count` entries.
pub fn namespace_summary(namespace: &str, count: usize) -> NamespaceSummary {
    NamespaceSummary {
        namespace: namespace.into(),
        count,
        last_updated: None,
    }
}

/// A [`NamespaceMemoryHit`] with only the vector component set — the signal the
/// vector-floored recall paths (Lane B, the contradiction check) filter on.
pub fn namespace_hit(
    namespace: &str,
    key: &str,
    content: &str,
    vector_similarity: f64,
) -> NamespaceMemoryHit {
    NamespaceMemoryHit {
        id: format!("{namespace}/{key}"),
        kind: crate::openhuman::memory::api::types::MemoryItemKind::Kv,
        namespace: namespace.into(),
        key: key.into(),
        title: None,
        content: content.into(),
        category: "core".into(),
        source_type: None,
        updated_at: 0.0,
        score: vector_similarity,
        score_breakdown: crate::openhuman::memory::api::types::RetrievalScoreBreakdown {
            vector_similarity,
            ..Default::default()
        },
        document_id: None,
        chunk_id: None,
        supporting_relations: Vec::new(),
        taint: MemoryTaint::default(),
    }
}
