# Removing `tinymemory-core` and `tinycortex` from OpenHuman

Goal: the host links **`tinymemory-api` only**. All memory behaviour reaches the
loaded TinyMemory TinyBus module through `MemoryProvider`; nothing in `src/`
names an engine crate.

Spans three repos: `openhuman` (host), `vendor/tinymemory`, `vendor/tinycortex`.
Branch `tinymemory-bus-only` in each.

## Measured starting point (2026-08-30)

- `modules::registry` pins tinymemory **v1.13.3** — 144 bus members, 23
  capability families, all implemented by `ModuleMemoryProvider`. The in-tree
  docs claiming v1.0.1/v1.2.0 and a narrow seam are stale.
- `DriverClass::Embedded` is refused (`binding.rs:252`), so every
  `MemoryProvider` call is already over the bus.
- Production files naming `tinymemory_core::`: **23**. Naming `tinycortex::`: **~45**.
- Deleting the six engine glob re-exports yields **40 compile errors in ~30 files**.
  That is the true host coupling.

## The three clusters, and where each goes

### A. Composio providers (~20 files, 8.8k LOC in `tinymemory-core`)

Consumed mostly by `integrations/composio`, `flows` and `task_sources` — none of
which is memory. Split by what the thing *is*:

| Part | Destination | Why |
| --- | --- | --- |
| `tool_scope`, `catalogs*`, `descriptions`, `scope_lookup`, `capability_matrix`, `is_action_visible_with_pref` | **`tinymemory-bus::composio`** | static tables + pure functions, no I/O, no runtime. `ToolScope`/`UserScopePref`/`CuratedTool` types are already there; `tool_scope.rs` is a 32-line re-export of them. |
| `user_scopes::load_or_default` | bus `KvGet`/`KvPut` via `MemoryGraph` | it is a KV read (`crate::store::MemoryClientRef`), not a table. |
| `ComposioProvider` trait, `registry`, `ProviderContext`, provider impls (github/gmail/linear/notion/slack/clickup), `profile`, `profile_md`, `sync_state`, `periodic` | **stays sync-side** (tinycortex/module) | this is the syncing half. Host reaches it through `MemorySourceSync`. |

Host `get_provider(..).curated_tools()` becomes `catalog_for_toolkit(..)` from
the contract, which removes most `get_provider` call sites outright. The ones
that remain are real sync behaviour (`fetch_tasks`, `list_databases`,
`fetch_user_profile`) and need bus members.

### B. Tree (~9 files) — becomes graph/recall/retrieval via the API

Do **not** ask upstream for tree-shaped twins. Map onto the existing families:

| Host call today | Goes to |
| --- | --- |
| `tree::retrieval::{fast_retrieve, FastRetrieveOptions, QueryResponse}` | `MemoryRetrieval::fast_retrieve` |
| `tree::retrieval::source::query_source_scoped` | `MemoryTree::query_source` / `MemoryRetrieval::retrieve_source` |
| `tree::retrieval::types::NodeKind` | `RetrievalNodeKind` |
| `tree::score::extract::EntityKind` | `provider::retrieval::EntityKind` |
| `tree::health::async_run_doctor` | `MemoryMaintenance::doctor` |
| `tree::score::DEFAULT_DROP_THRESHOLD` consumer | `MemoryProfile::drop_facets_below` |

Comes **home** rather than going to the bus, because both build an LLM chat
provider and the host owns inference (same argument as `source_scope`,
`memory::safety`, `util::redact`):

- `tree::summarise::{summarise, SummaryContext, SummaryInput}` — `crate::chat::build_chat_provider`
- `tree_runtime::engine::{run_summarization, rebuild_tree}` — `tinyagents::harness::model::ChatModel`

Genuinely missing from the contract, so upstream asks:

- `MemoryEntities::entity_score(id)` — replaces `tree::score::store::get_score`
- `DEFAULT_DROP_THRESHOLD` as a contract constant
- `MemoryTree` extension for the node store: `read_node`, `read_children`,
  `tree_status`, `write_node`, `buffer_write`, namespace/node-id validation
  (`tree_runtime::store::*`)

### C. Sources (~4 files)

`sources::{readers, registry, types, sync}` → `MemorySourceSink`,
`MemorySourceSync`, `MemoryChunks::source_totals`.

### D. `tinycortex` direct calls — the hard one

`tinycortex::memory::conversations::{list_threads, get_messages, ensure_thread,
append_message, purge_threads, ConversationMessage, CreateConversationThread}`
has **no contract family at all**. Callers: `memory/conversations/bus.rs`,
`memory/conversations/blocking.rs`, `channels/host/adapters.rs`, `threads/`.
Upstream ask: a `MemoryConversations` family.

Also engine-shaped: `memory::archivist::store::{session_entries, record_turn}`
(→ `MemoryEpisodic`, mostly covered), `memory::tree::{compile_flavoured_root,
flavoured_root_abs_path, get_tree_by_scope}` (`memory/tools/flavour.rs`),
`memory::persona::PersonaFacet` (→ `ProfileFacet`),
`memory::ingest::canonicalize::{chat, document}`.

## Ordering (each step must leave the tree green)

1. **tinymemory-bus**: move the static composio catalog/scope surface into the
   contract. No release coupling — it is a compile-time crate.
2. **Host**: repoint `integrations/composio`, `flows`, `task_sources` at the
   contract; delete `memory::sync::composio` re-export shims that are now dead.
3. **Host**: bring `summarise` and the tree-runtime summarisation home.
4. **Host**: repoint the tree/sources clusters onto retrieval/graph/recall.
5. **Upstream tinymemory/tinycortex**: `MemoryConversations`, the `MemoryTree`
   node-store extension, `entity_score`. Release; re-pin `modules::registry`.
6. **Host**: last engine callers; delete `host_impls` seam installation
   (`core/runtime/context.rs:616`), bring `thread_context` and
   `learning_candidate` home **in the same commit** as the engine's removal.
7. Drop both crates from `Cargo.toml` (deps and dev-deps) and both `[patch]`
   blocks; delete `direct_engine_refs_tests` once it can only be empty.

### Two traps that must not be split across commits

- **`thread_context`** is a `tokio::task_local!`. `tinymemory_core::store::recall_policy.rs:58`
  reads it. Two `task_local!` invocations are two keys, and unset means
  *exclude nothing* — recall would silently start echoing the caller's own
  thread back. It comes home **with** the engine, not before.
- **`learning_candidate::global()`** is a process-global ring buffer that the
  engine's `sync/composio/providers/profile.rs:129` pushes into. Moving it home
  while `sync::composio` still resolves into the engine gives two buffers and a
  silently empty one.
