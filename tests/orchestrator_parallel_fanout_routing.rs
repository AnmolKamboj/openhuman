//! Pins the orchestrator's parallel fan-out routing (#4754, re-anchored for #5757).
//!
//! An agent-efficiency eval found the orchestrator never fanning out workers
//! concurrently: parallel/"separate researcher for each"/council prompts either
//! single-spawned or issued serial `spawn_subagent` calls 145-200s apart
//! (each sub-agent finishing before the next started), defeating the request.
//!
//! Root cause was routing, not harness concurrency, and it is still routing
//! that these assertions guard. What changed is the primitive being routed to.
//!
//! #5757 (`02d81f6cf`) retired `spawn_parallel_agents` along with five other
//! sub-agent tools, cutting the surface from 11 tools to 3. A dedicated fan-out
//! tool was judged "a second way to say spawn again" now that
//! `spawn_async_subagent` is always async: it returns a task id immediately, so
//! N spawns issued together are already N workers running concurrently. The
//! retirement is pinned both ways in `agents/loader.rs` — three tools required,
//! six asserted absent — so it is deliberate and enforced, not drift.
//!
//! The anti-pattern this file exists to catch is therefore unchanged — a prompt
//! that lets fan-out serialize — but the guidance it anchors on had to move
//! with the tool. Anchoring on the retired name is what left this suite
//! asserting against a prompt that no longer mentions it.

const ORCHESTRATOR_PROMPT: &str =
    include_str!("../src/openhuman/agent/registry/agents/orchestrator/prompt.md");

const SPAWN_SUBAGENT_SRC: &str =
    include_str!("../src/openhuman/agent/orchestration/tools/spawn_subagent.rs");

#[test]
fn prompt_routes_fanout_to_concurrent_async_spawns() {
    let prompt = ORCHESTRATOR_PROMPT.to_lowercase();

    // The fan-out guidance must exist and name the primitive that actually
    // runs concurrently. Post-#5757 that is `spawn_async_subagent`; the
    // assertion is deliberately on the *current* tool rather than on whichever
    // name happened to be right in 2026, because a prompt naming a retired
    // tool is the failure this file is meant to catch.
    assert!(
        ORCHESTRATOR_PROMPT.contains("spawn_async_subagent"),
        "orchestrator prompt must name the concurrent spawn primitive (#4754, #5757)"
    );

    // It must say that fan-out IS several spawns — the sentence that replaced
    // "use one spawn_parallel_agents call". Without it a model reading the
    // prompt has no instruction to issue them together, which is exactly the
    // serialization the eval measured.
    assert!(
        prompt.contains("fan-out is just several") || prompt.contains("n spawns"),
        "orchestrator prompt must state that fan-out is several spawns issued \
         together, not a sequence of dependent ones (#4754, #5757)"
    );

    // And it must state that they run concurrently. "Several spawns" is only
    // the fix if the spawns overlap; a prompt that dropped this could be read
    // as endorsing the 145-200s serial gaps the eval found.
    assert!(
        prompt.contains("run concurrently") || prompt.contains("concurrently"),
        "orchestrator prompt must state that several spawns run concurrently, \
         which is what makes fan-out a fan-out (#4754, #5757)"
    );
}

/// The retired fan-out tool must not come back in the prompt without coming
/// back in `agent.toml` — a prompt teaching a tool the orchestrator cannot call
/// is worse than one that teaches nothing, because the model spends a turn
/// discovering it. `agents/loader.rs` already pins the tool list itself; this
/// pins the half of the contract that lives in prose.
#[test]
fn prompt_does_not_teach_a_retired_subagent_tool() {
    for retired in [
        "spawn_parallel_agents",
        "wait_subagent",
        "steer_subagent",
        "close_subagent",
        "wait_loop",
    ] {
        assert!(
            !ORCHESTRATOR_PROMPT.contains(retired),
            "orchestrator prompt teaches `{retired}`, which #5757 retired from \
             agent.toml — re-adding one means re-adding it in both places"
        );
    }
}

#[test]
fn spawn_subagent_description_redirects_fanout_to_parallel() {
    // The tool description is what the model reads while picking a tool, so the
    // redirect has to live there, not only in the system prompt. Anchor on the
    // description() body to avoid matching an unrelated mention elsewhere.
    let desc_start = SPAWN_SUBAGENT_SRC
        .find("fn description(&self)")
        .expect("spawn_subagent must have a description()");
    let desc = &SPAWN_SUBAGENT_SRC[desc_start..];
    let desc_body = &desc[..desc.find("fn parameters_schema").unwrap_or(desc.len())];
    assert!(
        desc_body.contains("spawn_parallel_agents"),
        "spawn_subagent's description must redirect concurrent fan-out to \
         `spawn_parallel_agents` so the model picks the parallel tool (#4754)"
    );
}
