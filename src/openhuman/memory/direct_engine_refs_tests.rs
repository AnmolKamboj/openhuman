//! Enforcement lint: the set of production files that call `tinymemory-core`
//! **directly**, around the module seam, must not grow.
//!
//! # Why a second path to memory is a correctness problem, not a size problem
//!
//! `memory::binding` says it plainly: "the built-in driver is the compiled
//! TinyMemory TinyBus module. The host no longer exposes an embedded engine
//! class for memory." Every call that goes over that bus is round-tripped
//! through [`crate::openhuman::memory::api::wire`]'s error table — the one
//! `modules/memory.rs` keeps shared "because reimplementing the mapping here is
//! what would let a `PathEscape` arrive as an `Invalid`, silently reclassifying
//! a sandbox escape as a caller mistake" — and is filtered by the capability
//! set `ModuleMemoryProvider::verify` cross-checks against the module's own
//! answer.
//!
//! A direct `tinymemory_core::…` call gets neither. It is a second, unpoliced
//! door into the same subsystem, and two doors into one capability is a
//! capability whose behaviour can diverge. `tinymemory-core` also stays linked
//! into the shipped binary — 1.44 MB of `.text`, the 7th largest crate — for as
//! long as any of these remain, which is the visible symptom rather than the
//! disease (#5560).
//!
//! # This lint is a ratchet, not an invariant
//!
//! Same shape and same reasoning as [`super::bypass_allowlist_tests`], and as
//! `INTENTIONALLY_NOT_FORWARDED` in `scripts/lib/feature-forwarding.mjs`: the
//! current direct callers are enumerated in [`ALLOWED`] with a classification
//! and a reason each, and **that list may shrink but must never grow**. A lint
//! that was red on day one would be `#[ignore]`d within a week; a green ratchet
//! converges.
//!
//! # The classification, and why most of the list cannot move yet
//!
//! Each entry carries a [`Verdict`], which is the inventory the migration is
//! driven from:
//!
//! - [`Verdict::SeamExpressible`] — the existing `MemoryProvider` surface
//!   already covers this. These are the ones to migrate; a non-empty set here
//!   is a to-do list, not a steady state.
//! - [`Verdict::NeedsWiderSeam`] — the call wants something the thirteen
//!   capability families do not expose. **These are blocked upstream, not
//!   here.** `modules::registry` pins the TinyMemory module to a released,
//!   SHA-256-verified artifact (v1.0.1 at the time of writing), so a new bus
//!   method is a `tinymemory` release plus a registry re-pin before it is a
//!   host change. Adding the trait method alone would produce a driver that
//!   answers `Unsupported` — strictly worse than the direct call it replaced,
//!   because the failure moves from compile time to run time.
//! - [`Verdict::HostSide`] — not a driver call at all. Re-export shims,
//!   host-seam installation, and inert type imports. These are correct as they
//!   stand and are counted only so "deliberate" stays distinguishable from
//!   "forgotten".
//!
//! ## The concrete gaps, for whoever picks the upstream work up
//!
//! The seam's tree family is `query_source(namespace, source_id, limit, scope)
//! -> Vec<Chunk>`, `drill_down(namespace, node_id) -> QueryResult`, `append`,
//! `seal`, `cascade`. What the host actually calls is richer in four ways, and
//! each is a distinct upstream ask:
//!
//! 1. **Retrieval takes filters the seam has no room for** — a time window, a
//!    free-text query, a `SourceKind`, a depth and a limit
//!    (`query::backend`, `query::cover_window`, `query::fast_walk`).
//! 2. **Chunk reads have no family at all.** `store::chunks::store::{get_chunk,
//!    list_chunks}` with a nine-field `ListChunksQuery` backs three agent tools
//!    (`memory_chunk_context`, `raw_chunks`, `vector_search`); the seam's only
//!    chunk door is `query_source`, keyed on a single source id.
//! 3. **Entity search has no kind filter.** `MemoryEntities::entities` takes
//!    `(namespace, query, limit)`; `memory_tree_search_entities` additionally
//!    filters on `Vec<EntityKind>`.
//! 4. **Sources cannot be listed.** `MemorySourceSink` is
//!    `accept_source_items` + `forget_source`; `memory_diff` needs
//!    `sources::{get_source, list_sources}`.
//!
//! Two more sit outside the tree families entirely: the `people` domain has no
//! capability family (`PeopleStore`, `Handle`, `PersonId`), and
//! `source_scope::{current_source_scope, chunk_source_allowed}` is a
//! task-local the host sets and the engine reads — it is host policy that
//! happens to live in the engine crate, and it should probably move to
//! `tinymemory-api` rather than gain a bus method.
//!
//! # Known weaknesses, stated rather than hidden
//!
//! - **The lint sees text, not types.** A reference reached through a
//!   re-export under another name is invisible to it — and the memory tree is
//!   full of those on purpose: `memory/mod.rs` re-exports twenty-five engine
//!   modules, and ~687 `memory::store::…` / `memory::tree::…` paths elsewhere
//!   resolve into the crate through them. **This lint deliberately does not
//!   count those.** It counts the sites that *name* the crate, because those
//!   are the ones a migration edits. The re-export surface is a separate,
//!   larger problem tracked in the issue, and pretending this number covers it
//!   would be the worst outcome.
//! - **By-path test files are out of scope** (`*_tests.rs`, `tests.rs`,
//!   `test_support/`), matching the sibling lint. Several inline
//!   `#[cfg(test)]` modules do name the crate (`query::drill_down`,
//!   `query::fetch_leaves`, `query::query_source` each assert a tool's result
//!   against a direct engine call); those files are listed, and the entry says
//!   so.
//! - **Comment lines are skipped**, so the many doc comments that reference
//!   `tinymemory_core::…` by path do not inflate the count.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Why a file may name the engine crate today.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Verdict {
    /// The existing `MemoryProvider` surface covers this. Migrate it.
    SeamExpressible,
    /// Blocked on a wider bus surface, which means an upstream `tinymemory`
    /// release and a `modules::registry` re-pin.
    NeedsWiderSeam,
    /// Not a driver call: a re-export shim, host-seam installation, or an
    /// inert type import.
    HostSide,
}

