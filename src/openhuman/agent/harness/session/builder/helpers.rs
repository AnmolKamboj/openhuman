//! Utility helpers used during agent construction.
//!
//! # A note on the store this reaches (#5560)
//!
//! [`tool_memory_store`] is host-side now — see
//! [`memory::tool_memory::store`](crate::openhuman::memory::tool_memory::store)
//! — where it used to be `tinycortex`'s. No call site changed, because the
//! convention it applies (the `tool-<name>` namespace, the `rule/<id>` key, the
//! record's serde) comes from the contract that both ends read.
//!
//! **Do not "modernise" this onto `active_memory_guard().as_tool_memory()`.**
//! `memory` here is the session's own subtree — `DriverMemory::for_subtree`,
//! resolved in `factory` from the profile's `memory_subdir` — so a profile with
//! `dedicatedMemory` prefetches its own rules. The ambient guard is the
//! *shared* tree, and routing this through it would quietly merge every
//! profile's tool rules into one prompt. The correct family call is the one
//! reached through this session's own `binding::for_subtree(..)`, which needs
//! the subtree passed in rather than the memory object.

use crate::openhuman::memory::tool_memory::{tool_memory_store, ToolMemoryRule};
use crate::openhuman::memory::Memory;
use std::sync::Arc;

use crate::openhuman::agent::context::prompt::SystemPromptBuilder;
use crate::openhuman::config::Config;
use crate::openhuman::tools::traits::Tool;
use std::collections::HashSet;

/// (#1400) Best-effort synchronous prefetch of eager tool-scoped rules.
///
/// `from_config_*` is sync but typically runs inside a multi-threaded
/// Tokio runtime (the agent harness path from the channels runtime).
/// We use `block_in_place` + the current runtime handle to call the
/// async store API without restructuring the whole session builder.
///
/// Returns an empty `Vec` (rather than erroring) when:
///   - no Tokio runtime is active (e.g. a sync CLI bootstrap),
///   - the runtime is single-threaded (`block_in_place` would panic),
///   - or the underlying `rules_for_prompt` call returns an error
///     (e.g. the memory backend isn't ready yet).
///
/// Critical / High rules captured later in the session are still
/// available via the `memory_tool_rules_for_prompt` RPC; this prefetch
/// merely seeds the rules that exist at session start.
pub(super) fn prefetch_tool_memory_rules_blocking(
    memory: Arc<dyn Memory>,
    tool_names: &[String],
) -> Vec<ToolMemoryRule> {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return Vec::new();
    };
    if handle.runtime_flavor() != tokio::runtime::RuntimeFlavor::MultiThread {
        return Vec::new();
    }
    let tool_names = tool_names.to_vec();
    tokio::task::block_in_place(|| {
        handle.block_on(async move {
            let store = tool_memory_store(memory);
            match store.rules_for_prompt(&tool_names).await {
                Ok(grouped) => {
                    let mut flat: Vec<_> = grouped.into_values().flatten().collect();
                    flat.sort_by(|a, b| {
                        b.priority
                            .cmp(&a.priority)
                            .then_with(|| a.tool_name.cmp(&b.tool_name))
                            .then_with(|| a.rule.cmp(&b.rule))
                    });
                    flat
                }
                Err(err) => {
                    log::warn!("[memory::tool_memory] prefetch failed: {err}");
                    Vec::new()
                }
            }
        })
    })
}

/// Register the memory prompt sections a session should carry.
///
/// Two independent gates, one per direction:
///
/// - `MemoryAccessSection` (read side, #566): only when `learning.enabled` and
///   a retrieval tool is offered.
/// - `MemoryWriteSection` (write side, #6048): whenever a writing tool is
///   offered, regardless of `learning.enabled`. The tools are registered either
///   way, and a model that holds them with no rule on when to use them narrates
///   a save it never made — "got it, saved" with zero tool calls.
///
/// Each gate needs the tool registered **and** visible after filtering
/// (`any_tool_offered`): a rule about a tool the model cannot see would only
/// teach it to apologise.
pub(super) fn add_memory_prompt_sections(
    prompt_builder: SystemPromptBuilder,
    config: &Config,
    tools: &[Box<dyn Tool>],
    delegation_tools: &[Box<dyn Tool>],
    visible: &HashSet<String>,
    agent_id: &str,
) -> SystemPromptBuilder {
    use crate::openhuman::agent::learning::{
        any_tool_offered, MemoryAccessSection, MemoryWriteSection, MEMORY_READ_TOOLS,
        MEMORY_WRITE_TOOLS,
    };
    let mut prompt_builder = prompt_builder;
    if config.learning.enabled {
        if any_tool_offered(&MEMORY_READ_TOOLS, tools, delegation_tools, visible) {
            prompt_builder = prompt_builder.add_section(Box::new(MemoryAccessSection));
            log::debug!("[learning] memory_access prompt section registered");
        } else {
            log::debug!(
                "[learning] skipping MemoryAccessSection — neither memory_recall nor \
                 memory_search is registered+visible for agent={agent_id}"
            );
        }
    }
    if any_tool_offered(&MEMORY_WRITE_TOOLS, tools, delegation_tools, visible) {
        prompt_builder = prompt_builder.add_section(Box::new(MemoryWriteSection));
        log::debug!("[memory_write] prompt section registered for agent={agent_id}");
    } else {
        log::debug!(
            "[memory_write] skipping MemoryWriteSection — neither memory_store nor \
             save_preference is registered+visible for agent={agent_id}"
        );
    }
    prompt_builder
}
