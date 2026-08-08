//! [`MemoryEntities`] tests for the embedded driver.

use super::super::test_support::fresh_driver;
use super::CO_OCCURRENCE_PREDICATE;

use chrono::Utc;
use tinycortex_api::provider::MemoryEntities;

use crate::openhuman::config::Config;
use crate::openhuman::memory::store::entities::index_entity;
use crate::openhuman::memory::store::trees::hotness;
use crate::openhuman::memory::tree::graph::store as graph_store;
use tinycortex::memory::store::entity_index::{CanonicalEntity, EntityKind};

/// Seed one occurrence into the entity index through the host's own writer.
///
/// Deliberately not raw SQL: `mem_tree_entity_index` is engine-owned and its
/// column set has already moved once, so a hand-written INSERT here would rot
/// against a schema change instead of following it.
fn seed_entity(config: &Config, entity_id: &str, kind: EntityKind, surface: &str, node_id: &str) {
    index_entity(
        config,
        &CanonicalEntity {
            canonical_id: entity_id.to_string(),
            kind,
            surface: surface.to_string(),
            span_start: 0,
            span_end: u32::try_from(surface.len()).unwrap_or(1),
            score: 1.0,
        },
        node_id,
        "chunk",
        Utc::now().timestamp_millis(),
        None,
    )
    .expect("seed entity index row");
}

#[tokio::test]
async fn entities_unknown_workspace_yields_empty() {
    let (_tmp, provider) = fresh_driver();
    let hits = provider.entities("work", None, 10).await.expect("entities");
    assert!(hits.is_empty(), "an empty index ranks nothing");
}

#[tokio::test]
async fn entities_ranked_by_mentions_when_query_absent() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_entity(&config, "topic:phoenix", EntityKind::Topic, "Phoenix", "n1");
    seed_entity(&config, "topic:phoenix", EntityKind::Topic, "Phoenix", "n2");
    seed_entity(&config, "topic:atlas", EntityKind::Topic, "Atlas", "n3");

    let hits = provider.entities("work", None, 10).await.expect("entities");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].entity.id, "topic:phoenix", "most mentions first");
    assert_eq!(hits[0].mentions, 2);
    assert_eq!(hits[0].entity.kind, "topic");
    assert_eq!(hits[0].entity.name, "Phoenix");
}

#[tokio::test]
async fn entities_ranked_by_match_when_query_present() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_entity(&config, "topic:phoenix", EntityKind::Topic, "Phoenix", "n1");
    seed_entity(&config, "topic:atlas", EntityKind::Topic, "Atlas", "n2");

    let hits = provider
        .entities("work", Some("Phoenix"), 10)
        .await
        .expect("entities");
    assert!(
        hits.iter().any(|hit| hit.entity.id == "topic:phoenix"),
        "the matching entity must be returned, got {:?}",
        hits.iter().map(|h| &h.entity.id).collect::<Vec<_>>()
    );
    assert!(
        !hits.iter().any(|hit| hit.entity.id == "topic:atlas"),
        "a non-matching entity must not be"
    );
}

#[tokio::test]
async fn entities_hotness_reflects_the_hotness_table() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    seed_entity(&config, "topic:phoenix", EntityKind::Topic, "Phoenix", "n1");

    let cold = provider.entities("work", None, 10).await.expect("entities");
    assert_eq!(
        cold[0].hotness, 0.0,
        "an entity with no hotness row scores zero"
    );

    provider
        .touch_entities("work", &["topic:phoenix".to_string()])
        .await
        .expect("touch_entities");

    let warm = provider.entities("work", None, 10).await.expect("entities");
    assert!(
        warm[0].hotness > 0.0,
        "touching the entity must raise its hotness, got {}",
        warm[0].hotness
    );
}

#[tokio::test]
async fn touch_entities_bumps_hotness_counters() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();

    provider
        .touch_entities("work", &["topic:phoenix".to_string()])
        .await
        .expect("first touch");
    provider
        .touch_entities("work", &["topic:phoenix".to_string()])
        .await
        .expect("second touch");

    let counters = hotness::get(&config, "topic:phoenix")
        .expect("hotness read")
        .expect("row exists after touching");
    assert_eq!(counters.mention_count_30d, 2);
    assert!(counters.last_seen_ms.is_some());
}

#[tokio::test]
async fn touch_entities_accepts_unknown_ids_rather_than_rejecting_them() {
    let (_tmp, provider) = fresh_driver();
    provider
        .touch_entities("work", &["entity:never-seen".to_string()])
        .await
        .expect("the contract says unknown ids are ignored, not rejected");
    provider
        .touch_entities("work", &[])
        .await
        .expect("an empty list is a no-op");
}

#[tokio::test]
async fn entity_edges_projects_cooccurrence_neighbours() {
    let (_tmp, provider) = fresh_driver();
    let config = provider.config().await.expect("config").clone();
    graph_store::upsert_edges(
        &config,
        &[("topic:phoenix".to_string(), "person:alice".to_string())],
        Utc::now().timestamp_millis(),
    )
    .expect("seed co-occurrence edge");

    let edges = provider
        .entity_edges("work", "topic:phoenix", 10)
        .await
        .expect("entity_edges");

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].subject, "topic:phoenix");
    assert_eq!(edges[0].object, "person:alice");
    assert_eq!(
        edges[0].predicate, CO_OCCURRENCE_PREDICATE,
        "the projection materialises one fixed predicate"
    );
    assert!(
        edges[0].evidence_count >= 1,
        "the co-occurrence weight is the evidence count"
    );
    assert!(edges[0].document_ids.is_empty());
    assert!(edges[0].chunk_ids.is_empty());
}

#[tokio::test]
async fn entity_edges_unknown_entity_is_empty_not_not_found() {
    let (_tmp, provider) = fresh_driver();
    let edges = provider
        .entity_edges("work", "topic:never-seen", 10)
        .await
        .expect("'no edges' and 'no such entity' are the same answer");
    assert!(edges.is_empty());
}
