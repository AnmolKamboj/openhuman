//! Tool-pack types: the unit of on-demand tool disclosure.

/// A named bundle of tools that is **not** advertised to the model by default.
///
/// The pack's tools stay fully constructed and executable; what changes is that
/// their JSON schemas never reach the provider until the agent asks for them.
/// That trade is the whole point: an orchestrator carrying ~77 tool schemas
/// spends far more of its fixed per-turn budget on schemas than on its own
/// instructions, and most of those tools go untouched in most conversations.
pub struct ToolPack {
    /// Stable id the agent names in `load_skill` / `use_skill`.
    pub id: &'static str,
    /// One line, rendered in the always-on pack index. This is the only text
    /// about the pack the model sees before loading it, so it has to carry
    /// enough intent for the model to know when to reach for it.
    pub summary: &'static str,
    /// Tool names this pack owns. A name listed here is removed from the
    /// agent's advertised surface and reachable only through `use_skill`.
    pub tools: &'static [&'static str],
}

impl ToolPack {
    pub fn owns(&self, tool: &str) -> bool {
        self.tools.contains(&tool)
    }
}
