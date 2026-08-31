//! Agent tool: read a compiled persona flavour profile (issue #5172).
//!
//! Persona ingestion (engine-side, `tinycortex::memory::persona`) distills a
//! person's coding-agent history into seven [`PersonaFacet`] flavoured trees
//! (communication, coding style, stack, workflow, environment, directives,
//! anti-preferences), each compiled into a small prompt-ready markdown
//! profile via [`compile_flavoured_root`]. Until this tool, nothing surfaced
//! those compiled profiles to the agent loop — the ingested data sat unread.
//! `memory_flavour` lets an agent pull one facet's profile on demand.
//!
//! Strictly read-only: it never ingests, seals, or otherwise creates persona
//! evidence. The only disk write it can trigger is `compile_flavoured_root`
//! re-staging the fixed-path compiled artifact — a pure, idempotent
//! projection of the tree's existing root node (see
//! `vendor/tinycortex/src/memory/tree/flavoured.rs`), not new memory content.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tinycortex::memory::tree::store::{get_tree_by_scope, TreeKind};
use tinycortex::memory::tree::{compile_flavoured_root, flavoured_root_abs_path};

use crate::openhuman::config::Config;
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolResult};

/// The seven persona facets, host-side (#5560).
///
/// This was `tinycortex::memory::persona::PersonaFacet`, and it came home
/// because it is a pure value type: a field-less enum whose whole behaviour is
/// three total string mappings. Nothing about it needs the engine — the engine
/// functions this file calls take the resulting `String`/`&str`, never the enum
/// — so a host copy is the same value under a different path, not a
/// translation.
///
/// # The strings are an on-disk contract, not cosmetics
///
/// [`Self::tree_scope`] is the **key a flavoured tree is stored under**.
/// Persona ingestion writes `persona/<facet>` into `mem_tree_trees`, and
/// `get_tree_by_scope` finds it by exact string match. So the mappings below
/// are reproduced verbatim from the engine, and a "tidy-up" that renames one
/// (`coding_style` → `codingStyle`, say) does not fail a build or throw — it
/// silently stops finding a tree that is still there, and `memory_flavour`
/// starts answering "No profile built yet" forever.
///
/// [`Self::parse_loose`]'s alias table is the agent-facing half of the same
/// contract: an LLM emits `tone` or `pet_peeves`, and dropping an alias
/// narrows what the tool accepts. [`Self::heading`] is display-only and the one
/// mapping here that is safe to reword.
///
/// The engine's enum carries three more members this host never reads — `ALL`
/// (the pack's fixed compile order), `default_ask` (per-facet ingestion
/// prompts) and its serde derives. They are ingestion concerns and are
/// deliberately not copied: an unused copy is a second thing to keep in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersonaFacet {
    /// Tone, verbosity, directness, phrasing quirks, how they give feedback.
    Communication,
    /// Naming, structure, comments, error handling, testing habits.
    CodingStyle,
    /// Languages, frameworks, libraries, recurring architectural choices.
    Stack,
    /// Branching/commit granularity, plan-first vs. dive-in, PR habits.
    Workflow,
    /// Editors/harnesses, CLIs, package managers, OS.
    Environment,
    /// Explicit standing rules (mostly T0, near-verbatim).
    Directives,
    /// Pet peeves: things they correct agents for, revert, or forbid.
    AntiPreferences,
}

impl PersonaFacet {
    /// Stable string form. Verbatim from the engine — see the type's docs for
    /// why this one is not free to change.
    fn as_str(self) -> &'static str {
        match self {
            PersonaFacet::Communication => "communication",
            PersonaFacet::CodingStyle => "coding_style",
            PersonaFacet::Stack => "stack",
            PersonaFacet::Workflow => "workflow",
            PersonaFacet::Environment => "environment",
            PersonaFacet::Directives => "directives",
            PersonaFacet::AntiPreferences => "anti_preferences",
        }
    }

    /// Human-facing section heading used in error and "not built" messages.
    /// Display-only, so this is the one mapping here that may be reworded.
    pub(crate) fn heading(self) -> &'static str {
        match self {
            PersonaFacet::Communication => "Communication style",
            PersonaFacet::CodingStyle => "Coding style",
            PersonaFacet::Stack => "Stack",
            PersonaFacet::Workflow => "Workflow",
            PersonaFacet::Environment => "Environment",
            PersonaFacet::Directives => "Directives",
            PersonaFacet::AntiPreferences => "Anti-preferences",
        }
    }

    /// Flavoured-tree scope for this facet (`persona/<facet>`) — the exact key
    /// the tree is persisted under.
    pub(crate) fn tree_scope(self) -> String {
        format!("persona/{}", self.as_str())
    }

    /// Parse the loose forms an LLM might emit.
    pub(crate) fn parse_loose(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace([' ', '-'], "_").as_str() {
            "communication" | "comms" | "tone" => Some(PersonaFacet::Communication),
            "coding_style" | "code_style" | "coding" | "style" => Some(PersonaFacet::CodingStyle),
            "stack" | "tech_stack" | "technology" => Some(PersonaFacet::Stack),
            "workflow" | "process" => Some(PersonaFacet::Workflow),
            "environment" | "env" | "tooling" => Some(PersonaFacet::Environment),
            "directives" | "rules" | "directive" => Some(PersonaFacet::Directives),
            "anti_preferences" | "anti_preference" | "antipreferences" | "dislikes"
            | "pet_peeves" => Some(PersonaFacet::AntiPreferences),
            _ => None,
        }
    }
}

