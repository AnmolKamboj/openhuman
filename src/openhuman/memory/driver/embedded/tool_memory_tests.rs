//! [`MemoryToolMemory`] tests.
//!
//! Two carry weight beyond a round-trip:
//!
//! - `put_tool_rule_with_a_blank_tool_name_is_invalid_not_other` pins the error
//!   classification. `ToolMemoryStore::put_rule` rejects a blank name before
//!   touching storage; collapsing that into `Other` would tell a caller their
//!   backend is broken when their input is.
//! - `put_tool_rule_through_the_contract_is_visible_to_an_independent_reader`
//!   is the same-store proof: a second client built over the same workspace
//!   sees the rule, so the contract write reached the workspace's real store
//!   rather than something private to the driver's handle.

use super::super::test_support::fresh_driver;
use super::*;

use crate::openhuman::config::schema::MemoryHooksConfig;
use crate::openhuman::memory::driver::embedded::EmbeddedMemoryProvider;
use crate::openhuman::memory::tool_memory::{ToolMemoryPriority, ToolMemorySource};

fn rule(tool: &str, body: &str, priority: ToolMemoryPriority) -> ToolMemoryRule {
    ToolMemoryRule::new(tool, body, priority, ToolMemorySource::Programmatic)
}

#[tokio::test]
async fn put_tool_rule_then_tool_rules_returns_it() {
    let (_tmp, provider) = fresh_driver();

    let stored = rule("shell", "never rm -rf /", ToolMemoryPriority::Critical);
    provider
        .put_tool_rule(stored.clone())
        .await
        .expect("put_tool_rule");

    let rules = provider.tool_rules("shell").await.expect("tool_rules");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, stored.id);
    assert_eq!(rules[0].rule, "never rm -rf /");
    assert_eq!(rules[0].priority, ToolMemoryPriority::Critical);
}

#[tokio::test]
async fn tool_rules_is_empty_for_a_tool_with_no_rules() {
    let (_tmp, provider) = fresh_driver();
    assert!(provider
        .tool_rules("never-used")
        .await
        .expect("tool_rules")
        .is_empty());
}

#[tokio::test]
async fn tool_rules_orders_critical_before_high_before_normal() {
    let (_tmp, provider) = fresh_driver();

    for (body, priority) in [
        ("normal one", ToolMemoryPriority::Normal),
        ("critical one", ToolMemoryPriority::Critical),
        ("high one", ToolMemoryPriority::High),
    ] {
        provider
            .put_tool_rule(rule("email", body, priority))
            .await
            .expect("put_tool_rule");
    }

    let rules = provider.tool_rules("email").await.expect("tool_rules");
    let priorities: Vec<ToolMemoryPriority> = rules.iter().map(|r| r.priority).collect();
    assert_eq!(
        priorities,
        vec![
            ToolMemoryPriority::Critical,
            ToolMemoryPriority::High,
            ToolMemoryPriority::Normal
        ],
        "the contract says highest priority first"
    );
}

#[tokio::test]
async fn put_tool_rule_upserts_on_the_same_id() {
    let (_tmp, provider) = fresh_driver();

    let mut existing = rule("shell", "first body", ToolMemoryPriority::Normal);
    provider
        .put_tool_rule(existing.clone())
        .await
        .expect("first put");
    existing.rule = "second body".to_string();
    provider
        .put_tool_rule(existing.clone())
        .await
        .expect("second put");

    let rules = provider.tool_rules("shell").await.expect("tool_rules");
    assert_eq!(rules.len(), 1, "same id must upsert, not duplicate");
    assert_eq!(rules[0].rule, "second body");
}

#[tokio::test]
async fn put_tool_rule_with_a_blank_tool_name_is_invalid_not_other() {
    let (_tmp, provider) = fresh_driver();

    let mut blank = rule("shell", "some body", ToolMemoryPriority::Normal);
    blank.tool_name = "   ".to_string();

    let error = provider
        .put_tool_rule(blank)
        .await
        .expect_err("a blank tool name must be rejected");
    assert!(
        matches!(error, MemoryError::Invalid(_)),
        "caller error must not be reported as a backend failure: {error:?}"
    );
}

#[tokio::test]
async fn put_tool_rule_with_a_blank_body_is_invalid_not_other() {
    let (_tmp, provider) = fresh_driver();

    let mut blank = rule("shell", "placeholder", ToolMemoryPriority::Normal);
    blank.rule = "  ".to_string();

    let error = provider
        .put_tool_rule(blank)
        .await
        .expect_err("a blank rule body must be rejected");
    assert!(matches!(error, MemoryError::Invalid(_)), "{error:?}");
}

#[tokio::test]
async fn delete_tool_rule_reports_existence_then_is_idempotent() {
    let (_tmp, provider) = fresh_driver();

    let stored = rule("shell", "a rule", ToolMemoryPriority::Normal);
    provider
        .put_tool_rule(stored.clone())
        .await
        .expect("put_tool_rule");

    assert!(
        provider
            .delete_tool_rule("shell", &stored.id)
            .await
            .expect("first delete"),
        "the first delete must report that the rule existed"
    );
    assert!(
        !provider
            .delete_tool_rule("shell", &stored.id)
            .await
            .expect("second delete"),
        "deleting twice is a successful no-op, not an error"
    );
    assert!(provider
        .tool_rules("shell")
        .await
        .expect("tool_rules")
        .is_empty());
}

#[tokio::test]
async fn put_tool_rule_through_the_contract_is_visible_to_an_independent_reader() {
    use crate::openhuman::memory::store::MemoryClient;
    use crate::openhuman::memory::tool_memory::tool_memory_store;

    let tmp = tempfile::TempDir::new().expect("temp workspace");
    let workspace = tmp.path().join("ws");
    let provider = EmbeddedMemoryProvider::new(workspace.clone(), MemoryHooksConfig::default());

    let stored = rule(
        "driver_store_proof",
        "reachable from a second handle",
        ToolMemoryPriority::High,
    );
    provider
        .put_tool_rule(stored.clone())
        .await
        .expect("put_tool_rule");

    // A second, independently constructed client over the same workspace —
    // exactly how `memory::ops::tool_memory::open_store` builds its store, but
    // without the process-global slot. The RPC handler itself resolves that
    // global, which any concurrently running test may rebind to its own
    // workspace mid-body; dialling it here would make this proof flaky for a
    // reason that has nothing to do with the driver.
    let independent = MemoryClient::from_workspace_dir(workspace).expect("second client");
    let rules = tool_memory_store(independent.memory_handle())
        .list_rules("driver_store_proof")
        .await
        .expect("list_rules");

    assert!(
        rules.iter().any(|r| r.id == stored.id),
        "contract write must land in the workspace store every other reader sees: {rules:?}"
    );
}