/// The literal this lint searches for. A single needle, deliberately: the
/// question is "does this file name the engine crate", not "which item".
const NEEDLE: &str = "tinymemory_core::";

/// `(repo-relative path, verdict, why it names the engine today)`.
///
/// Adding an entry is a decision, not a way to silence the lint. Sorted by
/// path — [`scan`] returns a `BTreeSet`, so keeping the literal in the same
/// order makes diffs readable.
const ALLOWED: &[(&str, Verdict, &str)] = &[
    // ── Re-export shims: `pub use tinymemory_core::<domain>::*;` ────────────
    //
    // These are the historical-path aliases `memory/mod.rs` documents. They
    // name the crate once each and call nothing. Removing them is the
    // re-export problem, not the direct-call problem.
    (
        "src/openhuman/agent/learning/candidate.rs",
        Verdict::HostSide,
        "re-export shim for learning_candidate types",
    ),
    (
        "src/openhuman/agent/tinyagents/thread_context.rs",
        Verdict::HostSide,
        "re-export shim for the thread-id task-local",
    ),
    (
        "src/openhuman/inference/embeddings/provider_trait.rs",
        Verdict::HostSide,
        "re-export shim for TinyAgentsEmbeddingProvider, which cannot live host-side",
    ),
    (
        "src/openhuman/memory/conversations/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::conversations::*",
    ),
    (
        "src/openhuman/memory/diff/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::diff::*",
    ),
    (
        "src/openhuman/memory/mod.rs",
        Verdict::HostSide,
        "the re-export block itself — twenty-five engine modules under their historical paths",
    ),
    (
        "src/openhuman/memory/people/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::people::*",
    ),
    (
        "src/openhuman/memory/sources/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sources::*",
    ),
    (
        "src/openhuman/memory/sync/composio/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sync::composio::*",
    ),
    (
        "src/openhuman/memory/sync/composio/providers/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sync::composio::providers::*",
    ),
    (
        "src/openhuman/memory/sync/composio/providers/slack/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sync::composio::providers::slack::*",
    ),
    (
        "src/openhuman/memory/sync/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sync::*",
    ),
    (
        "src/openhuman/memory/sync/sync_status/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::sync::sync_status::*",
    ),
    (
        "src/openhuman/memory/tool_memory/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tool_memory::*",
    ),
    (
        "src/openhuman/memory/tree/health/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tree::health::*",
    ),
    (
        "src/openhuman/memory/tree/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tree::*",
    ),
    (
        "src/openhuman/memory/tree/retrieval/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tree::retrieval::*",
    ),
    (
        "src/openhuman/memory/tree/tree/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tree::tree::*",
    ),
    (
        "src/openhuman/memory/tree/tree_runtime/mod.rs",
        Verdict::HostSide,
        "re-export shim: pub use tinymemory_core::tree::tree_runtime::*",
    ),
    // ── Host-seam installation: the host handing itself TO the engine ───────
    //
    // The direction of these is inbound, not outbound: they install embedding /
    // chat / config / NLP / scheduler / shutdown / error-reporting callbacks
    // into the in-process engine. `modules/memory_host.rs` is the same seam
    // served over the bus. They are what an embedded engine needs, and they
    // are the last thing to remove, not the first.
    (
        "src/openhuman/memory/host.rs",
        Verdict::HostSide,
        "installs the event sink into the in-process engine",
    ),
    (
        "src/openhuman/memory/host_impls.rs",
        Verdict::HostSide,
        "installs the eight host seams (embedding, chat, composio, config, nlp, scheduler gate, shutdown, error reporter); mirrored over the bus by modules/memory_host.rs",
    ),
    // ── Retrieval: filters the seam's tree family has no room for ───────────
    (
        "src/openhuman/memory/query/backend.rs",
        Verdict::NeedsWiderSeam,
        "retrieval::source::query_source / drill_down / fetch_leaves take a time window, a free-text query, a SourceKind and a depth; MemoryTree::query_source takes (namespace, source_id, limit, scope) and drill_down takes (namespace, node_id)",
    ),
    (
        "src/openhuman/memory/query/cover_window.rs",
        Verdict::NeedsWiderSeam,
        "retrieval::cover::cover_window has no seam equivalent",
    ),
    (
        "src/openhuman/memory/query/drill_down.rs",
        Verdict::NeedsWiderSeam,
        "inline #[cfg(test)] module only — asserts the tool result against a direct engine drill_down; the production path goes through query/backend.rs",
    ),
    (
        "src/openhuman/memory/query/fast_walk.rs",
        Verdict::NeedsWiderSeam,
        "retrieval::fast_retrieve (the E2GraphRAG retriever) has no seam equivalent",
    ),
    (
        "src/openhuman/memory/query/fetch_leaves.rs",
        Verdict::NeedsWiderSeam,
        "inline #[cfg(test)] module only — asserts the tool result against a direct engine fetch_leaves",
    ),
    (
        "src/openhuman/memory/query/ingest_document.rs",
        Verdict::NeedsWiderSeam,
        "names SourceKind / SourceRef, which are tinycortex-api types the engine re-exports — NOT the same type as the contract's api::chunks::SourceKind, so this is not a type carve-out",
    ),
    (
        "src/openhuman/memory/query/query_source.rs",
        Verdict::NeedsWiderSeam,
        "SourceKind in production, plus an inline #[cfg(test)] assertion against a direct engine query_source",
    ),
    (
        "src/openhuman/memory/query/search_entities.rs",
        Verdict::NeedsWiderSeam,
        "retrieval::search_entities filters on Vec<EntityKind>; MemoryEntities::entities takes (namespace, query, limit) only",
    ),
    // ── Agent tools: chunk reads, source listing, people, source scope ──────
    (
        "src/openhuman/memory/sync/composio/providers/context_ext.rs",
        Verdict::NeedsWiderSeam,
        "extends the engine's ProviderContext; the sync pipeline is engine-internal and has no capability family",
    ),
    (
        "src/openhuman/memory/tools/diff.rs",
        Verdict::NeedsWiderSeam,
        "sources::{get_source, list_sources}; MemorySourceSink is accept_source_items + forget_source, with no list door",
    ),
    (
        "src/openhuman/memory/tools/people.rs",
        Verdict::NeedsWiderSeam,
        "people::store::PeopleStore and the Handle/Interaction/PersonId vocabulary; there is no people capability family",
    ),
    (
        "src/openhuman/memory/tools/raw_store/kinds.rs",
        Verdict::NeedsWiderSeam,
        "store::MemoryKind — an engine type with no contract counterpart",
    ),
    (
        "src/openhuman/memory/tools/raw_store/raw_chunks.rs",
        Verdict::NeedsWiderSeam,
        "store::chunks::store::list_chunks with a nine-field ListChunksQuery, plus the source_scope task-local",
    ),
    (
        "src/openhuman/memory/tools/raw_store/raw_search.rs",
        Verdict::NeedsWiderSeam,
        "retrieval::search::search_entities with an EntityKind filter",
    ),
    (
        "src/openhuman/memory/tools/search/chunk_context.rs",
        Verdict::NeedsWiderSeam,
        "get_chunk / list_chunks by id and source, plus source_scope::chunk_source_allowed",
    ),
    (
        "src/openhuman/memory/tools/search/hybrid_search.rs",
        Verdict::NeedsWiderSeam,
        "constructs a UnifiedMemory directly and reads MemoryItemKind; the seam has no constructor door",
    ),
    (
        "src/openhuman/memory/tools/search/vector_search.rs",
        Verdict::NeedsWiderSeam,
        "vector chunk search over ListChunksQuery, plus the source_scope task-local",
    ),
];