/// The seven valid `flavour` slugs, for error messages.
const VALID_FLAVOURS: &str =
    "communication, coding_style, stack, workflow, environment, directives, anti_preferences";

/// Let the agent read the compiled persona profile for one facet.
pub struct MemoryFlavourTool {
    config: Arc<Config>,
}

impl MemoryFlavourTool {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

/// Strip the YAML front matter written by [`compile_flavoured_root`]
/// (`---\n...\n---\n<body>`) and return just the body. Front-matter field
/// values are single-line (`yaml_quote` collapses interior newlines), so the
/// first `\n---\n` after the opening delimiter is always the closing one.
fn body_after_front_matter(content: &str) -> &str {
    match content.strip_prefix("---\n") {
        Some(rest) => match rest.find("\n---\n") {
            Some(pos) => &rest[pos + "\n---\n".len()..],
            // Malformed front matter (opener present but no closer): fall
            // back to everything after the opening delimiter rather than
            // the raw content, so the opener itself is never leaked.
            None => rest,
        },
        None => content,
    }
}

/// Outcome of [`lookup_flavour`] — split from a hard `Err` so a caller can
/// distinguish "bad input" (never reached the store) from "reached the store
/// and here's what it found (or didn't, or a lookup itself failed)".
pub(crate) enum FlavourLookup {
    /// A compiled profile body, ready to hand to the agent/node.
    Profile(String),
    /// No profile has been built yet for this facet — not an error, just
    /// empty (persona ingestion hasn't run, or produced nothing for this
    /// facet yet).
    NotBuilt(String),
    /// The tree lookup or compile step itself failed (I/O, corrupt tree,
    /// …) — distinct from `NotBuilt` because this IS an error, just one
    /// discovered after `flavour_raw` was already validated.
    Failed(String),
}

/// Pure lookup shared by [`MemoryFlavourTool::execute`] and the tinyflows
/// `memory` node's `flavour` operation
/// (`OpenHumanMemory::flavour` in `crate::openhuman::flows::tinyflows::memory_adapter`)
/// — both surfaces read the exact same flavoured-tree path, so there is only
/// one place that knows how a `flavour` slug resolves to a compiled profile.
///
/// `Err` is reserved for input the caller should have caught before ever
/// reaching the store (empty/unknown `flavour_raw`); everything the store
/// itself can report — hit, miss, or lookup failure — comes back as `Ok` of
/// the matching [`FlavourLookup`] variant so callers can shape each case
/// (tool result vs. node output) however their surface needs.
pub(crate) fn lookup_flavour(config: &Config, flavour_raw: &str) -> Result<FlavourLookup, String> {
    let flavour_raw = flavour_raw.trim();
    if flavour_raw.is_empty() {
        return Err("'flavour' cannot be empty".to_string());
    }

    let facet = PersonaFacet::parse_loose(flavour_raw).ok_or_else(|| {
        format!("Unknown flavour '{flavour_raw}'. Valid flavours: {VALID_FLAVOURS}")
    })?;

    // The `MemoryConfig` the three TinyCortex calls below take, built here
    // rather than through `tinymemory_core::tinycortex::memory_config_from` —
    // the engine crate's `Config` → `MemoryConfig` mapping, and what used to be
    // the one `tinymemory_core::` reference in this file (#5560).
    //
    // That mapping sets three fields. Two of them are read on this path and are
    // reproduced verbatim: `workspace`, which is where `get_tree_by_scope` and
    // `compile_flavoured_root` open the shared chunk/tree connection, and
    // `content_root`, which `flavoured_root_abs_path` resolves the compiled
    // artifact under (`memory_tree.content_dir` when the user set one, else
    // `<workspace>/memory_tree/content`). `Config::memory_tree_content_root` is
    // the host's own single source of truth for that path, so this reads the
    // same value the engine mapping read.
    //
    // The third — `embedding`, whose `provider` the engine derives from its
    // `effective_embedder_slug` ladder — is deliberately left at its default,
    // and this is the one reduction to be aware of. That field is the signature
    // per-model embedding sidecar rows are keyed by, so it matters wherever a
    // vector is written or matched; **nothing on this path is.** `memory_flavour`
    // is strictly read-only over the flavoured tree: `get_tree_by_scope` and
    // `store::get_summary` are plain SQL over `mem_tree_trees` /
    // `mem_tree_summaries`, and `compile_flavoured_root` clamps the root node's
    // stored content to `tree.flavour_root_token_budget` and stages it as
    // markdown. None of the three reads `config.embedding`.
    //
    // So: if a call that embeds, re-embeds, or matches a vector is ever added
    // to this file, this config is no longer sufficient and the embedder ladder
    // has to come with it. A defaulted signature would file rows under the
    // wrong provider, which is silent rather than loud.
    let mut mc = tinycortex::memory::MemoryConfig::new(config.workspace_dir.clone());
    mc.content_root = Some(config.memory_tree_content_root());
    let scope = facet.tree_scope();
    let heading = facet.heading();

    tracing::debug!(
        target: "memory_flavour",
        flavour = flavour_raw,
        facet = ?facet,
        "[memory_flavour] entry"
    );

    // Fast path: the compiled artifact already exists on disk with a
    // non-empty body — read it directly without touching the tree store.
    let abs_path = flavoured_root_abs_path(&mc, &scope);
    if abs_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&abs_path) {
            let body = body_after_front_matter(&content);
            if !body.trim().is_empty() {
                tracing::debug!(
                    target: "memory_flavour",
                    flavour = flavour_raw,
                    body_len = body.len(),
                    "[memory_flavour] fast path hit: returning stripped body from disk"
                );
                return Ok(FlavourLookup::Profile(body.to_string()));
            }
        }
    }

    tracing::debug!(
        target: "memory_flavour",
        flavour = flavour_raw,
        "[memory_flavour] fast path missed or empty, falling to tree lookup"
    );

    // Slow path: look up the flavoured tree and (re)compile its root.
    match get_tree_by_scope(&mc, TreeKind::Flavoured, &scope) {
        Ok(None) => {
            tracing::debug!(
                target: "memory_flavour",
                flavour = flavour_raw,
                "[memory_flavour] no flavoured tree exists yet"
            );
            Ok(FlavourLookup::NotBuilt(format!(
                "No profile built yet for {heading}. Run persona ingestion first, then try \
                 again."
            )))
        }
        Ok(Some(tree)) => {
            tracing::debug!(
                target: "memory_flavour",
                flavour = flavour_raw,
                tree_id = %tree.id,
                "[memory_flavour] tree found, compiling root"
            );
            match compile_flavoured_root(&mc, &tree.id) {
                Ok(markdown) => {
                    let body = body_after_front_matter(&markdown);
                    if body.trim().is_empty() {
                        Ok(FlavourLookup::NotBuilt(format!(
                            "No profile built yet for {heading}. Run persona ingestion \
                             first, then try again."
                        )))
                    } else {
                        tracing::debug!(
                            target: "memory_flavour",
                            flavour = flavour_raw,
                            body_len = body.len(),
                            "[memory_flavour] compiled profile returned"
                        );
                        Ok(FlavourLookup::Profile(body.to_string()))
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        %err,
                        flavour = flavour_raw,
                        "[memory_flavour] failed to compile flavoured profile"
                    );
                    Ok(FlavourLookup::Failed(format!(
                        "Failed to compile the {heading} profile: {err}"
                    )))
                }
            }
        }
        Err(err) => {
            tracing::warn!(
                %err,
                flavour = flavour_raw,
                "[memory_flavour] failed to look up flavoured tree"
            );
            Ok(FlavourLookup::Failed(format!(
                "Failed to look up the {heading} profile: {err}"
            )))
        }
    }
}

