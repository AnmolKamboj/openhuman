//! Content store for memory-tree chunk and summary `.md` files (Phase MD-content).
//!
//! Bodies are stored on disk as `.md` files with YAML front-matter.
//! SQLite holds `content_path` (relative, forward-slash) and `content_sha256`
//! (over body bytes only) as pointers + integrity tokens.
//!
//! ## Module layout
//!
//! - [`paths`]   — path generation + `slugify_source_id` + summary path builders
//! - [`compose`] — YAML front-matter + body composition; tag rewriting
//! - [`atomic`]  — tempfile+fsync+rename writes; SHA-256; `stage_summary`
//! - [`read`]    — read + SHA-256 verification + `split_front_matter`; summary variants
//! - [`tags`]    — `update_chunk_tags` + `update_summary_tags` + slugifiers
//! - `obsidian` / `obsidian_registry` / `wiki_git` — on-disk content formats,
//!   owned by TinyCortex and re-exported here (see the `pub use` below)

pub mod read;
pub mod tags;

pub use tinycortex::memory::chunks::StagedChunk;
/// The git-backed wiki content format. Re-exported only when `memory-git` is
/// on: it lives behind tinycortex's `wiki-git` feature, which the gate carries
/// along with `git-diff` and the libgit2 cohort.
#[cfg(feature = "memory-git")]
pub use tinycortex::memory::store::content::wiki_git;
pub use tinycortex::memory::store::content::{
    atomic, compose, obsidian, obsidian_registry, paths, raw, stage_chunks, StagedSummary,
    SummaryComposeInput, SummaryTreeKind,
};

/// Update the `tags:` block in a summary's on-disk `.md` file after an
/// extraction job runs.
///
/// Delegates to [`tags::update_summary_tags`].
pub fn update_summary_tags(
    config: &crate::openhuman::config::Config,
    summary_id: &str,
) -> anyhow::Result<()> {
    tags::update_summary_tags(config, summary_id)
}
