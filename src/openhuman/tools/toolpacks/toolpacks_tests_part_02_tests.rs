//! Tool-pack advertisement: the index the model reads, and the disclosure /
//! execution distinction the whole pack mechanism rests on.

use super::*;

/// **The contract this fix must not break** (AGENTS.md: `Withheld` = "Registered
/// and callable: yes"). A withheld packed tool is hidden from the prompt and
/// still callable through `use_skill` — that is the entire point of a pack.
///
/// `ToolPolicySession` records that state as `HideFromPrompt`, and
/// `ToolPolicyDecision::is_denied()` is `!matches!(action, Allow)` — so it
/// answers **true** for a perfectly callable packed tool. Any "can this session
/// call it" predicate built on `is_denied()` therefore eats the whole pack.
#[test]
fn a_withheld_packed_tool_is_hidden_but_not_denied() {
    use crate::openhuman::tools::agent_policy::{ToolPolicyAction, ToolPolicyEngine};

    let tools = registry_with_all(&["goal_set", "build_workflow"]);
    // The real shape: the harness seeds `visible` with everything, then
    // `strip_packed_from_visible` removes the packed names for a non-owner.
    let mut visible: HashSet<String> = tools.iter().map(|t| t.name().to_string()).collect();
    strip_packed_from_visible(&mut visible, "orchestrator");
    assert!(
        !visible.contains("goal_set"),
        "precondition: the packed tool is withheld from the prompt"
    );

    let session = ToolPolicyEngine::build_session(
        "orchestrator",
        "web_chat",
        "chat",
        &Default::default(),
        &tools,
        &visible,
    );

    let decision = session.decision_for("goal_set");
    assert_eq!(
        decision.action,
        ToolPolicyAction::HideFromPrompt,
        "a withheld packed tool is classified as prompt-hidden, not denied"
    );
    assert!(
        decision.is_denied(),
        "…and `is_denied()` nevertheless answers true for it — this is the trap"
    );
}

/// **The permission ceiling survives the disclosure exemption.**
///
/// `build_session_from_refs` tests `explicitly_hidden` before
/// `exceeds_permission`, so a tool that is both hidden and over the ceiling is
/// recorded as `HideFromPrompt` and never gets its permission verdict.
/// `is_denied()` masked that by blocking every hidden tool. A predicate that
/// deliberately admits hidden tools must carry the ceiling itself, or `use_skill`
/// becomes a laundering route into a tool the channel would refuse.
#[test]
fn a_hidden_tool_over_the_permission_ceiling_still_blocks_execution() {
    use crate::openhuman::tools::agent_policy::{ToolPolicyAction, ToolPolicyDecision};

    let over = ToolPolicyDecision {
        tool_name: "dangerous_packed_tool".to_string(),
        action: ToolPolicyAction::HideFromPrompt,
        required_permission: Some(PermissionLevel::Dangerous),
        allowed_permission: PermissionLevel::ReadOnly,
    };
    assert!(
        over.blocks_execution(),
        "a hidden tool above the session's ceiling must not be executable"
    );

    let within = ToolPolicyDecision {
        tool_name: "ordinary_packed_tool".to_string(),
        action: ToolPolicyAction::HideFromPrompt,
        required_permission: Some(PermissionLevel::ReadOnly),
        allowed_permission: PermissionLevel::Dangerous,
    };
    assert!(
        !within.blocks_execution(),
        "a hidden tool within the ceiling is the normal packed case and must stay callable"
    );
}
