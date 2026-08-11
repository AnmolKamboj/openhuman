//! [`MemoryRecall`] tests.
//!
//! `recall_with_a_source_scope_is_refused_until_the_predicate_exists` is the
//! one that matters: it pins the deliberate refusal so nobody "fixes" it by
//! quietly dropping the argument, which would answer a scoped query in full.

use super::super::test_support::fresh_driver;
use super::*;

use tinycortex_api::provider::types::SourceScope;
use tinycortex_api::provider::MemoryCore;
use tinycortex_api::types::{MemoryCategory, MemoryTaint};

async fn seed(provider: &EmbeddedMemoryProvider) {
    provider
        .store(
            "ns_a",
            "rust",
            "the rust programming language is memory safe",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store rust");
    provider
        .store(
            "ns_a",
            "sailing",
            "sailing boats need wind",
            MemoryCategory::Core,
            None,
            MemoryTaint::Internal,
        )
        .await
        .expect("store sailing");
}

fn opts_for(namespace: &str) -> OwnedRecallOpts {
    OwnedRecallOpts {
        namespace: Some(namespace.to_string()),
        ..OwnedRecallOpts::default()
    }
}

#[tokio::test]
async fn recall_returns_ranked_results_for_a_query() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider).await;

    let hits = provider
        .recall("rust programming language", 5, &opts_for("ns_a"), None)
        .await
        .expect("recall");

    assert!(!hits.is_empty(), "expected at least one hit");
    let scores: Vec<f64> = hits.iter().map(|h| h.score.unwrap_or(0.0)).collect();
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "results must be most-relevant first: {scores:?}"
    );
    assert_eq!(hits[0].key, "rust");
}

#[tokio::test]
async fn recall_of_an_empty_store_is_empty_not_an_error() {
    let (_tmp, provider) = fresh_driver();
    let hits = provider
        .recall("anything", 5, &opts_for("ns_a"), None)
        .await
        .expect("recall must not error on an empty store");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn recall_honours_the_min_score_filter_from_owned_opts() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider).await;

    let opts = OwnedRecallOpts {
        min_score: Some(1.1),
        ..opts_for("ns_a")
    };
    let hits = provider
        .recall("rust programming language", 5, &opts, None)
        .await
        .expect("recall");
    assert!(
        hits.is_empty(),
        "an unreachable min_score must drop every hit, got {}",
        hits.len()
    );
}

#[tokio::test]
async fn recall_surfaces_external_sync_taint() {
    let (_tmp, provider) = fresh_driver();
    provider
        .store(
            "ns_a",
            "synced",
            "kubernetes cluster autoscaling notes",
            MemoryCategory::Core,
            None,
            MemoryTaint::ExternalSync,
        )
        .await
        .expect("store");

    let hits = provider
        .recall("kubernetes cluster autoscaling", 5, &opts_for("ns_a"), None)
        .await
        .expect("recall");
    let hit = hits.first().expect("expected a hit");
    assert_eq!(hit.taint, MemoryTaint::ExternalSync);
}

#[tokio::test]
async fn recall_with_a_source_scope_is_refused_until_the_predicate_exists() {
    let (_tmp, provider) = fresh_driver();
    seed(&provider).await;

    // The unscoped call still works …
    provider
        .recall("rust", 5, &opts_for("ns_a"), None)
        .await
        .expect("unscoped recall");

    // … while a scoped one is refused loudly rather than answered in full.
    let scope = SourceScope::new(["src-abc"]);
    let error = provider
        .recall("rust", 5, &opts_for("ns_a"), Some(&scope))
        .await
        .expect_err("a scope this driver cannot apply must be refused");
    match error {
        MemoryError::Invalid(reason) => assert_eq!(reason, SCOPE_UNAPPLIED),
        other => panic!("expected MemoryError::Invalid, got {other:?}"),
    }

    // An *empty* scope denies all source-attributed content, so it must be
    // refused too rather than read as "unrestricted".
    let empty = SourceScope::default();
    assert!(provider
        .recall("rust", 5, &opts_for("ns_a"), Some(&empty))
        .await
        .is_err());
}
