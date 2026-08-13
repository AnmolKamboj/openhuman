//! Prompt section that injects tool-scoped memory rules into the system
//! prompt — thin host shim over `tinycortex::memory::tool_memory::render` (W7).
//!
//! ## Why a prompt section
//!
//! Mid-session compression rewrites the rolling chat buffer but never the
//! system prompt — that prompt is frozen for the whole session by design (so the
//! inference backend's prefix cache stays warm; see
//! [`crate::openhuman::agent::prompts::SystemPromptBuilder::build`]). Anything we
//! want to be **compression-resistant** therefore has to live in the system
//! prompt — exactly where Critical and High priority [`ToolMemoryRule`]s belong.
//!
//! ## What this shim owns
//!
//! The rendering (`render_tool_memory_rules`) and the section type
//! ([`ToolMemoryRulesSection`], a byte-stable at-construction snapshot) are the
//! crate's and are re-exported here. Host-retained: the [`PromptSection`] impl
//! that plugs the crate section into the host system-prompt builder — a host
//! trait we can implement for the crate type under the orphan rule.
//!
//! [`ToolMemoryRule`]: super::types::ToolMemoryRule

use anyhow::Result;

use crate::openhuman::agent::context::prompt::{PromptContext, PromptSection};

use crate::openhuman::memory::api::tool_memory::{ToolMemoryPriority, ToolMemoryRule};

pub const TOOL_MEMORY_HEADING: &str = "## Tool-scoped rules";
pub struct ToolMemoryRulesSection {
    rendered: String,
}
impl ToolMemoryRulesSection {
    pub fn new<T: serde::Serialize>(rules: Vec<T>) -> Self {
        let rules: Vec<ToolMemoryRule> = rules
            .into_iter()
            .filter_map(|rule| {
                serde_json::to_value(rule)
                    .ok()
                    .and_then(|value| serde_json::from_value(value).ok())
            })
            .collect();
        Self {
            rendered: render_tool_memory_rules(&rules),
        }
    }
    pub fn empty() -> Self {
        Self {
            rendered: String::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.rendered.trim().is_empty()
    }
    pub fn rendered(&self) -> &str {
        &self.rendered
    }
}
pub fn render_tool_memory_rules(rules: &[ToolMemoryRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }
    let mut sorted: Vec<_> = rules.iter().collect();
    sorted.sort_by(|a, b| {
        a.tool_name
            .cmp(&b.tool_name)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| a.rule.cmp(&b.rule))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut out = format!("{TOOL_MEMORY_HEADING}\n\nThese rules are pinned by the user or by the safety pipeline. Treat every entry as a hard constraint when considering the matching tool — do not override them silently. Lower-priority guidance lives in the `tool-{{name}}` memory namespace and can be queried via `memory_recall` if needed.\n\n");
    let mut current = None;
    for rule in sorted {
        if current != Some(rule.tool_name.as_str()) {
            if current.is_some() {
                out.push('\n');
            }
            out.push_str(&format!(
                "### `{}`\n",
                prompt_line(&rule.tool_name).replace('`', "'")
            ));
            current = Some(rule.tool_name.as_str());
        }
        let priority = match rule.priority {
            ToolMemoryPriority::Critical => "**[critical]**",
            ToolMemoryPriority::High => "**[high]**",
            ToolMemoryPriority::Normal => "**[normal]**",
        };
        out.push_str(&format!("- {priority} {}\n", prompt_line(&rule.rule)));
    }
    out
}
fn prompt_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl PromptSection for ToolMemoryRulesSection {
    fn name(&self) -> &str {
        "tool_memory_rules"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        // build() must not depend on PromptContext fields — it returns the
        // at-construction snapshot verbatim so the inference prefix cache stays warm.
        Ok(self.rendered().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::agent::prompts::types::{
        LearnedContextData, PromptContext, ToolCallFormat,
    };
    use crate::openhuman::memory::tool_memory::{
        ToolMemoryPriority, ToolMemoryRule, ToolMemorySource,
    };

    fn rule(tool: &str, body: &str, priority: ToolMemoryPriority) -> ToolMemoryRule {
        ToolMemoryRule {
            id: format!("{tool}/{body}"),
            tool_name: tool.into(),
            rule: body.into(),
            priority,
            source: ToolMemorySource::UserExplicit,
            tags: vec![],
            created_at: "2026-05-11T00:00:00Z".into(),
            updated_at: "2026-05-11T00:00:00Z".into(),
        }
    }

    #[test]
    fn section_empty_returns_blank_build_output() {
        let section = ToolMemoryRulesSection::empty();
        assert!(section.is_empty());
    }

    #[test]
    fn section_renders_via_prompt_section_trait() {
        // Exercise the host PromptSection glue over the crate section: build()
        // returns the at-construction snapshot regardless of PromptContext.
        let section = ToolMemoryRulesSection::new(vec![rule(
            "email",
            "never email Sarah",
            ToolMemoryPriority::Critical,
        )]);
        assert!(!section.is_empty());
        let visible = std::collections::HashSet::new();
        let ctx = PromptContext {
            workspace_dir: std::path::Path::new("."),
            model_name: "test",
            agent_id: "test",
            tools: &[],
            workflows: &[],
            dispatcher_instructions: "",
            learned: LearnedContextData::default(),
            visible_tool_names: &visible,
            tool_call_format: ToolCallFormat::PFormat,
            connected_integrations: &[],
            connected_identities_md: String::new(),
            include_profile: false,
            include_memory_md: false,
            curated_snapshot: None,
            user_identity: None,
            personality_soul_md: None,
            personality_memory_md: None,
            personality_roster: vec![],
            agents_md_global: None,
            agents_md_local: None,
        };
        let built = section.build(&ctx).unwrap();
        assert!(built.contains("never email Sarah"));
    }
}
