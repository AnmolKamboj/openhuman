//! The always-on tool that stands in for every packed tool.

use std::sync::{Arc, RwLock, Weak};

use async_trait::async_trait;
use serde_json::{json, Value};

use super::registry;
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolCallOptions, ToolResult};
use tinytools::ToolRunContext;

pub const USE_SKILL: &str = "use_skill";

/// An `Arc`-shared, owned view of the tool registry a pack tool lives in.
type ToolVec = Arc<Vec<Box<dyn Tool>>>;

/// A non-owning view of the tool registry, kept to break the binding cycle.
type ToolRegistryRef = Weak<Vec<Box<dyn Tool>>>;

/// A late-bound, non-owning view of the tool registries a pack tool reads.
///
/// Late-bound because the pack tool is *inside* the registry it reads: the
/// vector cannot be built until it exists, and it cannot see it until it is
/// built. Non-owning because an `Arc` back into that same vector would be a
/// cycle that never drops.
///
/// **Two registries, not one, and that is load-bearing.** An agent's tools live
/// in two `Arc`s: the durable registry, and the `synthesized_tools` set that
/// `collect_orchestrator_tools` rebuilds whenever the Composio connection set
/// changes (they were split in #6145 so a reconcile cannot block on a reader).
/// Every `delegate_*` tool is in the second one. Binding only the first is why
/// `use_skill` could not reach a single packed delegate — `do_crypto`,
/// `run_skill`, `build_workflow` and four more were withheld from the wire and
/// then unreachable through the route that was supposed to replace them, which
/// is strictly worse than not packing them at all.
#[derive(Clone, Default)]
pub struct PackRegistryHandle {
    inner: Arc<RwLock<Slots>>,
}

/// The two registries, each independently rebindable.
///
/// They are separate slots rather than one vector because they are replaced on
/// different schedules: the durable registry is rebuilt when the agent is, the
/// synthesised one on every delegation refresh.
#[derive(Default)]
struct Slots {
    durable: Option<ToolRegistryRef>,
    synthesized: Option<ToolRegistryRef>,
}

impl PackRegistryHandle {
    /// Point this handle at the durable registry it lives in, replacing any
    /// previous binding.
    ///
    /// Rebinding has to actually take effect. This was a `OnceLock` whose
    /// second write was dropped, which silently contradicted
    /// [`super::bind_pack_registry`]'s own instruction to "re-bind after any
    /// later rebuild of this `Arc`": once an agent replaced its tool vector the
    /// handle still pointed at the old allocation, the `Weak` failed to
    /// upgrade, and every `use_skill` call reported the registry as unavailable
    /// for the rest of the session. Last write wins.
    pub fn bind(&self, registry: ToolRegistryRef) {
        self.with_slots(|slots| slots.durable = Some(registry));
    }

    /// Point this handle at the synthesised delegate set.
    ///
    /// Call it again after **every** `refresh_delegation_tools`, which replaces
    /// that `Arc` wholesale — a stale `Weak` stops upgrading as soon as the last
    /// reader of the old allocation goes, and the packed delegates silently
    /// become unreachable.
    pub fn bind_synthesized(&self, registry: ToolRegistryRef) {
        self.with_slots(|slots| slots.synthesized = Some(registry));
    }

    fn with_slots(&self, edit: impl FnOnce(&mut Slots)) {
        match self.inner.write() {
            Ok(mut slots) => edit(&mut slots),
            // The lock is only ever held for a pointer read or write, so a
            // poisoned lock means a panic elsewhere. Recover rather than
            // propagate: a stale binding degrades to "skill unavailable",
            // which is the failure this rebinding exists to prevent.
            Err(poisoned) => edit(&mut poisoned.into_inner()),
        }
    }

