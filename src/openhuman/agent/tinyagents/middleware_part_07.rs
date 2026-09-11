    /// The channel-permission gate the engine ran before the builder policy: a
    /// session-level deny, then a per-call permission-level ceiling check. Returns
    /// the blocking message when the call must not execute.
    fn channel_permission_block(&self, call: &TaToolCall) -> Option<String> {
        let decision = self.session.decision_for(&call.name);
        if decision.is_denied() {
            return Some(PolicyDenial::SessionForbidden { tool: &call.name, required: decision.required_permission, allowed: decision.allowed_permission, channel: &self.channel }.render());
        }
        let tool = self.resolve_tool(&call.name)?;
        let call_required = tool.permission_level_with_args(&call.arguments);
        if call_required > decision.allowed_permission {
            return Some(PolicyDenial::PermissionTooLow { tool: &call.name, required: call_required, allowed: decision.allowed_permission, channel: &self.channel }.render());
        }
        if call.name == "use_skill" {
            if let Some(inner_tool) = call.arguments.get("tool").and_then(serde_json::Value::as_str) {
                let inner_decision = self.session.decision_for(inner_tool);
                if inner_decision.blocks_execution() {
                    let hint = crate::openhuman::tools::toolpacks::pack_for_tool(inner_tool)
                        .map(|pack| self.route_for_pack(pack)).filter(|h| !h.is_empty())
                        .map(|h| format!(" {h}")).unwrap_or_default();
                    return Some(format!("Tool `{inner_tool}` is not allowed in the current session and cannot be used through `use_skill`.{hint}"));
                }
            }
        }
        None
    }

    fn generated_context(&self, name: &str, args: &serde_json::Value) -> Option<crate::openhuman::agent::tool_policy::GeneratedToolRuntimeContext> {
        self.tool_sets.iter().flat_map(|set| set.iter()).find(|t| t.name() == name)
            .and_then(|t| crate::openhuman::tools::traits::generated_runtime_context(t.as_ref(), args))
    }
}
