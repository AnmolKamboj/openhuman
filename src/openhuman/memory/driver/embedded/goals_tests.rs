//! [`MemoryGoals`] tests.
//!
//! `goals_doc_is_the_contract_type` is the one that outlives the round-trip: the
//! whole family is conversion-free only because the engine re-exports the
//! contract's `GoalsDoc`. If that ever forks, this file should say so.

use super::super::test_support::fresh_driver;
use super::*;

use tinycortex_api::goals::GoalItem;

#[test]
fn goals_doc_is_the_contract_type() {
    // Not a tautology: `store::load` is typed on
    // `tinycortex::memory::goals::types::GoalsDoc`, and this assignment only
    // compiles while that is a re-export of the contract type.
    let engine: tinycortex::memory::goals::types::GoalsDoc = Default::default();
    let _contract: GoalsDoc = engine;
}

#[tokio::test]
async fn goals_on_a_fresh_workspace_is_empty_not_not_found() {
    let (_tmp, provider) = fresh_driver();
    let doc = provider.goals().await.expect("goals must not be NotFound");
    assert!(doc.items.is_empty());
}

#[tokio::test]
async fn set_goals_then_goals_round_trips() {
    let (_tmp, provider) = fresh_driver();

    let doc = GoalsDoc {
        items: vec![
            GoalItem::new("g1", "ship the memory contract"),
            GoalItem::new("g2", "keep the build green"),
        ],
    };
    provider.set_goals(doc.clone()).await.expect("set_goals");

    let read_back = provider.goals().await.expect("goals");
    assert_eq!(read_back, doc);
}

#[tokio::test]
async fn set_goals_replaces_wholesale_rather_than_merging() {
    let (_tmp, provider) = fresh_driver();

    provider
        .set_goals(GoalsDoc {
            items: vec![GoalItem::new("g1", "first")],
        })
        .await
        .expect("first set_goals");
    provider
        .set_goals(GoalsDoc {
            items: vec![GoalItem::new("g2", "second")],
        })
        .await
        .expect("second set_goals");

    let read_back = provider.goals().await.expect("goals");
    assert_eq!(read_back.items.len(), 1, "whole-document replacement");
    assert_eq!(read_back.items[0].text, "second");
}

#[tokio::test]
async fn set_goals_writes_the_host_file_the_rest_of_the_product_reads() {
    let (_tmp, provider) = fresh_driver();
    provider
        .set_goals(GoalsDoc {
            items: vec![GoalItem::new("g1", "visible to the host")],
        })
        .await
        .expect("set_goals");

    // Same-store proof: the contract write must land where the existing RPC /
    // agent-tool readers look, not in a parallel file.
    let path = store::goals_path(provider.workspace_dir());
    let body = std::fs::read_to_string(&path).expect("MEMORY_GOALS.md exists");
    assert!(body.contains("visible to the host"), "got: {body}");
}