    /// Every live registry, durable first.
    ///
    /// Order matters on a name collision: `drop_synthesized_name_collisions`
    /// gives the durable tool the name, so resolving durable-first is what
    /// makes this agree with what the harness would actually execute.
    fn registries(&self) -> Vec<ToolVec> {
        let slots = match self.inner.read() {
            Ok(slots) => slots,
            Err(poisoned) => poisoned.into_inner(),
        };
        [slots.durable.as_ref(), slots.synthesized.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(Weak::upgrade)
            .collect()
    }

    /// Resolve a packed tool by name, enforcing that it belongs to `skill`.
    ///
    /// The pack check is not decoration: without it `use_skill` would dispatch
    /// into any packed tool regardless of the skill named, and the model could
    /// reach a crypto write through a workflow skill.
    fn resolve(&self, skill: &str, tool: &str) -> Option<(ToolVec, usize)> {
        registry::pack(skill).filter(|p| p.owns(tool))?;
        self.find(tool)
    }

    /// Locate `tool` in whichever registry holds it.
    fn find(&self, tool: &str) -> Option<(ToolVec, usize)> {
        for tools in self.registries() {
            if let Some(idx) = tools.iter().position(|t| t.name() == tool) {
                return Some((tools, idx));
            }
        }
        None
    }
}

fn render_pack(skill: &str, handle: &PackRegistryHandle) -> Result<String, String> {
    let Some(pack) = registry::pack(skill) else {
        return Err(format!(
            "Unknown skill `{skill}`. Available:\n{}",
            registry::pack_index_markdown()
        ));
    };
    let Some(tools) = handle.tools() else {
        return Err(
            "The skill registry is not available in this session; the tools in this skill \
             cannot be loaded."
                .to_string(),
        );
    };

    let mut out = format!("# Skill `{}`\n\n{}\n\n", pack.id, pack.summary);
    out.push_str(&format!(
        "Call these with `use_skill {{ \"skill\": \"{}\", \"tool\": \"<name>\", \"args\": {{ … }} }}`. \
         `args` is the tool's own argument object, exactly as documented below.\n\n",
        pack.id
    ));

    let mut found = 0usize;
    for name in pack.tools {
        // A pack may name a tool this build compiled out (feature gate) or that
        // this agent never had. Rendering the ones that exist beats failing the
        // whole load.
        let Some(tool) = tools.iter().find(|t| t.name() == *name) else {
            continue;
        };
        found += 1;
        out.push_str(&format!(
            "## `{}`\n\n{}\n\n",
            tool.name(),
            tool.description()
        ));
        // Minified, matching what the provider receives for a natively
        // advertised tool. Pretty-printing costs roughly a third more tokens
        // for indentation and newlines the model gains nothing from, and this
        // text is charged to the context window exactly like a native schema.
        out.push_str("```json\n");
        out.push_str(
            &serde_json::to_string(&tool.parameters_schema()).unwrap_or_else(|_| "{}".to_string()),
        );
        out.push_str("\n```\n\n");
    }

    if found == 0 {
        return Err(format!(
            "Skill `{}` has no tools available in this session.",
            pack.id
        ));
    }
    Ok(out)
}

fn skill_enum() -> Vec<&'static str> {
    registry::PACKS.iter().map(|p| p.id).collect()
}

/// The tool named in `args`, if the caller named one at all.
///
/// An absent (or empty) `tool` is not a malformed call: it is the disclosure
/// half of this tool, and the distinction decides both which branch
/// [`UseSkillTool::execute_with_context`] takes and what permission level the
/// call is gated at.
fn named_tool(args: &Value) -> Option<&str> {
    args.get("tool")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
}

/// The one always-on tool that stands in for every packed tool.
///
/// It is both halves of the pack seam. Called with a `skill` alone it renders
/// that pack's tool schemas into the conversation; called with a `skill` and a
/// `tool` it executes that tool. These were two tools — `load_skill` and
/// `use_skill` — until the pack index that each carried in its own description
/// made the pair spend 3.3 kB of every single turn saying one list twice, and
/// made the first call of any packed tool a mandatory two-call round trip.
/// A model that learned the retired name gets the harness's "unknown tool"
/// recovery result (#4249) and retries against the schema it can actually see,
/// so no alias is carried for it.
///
/// **Permission forwarding is load-bearing.** The harness gates a call on the
/// tool's `permission_level_with_args`, so a proxy reporting its own level would
/// launder every packed tool's risk down to this one's — a crypto write would
/// be admitted on a channel that refuses crypto writes. Both accessors resolve
/// the inner tool and defer to it; the arg-less one has nothing to resolve
/// from, so it reports the highest level any packed tool needs rather than
/// guessing low. A call that names no `tool` reads a schema and nothing else,
/// so that one branch is genuinely `ReadOnly`.
pub struct UseSkillTool {
    handle: PackRegistryHandle,
    description: String,
}

impl UseSkillTool {
    pub fn new(handle: PackRegistryHandle) -> Self {
        let description = format!(
            "Reach a skill's tools. Their names, descriptions and argument schemas are NOT in \
             your context until you ask for them: call this with `skill` alone to see them, then \
             again with `skill` + `tool` + `args` to run one.\n\nSkills:\n{}",
            registry::pack_index_markdown()
        );
        Self {
            handle,
            description,
        }
    }

