//! Tool-pack behaviour.
//!
//! The pair of assertions that matter most are the negative ones: that a packed
//! tool's schema really is withheld, and that `use_skill` cannot be used to
//! reach a tool through the wrong skill or to launder its permission level.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::*;
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolResult};

struct FakeTool {
    name: &'static str,
    level: PermissionLevel,
}

#[async_trait]
impl Tool for FakeTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "fake"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {"marker": {"type": "string"}}})
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::success(format!("{}:{}", self.name, args)))
    }
    fn permission_level(&self) -> PermissionLevel {
        self.level
    }
}

/// A registry holding one real packed tool plus the two pack tools, bound.
fn registry_with(name: &'static str, level: PermissionLevel) -> Arc<Vec<Box<dyn Tool>>> {
    let mut tools: Vec<Box<dyn Tool>> = vec![Box::new(FakeTool { name, level })];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);
    tools
}

fn find<'a>(tools: &'a [Box<dyn Tool>], name: &str) -> &'a dyn Tool {
    tools
        .iter()
        .find(|t| t.name() == name)
        .map(AsRef::as_ref)
        .unwrap_or_else(|| panic!("{name} missing"))
}

#[test]
fn every_packed_name_belongs_to_exactly_one_pack() {
    let mut seen = HashSet::new();
    for name in all_packed_tool_names() {
        assert!(seen.insert(name), "`{name}` is claimed by two packs");
    }
}

#[test]
fn packed_names_are_withheld_and_replaced() {
    let packed = all_packed_tool_names();
    let sample = packed[0];
    let mut visible: HashSet<String> = [sample.to_string(), "file_read".to_string()]
        .into_iter()
        .collect();

    strip_packed_from_visible(&mut visible);

    assert!(!visible.contains(sample), "packed tool stayed advertised");
    assert!(visible.contains("file_read"), "unpacked tool was dropped");
    assert!(visible.contains(LOAD_SKILL) && visible.contains(USE_SKILL));
}

#[test]
fn an_agent_that_lost_nothing_gains_nothing() {
    // A narrow sub-agent must not grow two tools that can only report an empty
    // skill, so the pack tools are added only when something was withheld.
    let mut visible: HashSet<String> = ["file_read".to_string()].into_iter().collect();
    strip_packed_from_visible(&mut visible);
    assert_eq!(visible.len(), 1);
    assert!(!visible.contains(LOAD_SKILL));
}

#[test]
fn an_empty_visible_set_is_left_alone() {
    // Empty is the harness's "everything is visible" sentinel, not "nothing".
    let mut visible: HashSet<String> = HashSet::new();
    strip_packed_from_visible(&mut visible);
    assert!(visible.is_empty());
}

#[tokio::test]
async fn load_skill_renders_the_schema_of_a_present_tool() {
    let name = pack("crypto").unwrap().tools[0];
    let tools = registry_with(name, PermissionLevel::ReadOnly);
    let result = find(&tools, LOAD_SKILL)
        .execute(json!({"skill": "crypto"}))
        .await
        .unwrap();
    assert!(!result.is_error);
    let text = format!("{:?}", result.content);
    assert!(text.contains(name), "rendered pack omitted `{name}`");
    assert!(
        text.contains("marker"),
        "rendered pack omitted the arg schema"
    );
}

#[tokio::test]
async fn load_skill_rejects_an_unknown_skill() {
    let tools = registry_with("do_crypto", PermissionLevel::ReadOnly);
    let result = find(&tools, LOAD_SKILL)
        .execute(json!({"skill": "nope"}))
        .await
        .unwrap();
    assert!(result.is_error);
}

#[tokio::test]
async fn use_skill_dispatches_to_the_packed_tool() {
    let name = pack("crypto").unwrap().tools[0];
    let tools = registry_with(name, PermissionLevel::ReadOnly);
    let result = find(&tools, USE_SKILL)
        .execute(json!({"skill": "crypto", "tool": name, "args": {"marker": "x"}}))
        .await
        .unwrap();
    assert!(!result.is_error);
    let text = format!("{:?}", result.content);
    assert!(
        text.contains("marker"),
        "inner args were not forwarded: {text}"
    );
}