#[async_trait]
impl Tool for MemoryFlavourTool {
    fn name(&self) -> &str {
        "memory_flavour"
    }

    fn description(&self) -> &str {
        "Read the compiled persona profile for one distillation facet, built from this \
         person's coding-agent history. Valid `flavour` values: communication (tone, \
         verbosity, feedback style), coding_style (naming, structure, testing habits), \
         stack (languages, frameworks, architecture), workflow (branching, PR habits, \
         parallelism), environment (editors, harnesses, CLIs, OS), directives (explicit \
         standing rules), anti_preferences (things to never do). Returns markdown prose, or \
         a clear message if no profile has been built yet. Read-only."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "flavour": {
                    "type": "string",
                    "description": "Which persona facet to read: communication, coding_style, \
                        stack, workflow, environment, directives, or anti_preferences."
                }
            },
            "required": ["flavour"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        // Genuinely read-only: this tool never ingests, seals, or writes
        // memory content. Overridden explicitly (not relying on the trait
        // default) so a future default change can't silently loosen this.
        PermissionLevel::ReadOnly
    }

    fn permission_level_with_args(&self, _args: &serde_json::Value) -> PermissionLevel {
        // No arg combination for this tool escalates past ReadOnly.
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let flavour_raw = args
            .get("flavour")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'flavour' parameter"))?;

        match lookup_flavour(&self.config, flavour_raw) {
            Err(hard) => Err(anyhow::anyhow!(hard)),
            Ok(FlavourLookup::Profile(body)) => Ok(ToolResult::success(body)),
            Ok(FlavourLookup::NotBuilt(msg)) => Ok(ToolResult::success(msg)),
            Ok(FlavourLookup::Failed(msg)) => Ok(ToolResult::error(msg)),
        }
    }
}

#[cfg(test)]
#[path = "flavour_tests.rs"]
mod tests;