/// True for source files the lint deliberately does not scan.
///
/// By-path only, matching [`super::bypass_allowlist_tests`] — see that module
/// for why inline `#[cfg(test)]` blocks are left in scope rather than
/// brace-tracked.
fn is_test_path(path: &Path) -> bool {
    if path.components().any(|c| c.as_os_str() == "test_support") {
        return true;
    }
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name == "tests.rs" || name.ends_with("_tests.rs"),
        None => false,
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") && !is_test_path(&path) {
            out.push(path);
        }
    }
}

/// Every repo-relative path in this crate's `src` that names the engine crate
/// outside a comment.
fn scan() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&root.join("src"), &mut files);

    let mut found = BTreeSet::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains(NEEDLE) {
                found.insert(rel.clone());
                break;
            }
        }
    }
    found
}

fn allowed_set() -> BTreeSet<String> {
    ALLOWED
        .iter()
        .map(|(path, _, _)| (*path).to_string())
        .collect()
}

fn render(paths: impl IntoIterator<Item = String>) -> String {
    paths.into_iter().map(|p| format!("\n  {p}")).collect()
}

/// A scanner that silently found nothing would turn every other test here into
/// a rubber stamp, so refuse to pass vacuously.
///
/// `memory/mod.rs` is the most stable pin available: it is the re-export block
/// itself, so it names the crate by construction. If the scanner stops seeing
/// it, the scanner is broken — fix it, do not relax this assertion.
#[test]
fn direct_reference_scanner_is_not_vacuous() {
    let found = scan();
    assert!(
        found.contains("src/openhuman/memory/mod.rs"),
        "scanner found no direct engine reference in memory/mod.rs, which is the re-export block; \
         the scanner is broken"
    );
    assert!(
        found.len() > 20,
        "scanner found only {} files; expected the full direct-reference surface",
        found.len()
    );
}

