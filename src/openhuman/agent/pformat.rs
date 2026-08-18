//! P-format tool calls — OpenHuman's adapter over
//! [`tinyagents::harness::tool_calling::pformat`].
//!
//! The format itself — the positional `name[arg|arg]` grammar, the schema-driven
//! argument reconstruction, the type coercion, the escape handling — lives in the
//! crate. What stays here is the part that speaks OpenHuman's own tool
//! vocabulary.
//!
//! # Why the crate takes schemas and this takes tools
//!
//! `build_registry` upstream takes `(name, schema)` pairs rather than a tool
//! trait object. A host's tool type is its own vocabulary, and a crate that
//! depended on it could not be used by a second host — which is the whole point
//! of the seam. So the two functions below are the adapter: they read
//! [`Tool::name`] and [`Tool::parameters_schema`] and hand the crate exactly the
//! two things it needs.
//!
//! Everything else is re-exported unchanged, so existing `pformat::…` call sites
//! keep working.

use crate::openhuman::tools::Tool;

pub use tinyagents::harness::tool_calling::{
    parse_call, render_signature, render_signature_from_schema, PFormatParamType, PFormatRegistry,
    PFormatToolParams,
};

/// Build a [`PFormatRegistry`] from the agent's tool slice.
///
/// Call once at construction time, before the tools are moved into the agent —
/// the result is owned and self-contained, so it survives the move without
/// keeping a reference back to the live `Vec<Box<dyn Tool>>` the agent owns.
///
/// The registry is also the safety boundary the format depends on: the parser
/// refuses to invent argument names for a tool it does not know, so a model
/// cannot tunnel arbitrary JSON through by guessing a tool name that does not
/// exist. A registry built from anything other than the agent's real tools
/// would widen that.
pub fn build_registry(tools: &[Box<dyn Tool>]) -> PFormatRegistry {
    tinyagents::harness::tool_calling::build_registry(
        tools.iter().map(|t| (t.name(), t.parameters_schema())),
    )
}

/// Render a single tool's p-format signature, e.g. `get_weather[location|unit]`.
///
/// This signature goes into the tool catalogue in the system prompt, telling the
/// model exactly how to order positional arguments.
pub fn render_signature_from_tool(tool: &dyn Tool) -> String {
    render_signature_from_schema(tool.name(), &tool.parameters_schema())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::tools::ToolResult;
    use async_trait::async_trait;

    struct StubTool(&'static str);

    #[async_trait]
    impl Tool for StubTool {
        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &str {
            "stub tool"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "count": { "type": "integer" }
                }
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult::success("ok"))
        }
    }

    #[test]
    fn build_registry_keys_on_the_tools_own_names() {
        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(StubTool("echo")), Box::new(StubTool("shell"))];
        let reg = build_registry(&tools);
        assert!(reg.contains_key("echo"));
        assert!(reg.contains_key("shell"));
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn a_tool_absent_from_the_registry_cannot_be_called() {
        // The safety boundary: the parser must not invent argument names for a
        // tool it does not know, or a model could tunnel arbitrary JSON through
        // by guessing a name. This adapter is what decides what is "known".
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(StubTool("echo"))];
        let reg = build_registry(&tools);
        assert!(parse_call("shell[rm -rf /]", &reg).is_none());
    }

    #[test]
    fn a_registered_tool_parses_positionally_through_the_adapter() {
        // End-to-end through the adapter: schema comes from the Tool impl, and
        // arguments come back named and coerced.
        let tools: Vec<Box<dyn Tool>> = vec![Box::new(StubTool("echo"))];
        let reg = build_registry(&tools);
        let (name, args) = parse_call("echo[3|hi]", &reg).expect("known tool parses");
        assert_eq!(name, "echo");
        // Schema properties are ordered alphabetically: count, value.
        assert_eq!(args["count"], 3);
        assert_eq!(args["value"], "hi");
    }

    #[test]
    fn signature_rendering_agrees_between_the_tool_and_schema_forms() {
        // The signature goes into the system prompt; if these two disagreed the
        // catalogue would advertise a different argument order than the parser
        // reconstructs.
        let tool = StubTool("echo");
        let from_tool = render_signature_from_tool(&tool);
        let from_schema = render_signature_from_schema("echo", &tool.parameters_schema());
        assert_eq!(from_tool, from_schema);
        assert_eq!(from_tool, "echo[count|value]");
    }
}