#[tokio::test]
async fn use_skill_refuses_a_tool_from_another_skill() {
    // Cross-skill dispatch would make the `skill` argument decoration and let a
    // workflow skill reach a crypto write.
    let crypto = pack("crypto").unwrap().tools[0];
    let tools = registry_with(crypto, PermissionLevel::Dangerous);
    let result = find(&tools, USE_SKILL)
        .execute(json!({"skill": "workflows", "tool": crypto, "args": {}}))
        .await
        .unwrap();
    assert!(result.is_error, "cross-skill dispatch was admitted");
}

#[test]
fn use_skill_reports_the_inner_tools_permission_level() {
    // The harness gates on this. Reporting the proxy's own level would launder
    // a dangerous packed tool onto a channel that refuses it.
    let name = pack("crypto").unwrap().tools[0];
    let tools = registry_with(name, PermissionLevel::Dangerous);
    let use_skill = find(&tools, USE_SKILL);
    assert_eq!(
        use_skill.permission_level_with_args(&json!({"skill": "crypto", "tool": name})),
        PermissionLevel::Dangerous
    );
}

#[test]
fn an_unresolvable_call_reports_the_ceiling_not_a_permissive_default() {
    let name = pack("crypto").unwrap().tools[0];
    let tools = registry_with(name, PermissionLevel::Dangerous);
    let use_skill = find(&tools, USE_SKILL);
    assert_eq!(
        use_skill.permission_level_with_args(&json!({"skill": "crypto", "tool": "nonexistent"})),
        PermissionLevel::Dangerous
    );
}

#[test]
fn an_unbound_handle_degrades_closed() {
    // A pack tool that never got bound must not become a permissive passthrough.
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    append_pack_tools(&mut tools);
    let use_skill = find(&tools, USE_SKILL);
    assert_eq!(use_skill.permission_level(), PermissionLevel::Dangerous);
}

#[test]
fn every_pack_declares_the_tools_it_is_named_for() {
    // Membership is compiled-in data, so a typo here is invisible until a
    // `load_skill` at runtime renders a pack that withheld nothing. Pin the
    // exact set per pack rather than a count.
    let expect: &[(&str, &[&str])] = &[
        (
            "workflows",
            &[
                "build_workflow",
                "discover_workflows",
                "run_workflow",
                "await_workflow",
                "describe_workflow",
                "list_workflows",
                "list_workflow_runs",
                "read_workflow_run_log",
            ],
        ),
        ("crypto", &["do_crypto", "use_tinyplace"]),
        (
            "integrations",
            &["use_mcp_server", "setup_mcp_server", "mcp_registry_status"],
        ),
        (
            "skills",
            &[
                "run_skill",
                "setup_skills",
                "skill_registry_browse",
                "skill_registry_search",
                "skill_registry_install",
                "skill_registry_sources",
            ],
        ),
        ("goals", &["goal_set", "goal_get", "goal_complete"]),
        ("app_update", &["update_check", "update_apply"]),
    ];

    for (id, tools) in expect {
        let found = registry::pack(id).unwrap_or_else(|| panic!("pack `{id}` is missing"));
        assert_eq!(found.tools, *tools, "pack `{id}` membership drifted");
    }

    assert_eq!(
        registry::PACKS.len(),
        expect.len(),
        "a pack was added without pinning its membership here"
    );
}

#[test]
fn the_reactive_fleet_tools_are_never_packed() {
    // Packing these would put a `load_skill` round-trip between an async
    // worker returning and the parent being able to steer or collect it.
    // See `DELIBERATELY_UNPACKED_FLEET_TOOLS` for the full reasoning.
    for name in registry::DELIBERATELY_UNPACKED_FLEET_TOOLS {
        assert!(
            registry::pack_for_tool(name).is_none(),
            "`{name}` is needed reactively mid-turn and must stay advertised"
        );
    }
}
