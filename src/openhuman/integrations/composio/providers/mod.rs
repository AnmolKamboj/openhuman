//! The Composio provider surface, sourced by what each half actually is.
//!
//! This was one line — `pub use crate::openhuman::memory::sync::composio::providers::*;`
//! — a compatibility shim over the engine crate's provider module. The glob
//! hid the fact that two very different things were coming through it, and
//! that most of what this host reads is not provider behaviour at all
//! (OpenHuman#5560).
//!
//! - **The curated catalogs, the scope verdicts and the capability matrix**
//!   come from the contract crate. They are `&'static str` tables and pure
//!   functions over them; nothing about answering "is this action curated, and
//!   at what scope" needs a provider, an HTTP client or a store. Every host
//!   read here — the agent's visible tool list, the `gated_tools` unlock hints,
//!   the agent-ready badge — is one of these.
//! - **The provider registry, the `ComposioProvider` trait and the run types**
//!   still come from the engine crate. That is the syncing half: fetching a
//!   profile, pulling items, normalising tasks. It reaches `reqwest` and the
//!   chunk store, and it is what still has to move behind
//!   `MemorySourceSync`.
//!
//! Keeping the split visible here is the point. While it was a glob, "the host
//! links the memory engine to render a tool list" and "the host links the
//! memory engine to run a sync" were the same line.

// ── The contract half ───────────────────────────────────────────────────────
pub use tinymemory_api::composio::catalogs::{
    catalog_for_toolkit, curated_scope_for, has_native_provider, is_action_visible_with_pref,
    native_provider_sync_interval_secs, sync_interval_env_var, toolkit_description,
    toolkit_has_scope, CAPABILITY_TOOLKITS, NATIVE_PROVIDERS,
};
pub use tinymemory_api::composio::scopes::{
    agent_ready_toolkits, classify_unknown, find_curated, toolkit_from_slug, CuratedTool,
    ToolScope, UserScopePref,
};
pub use tinymemory_api::composio::tasks::{
    GithubFetchMode, NormalizedTask, TaskContainer, TaskFetchFilter, TaskKind,
};
pub use tinymemory_api::composio::{SyncOutcome, SyncReason};
pub use tinymemory_api::host::composio::capability_matrix;

// ── The syncing half, still the engine's ────────────────────────────────────
//
// Each of these is provider *behaviour* or the state it keeps. They go when
// the sync pipelines move behind the bus; until then they are the whole of
// this host's remaining compile-time link to the engine's composio tree, and
// listing them by name is what keeps that measurable.
pub use crate::openhuman::memory::sync::composio::providers::{
    all_providers, get_provider, init_default_providers, load_user_scope_or_default, profile,
    profile_md, register_provider, resolve_sync_interval_secs, slack, sync_state,
    ComposioProvider, ProviderArc, ProviderContext, ProviderUserProfile,
};