    fn resolve(&self, args: &Value) -> Option<(ToolVec, usize)> {
        let skill = args.get("skill").and_then(Value::as_str)?;
        let tool = named_tool(args)?;
        self.handle.resolve(skill, tool)
    }
}

#[async_trait]
impl Tool for UseSkillTool {
    fn name(&self) -> &str {
        USE_SKILL
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string", "enum": skill_enum(), "description": "Skill to read or run a tool from." },
                "tool": { "type": "string", "description": "Tool to run. Omit to list the skill's tools and their arguments instead." },
                "args": {
                    "type": "object",
                    "description": "The tool's own arguments, as documented in the listing.",
                    "additionalProperties": true
                }
            },
            "required": ["skill"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.execute_with_context(args, ToolCallOptions::default(), None)
            .await
    }

    async fn execute_with_options(
        &self,
        args: Value,
        options: ToolCallOptions,
    ) -> anyhow::Result<ToolResult> {
        self.execute_with_context(args, options, None).await
    }

    async fn execute_with_context(
        &self,
        args: Value,
        options: ToolCallOptions,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        let Some(skill) = args.get("skill").and_then(Value::as_str) else {
            return Ok(ToolResult::error(format!(
                "`skill` is required.\n\nSkills:\n{}",
                registry::pack_index_markdown()
            )));
        };

        // Disclosure half: no tool named, so render the pack's schemas.
        let Some(name) = named_tool(&args) else {
            return Ok(match render_pack(skill, &self.handle) {
                Ok(text) => ToolResult::success(text),
                Err(message) => ToolResult::error(message),
            });
        };

        let Some((tools, idx)) = self.handle.resolve(skill, name) else {
            return Ok(ToolResult::error(format!(
                "No tool `{name}` in skill `{skill}`. Call `use_skill {{ \"skill\": \"{skill}\" }}` \
                 to see what it contains.\n\nSkills:\n{}",
                registry::pack_index_markdown()
            )));
        };
        let inner_args = args.get("args").cloned().unwrap_or_else(|| json!({}));
        tracing::debug!(tool = tools[idx].name(), "[toolpacks] use_skill dispatch");
        tools[idx]
            .execute_with_context(inner_args, options, context)
            .await
    }

    fn supports_markdown(&self) -> bool {
        // The inner result is forwarded verbatim, markdown rendering included,
        // so advertise the capability rather than suppressing a real saving.
        true
    }

    fn external_effect_with_args(&self, args: &Value) -> bool {
        match self.resolve(args) {
            Some((tools, idx)) => {
                let inner_args = args.get("args").cloned().unwrap_or_else(|| json!({}));
                tools[idx].external_effect_with_args(&inner_args)
            }
            None => false,
        }
    }

    fn timeout_policy(&self, args: &Value) -> crate::openhuman::tools::traits::ToolTimeout {
        match self.resolve(args) {
            Some((tools, idx)) => {
                let inner_args = args.get("args").cloned().unwrap_or_else(|| json!({}));
                tools[idx].timeout_policy(&inner_args)
            }
            None => crate::openhuman::tools::traits::ToolTimeout::Inherit,
        }
    }

    fn permission_level(&self) -> PermissionLevel {
        let Some(tools) = self.handle.tools() else {
            return PermissionLevel::Dangerous;
        };
        let packed = registry::all_packed_tool_names();
        tools
            .iter()
            .filter(|t| packed.contains(&t.name()))
            .map(|t| t.permission_level())
            .max()
            // Unbound or empty: report the ceiling, never a permissive default.
            .unwrap_or(PermissionLevel::Dangerous)
    }

    fn permission_level_with_args(&self, args: &Value) -> PermissionLevel {
        // Naming no tool renders a schema and does nothing else. Reporting the
        // packed ceiling here would put an approval prompt in front of reading
        // a tool list, which is the round trip this tool exists to remove.
        if named_tool(args).is_none() {
            return PermissionLevel::ReadOnly;
        }
        match self.resolve(args) {
            Some((tools, idx)) => {
                let inner = args.get("args").cloned().unwrap_or_else(|| json!({}));
                tools[idx].permission_level_with_args(&inner)
            }
            // Unresolvable: the call will fail anyway, but report the ceiling so
            // a malformed call can never be admitted on a channel that would
            // have refused the real tool.
            None => self.permission_level(),
        }
    }

    /// The registry handle rides on the vocabulary's erased host extension:
    /// `PackRegistryHandle` is this host's concept, and `tinytools` has no
    /// business naming it. `traits::pack_registry_handle` reads it back.
    fn host_extension(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
        Some(&self.handle)
    }
}