/// **The ratchet.** A new file naming `tinymemory_core::` fails here.
///
/// If the new call is genuinely unavoidable, add it to [`ALLOWED`] with a
/// [`Verdict`] and a reason. If it is not, route it through
/// `CoreContext::memory()` and the `MemoryProvider` seam.
#[test]
fn no_new_files_call_the_engine_directly() {
    let found = scan();
    let allowed = allowed_set();
    let unexpected: Vec<String> = found.difference(&allowed).cloned().collect();
    assert!(
        unexpected.is_empty(),
        "new direct `tinymemory_core::` reference(s) — route these through the MemoryProvider seam, \
         or add them to ALLOWED with a Verdict and a reason:{}",
        render(unexpected)
    );
}

/// The staleness half. An allowlist that outlives its entries rots into dead
/// strings that document nothing — the same failure `INTENTIONALLY_NOT_FORWARDED`
/// guards against. A migrated file must be *removed* from the list, so the
/// count is always the real one.
#[test]
fn allowlist_has_no_stale_entries() {
    let found = scan();
    let allowed = allowed_set();
    let stale: Vec<String> = allowed.difference(&found).cloned().collect();
    assert!(
        stale.is_empty(),
        "ALLOWED names file(s) that no longer reference the engine — delete these entries so the \
         ratchet reflects reality:{}",
        render(stale)
    );
}

