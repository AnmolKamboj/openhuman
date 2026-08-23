//! The compiled-in pack table.
//!
//! Membership is a build-time decision, deliberately: a pack that config or RPC
//! could edit would let a caller move a dangerous tool out of the advertised
//! surface (or back into it) without review. Adding a pack is a source change.

use super::types::ToolPack;

/// Every pack this build knows about.
///
/// Chosen by measured schema cost against how often the orchestrator actually
/// needs them: together ~5.9k tokens of the Master Agent's tool-schema budget,
/// idle in the large majority of turns. Measured with tiktoken `o200k_base`
/// against a real `agent dump-all`, not estimated.
///
/// Frequency of use is the whole criterion — see
/// `DELIBERATELY_UNPACKED_FLEET_TOOLS` below for the family that is expensive
/// but must stay advertised.
pub const PACKS: &[ToolPack] = &[
    ToolPack {
        id: "workflows",
        summary: "Build, discover, run and inspect saved automation workflows (flows) and their run logs.",
        tools: &[
            "build_workflow",
            "discover_workflows",
            "run_workflow",
            "await_workflow",
            "describe_workflow",
            "list_workflows",
            "list_workflow_runs",
            "read_workflow_run_log",
        ],
    },
    ToolPack {
        id: "crypto",
        summary: "Crypto wallet and market actions (balances, transfers, swaps, contract calls, exchange trades) and tiny.place agent-network operations.",
        tools: &["do_crypto", "use_tinyplace"],
    },
    ToolPack {
        id: "integrations",
        summary: "MCP server setup, connection status, and calling tools on a connected MCP server.",
        tools: &["use_mcp_server", "setup_mcp_server", "mcp_registry_status"],
    },
    ToolPack {
        id: "skills",
        summary: "Find, install and run agent skills from the community registries.",
        tools: &[
            "run_skill",
            "setup_skills",
            "skill_registry_browse",
            "skill_registry_search",
            "skill_registry_install",
            "skill_registry_sources",
        ],
    },
    ToolPack {
        id: "goals",
        summary: "Read, set and complete the user's long-term goals.",
        tools: &["goal_set", "goal_get", "goal_complete"],
    },
    ToolPack {
        id: "app_update",
        summary: "Check for and apply OpenHuman application updates.",
        tools: &["update_check", "update_apply"],
    },
];

/// The fleet tools are deliberately NOT a pack, and this is worth stating
/// because they look like an obvious 1.6k-token candidate.
///
/// `steer_subagent`, `wait_subagent`, `close_subagent`, `list_subagents`,
/// `continue_subagent`, `wait`, `wait_loop` and `spawn_parallel_agents` are
/// needed *reactively*, mid-turn — exactly when an async worker returns or
/// pauses on `ask_user_clarification`. A load round-trip at that moment is the
/// worst possible time to add one, and a `continue_subagent` the model cannot
/// see is the known infinite-re-delegation failure mode (#4291): the only
/// continuation left is a fresh stateless sub-agent that asks the same
/// question again.
#[cfg(test)]
pub(crate) const DELIBERATELY_UNPACKED_FLEET_TOOLS: &[&str] = &[
    "steer_subagent",
    "wait_subagent",
    "close_subagent",
    "list_subagents",
    "continue_subagent",
    "wait",
    "wait_loop",
    "spawn_parallel_agents",
];

pub fn pack(id: &str) -> Option<&'static ToolPack> {
    PACKS.iter().find(|p| p.id == id)
}

/// The pack owning `tool`, if any.
pub fn pack_for_tool(tool: &str) -> Option<&'static ToolPack> {
    PACKS.iter().find(|p| p.owns(tool))
}

/// Every packed tool name across all packs.
pub fn all_packed_tool_names() -> Vec<&'static str> {
    PACKS.iter().flat_map(|p| p.tools.iter().copied()).collect()
}

/// The always-on index: one line per pack, rendered into `load_skill`'s own
/// description so the model can pick a pack without a round trip.
pub fn pack_index_markdown() -> String {
    let mut out = String::new();
    for p in PACKS {
        out.push_str(&format!("- `{}` — {}\n", p.id, p.summary));
    }
    out
}