/// Every entry carries a reason, and no path is listed twice. A blank reason is
/// an allowlist entry that documents nothing, which is what the list exists to
/// prevent.
#[test]
fn allowlist_entries_are_well_formed() {
    let mut seen = BTreeSet::new();
    for (path, _, reason) in ALLOWED {
        assert!(
            !reason.trim().is_empty(),
            "{path} is allowlisted with no reason"
        );
        assert!(
            seen.insert(*path),
            "{path} is listed twice; one entry per file"
        );
    }
}

/// The migration to-do list must be empty, and stay empty by being *worked*
/// rather than re-labelled.
///
/// A [`Verdict::SeamExpressible`] entry says "the seam already covers this and
/// nobody moved it". That is a bug with a known fix, so it fails here rather
/// than sitting in a list nobody reads. Downgrading an entry to
/// [`Verdict::NeedsWiderSeam`] to silence this is the one edit that would make
/// the lint lie — [`no_new_files_call_the_engine_directly`] would still pass,
/// and the gap would vanish from view.
#[test]
fn nothing_is_left_migratable() {
    let pending: Vec<&str> = ALLOWED
        .iter()
        .filter(|(_, verdict, _)| *verdict == Verdict::SeamExpressible)
        .map(|(path, _, _)| *path)
        .collect();
    assert!(
        pending.is_empty(),
        "these files can already be expressed through MemoryProvider and should be migrated: {pending:?}"
    );
}

/// The blocked set is the upstream ask, so it must be non-empty for as long as
/// the engine is still linked — and empty when it is not.
///
/// This is the test that makes "`tinymemory-core` left the build" self-proving:
/// on the day the crate is dropped, [`scan`] returns nothing, `ALLOWED` empties,
/// and this assertion is what forces the module docs above to be rewritten
/// rather than left describing a world that no longer exists.
#[test]
fn the_blocked_set_matches_the_engine_still_being_linked() {
    let blocked = ALLOWED
        .iter()
        .filter(|(_, verdict, _)| *verdict == Verdict::NeedsWiderSeam)
        .count();
    let host_side = ALLOWED
        .iter()
        .filter(|(_, verdict, _)| *verdict == Verdict::HostSide)
        .count();
    assert!(
        blocked > 0 || host_side > 0,
        "nothing references tinymemory-core any more — drop the path dependency from Cargo.toml, \
         remove its cargo-machete `ignored` entry, ratchet scripts/kernel-floor.limits, and rewrite \
         this module's docs (#5560)"
    );
}
