//! Migration 3 → 4: expand autonomy defaults for existing users.
//!
//! PR #2500 expanded the code defaults for `autonomy.allowed_commands` and
//! `autonomy.auto_approve`, and changed `max_actions_per_hour` from 20 to
//! `u32::MAX` (effectively unlimited). Existing users had the old values
//! persisted in their `config.toml` at `schema_version = 3`, so they did
//! **not** pick up the new defaults automatically — their on-disk values
//! shadow the code defaults.
//!
//! ## What this migration does
//!
//! 1. **Merges new commands** into `config.autonomy.allowed_commands`. Only
//!    commands not already present are added, so additional entries a user has
//!    made are preserved.
//!
//!    ⚠️ **A deliberate removal is NOT preserved, and cannot be.** The merge is
//!    additive and has no record of what the v3 default contained, so it cannot
//!    tell "absent because the user removed it" from "absent because it
//!    post-dates this config". A narrowed `allowed_commands` is therefore
//!    widened back. This doc previously claimed removals were "fully
//!    preserved"; that was false in the one direction that matters for a
//!    command allowlist.
//!
//!    This bites hardest on a **hand-written** `config.toml`: `schema_version`
//!    is `#[serde(default)]`, so a file without the field loads as version `0`
//!    and runs the whole 0→10 chain including this migration — even though it
//!    was never a v3 config. A config this code writes is stamped with
//!    `CURRENT_SCHEMA_VERSION` on creation (`schema/load/impl_load.rs`), so the
//!    fresh-install path is unaffected. Set `schema_version` explicitly to opt
//!    out.
//! 2. **Merges new auto-approve tools** into `config.autonomy.auto_approve`
//!    with the same additive-only merge logic.
//! 3. **Bumps `max_actions_per_hour`** from 20 (the old hard-coded default)
//!    to `u32::MAX` only when the persisted value is exactly 20. Users who
//!    deliberately set a different limit are left untouched.
//!
//! ## Idempotency
//!
//! - Gated externally by [`Config::schema_version`] (`== 3`). Once the bump
//!   to version 4 is persisted, future launches skip this migration entirely.
//! - Internally idempotent: merging already-present items is a no-op because
//!   the merge logic guards every insert with a `contains` check.

use crate::openhuman::config::Config;

/// The old hard-coded default for `max_actions_per_hour`. When this exact
/// value is still persisted, we assume it was never deliberately customised
/// and bump it to the new unlimited sentinel.
const OLD_DEFAULT_MAX_ACTIONS_PER_HOUR: u32 = 20;

/// Commands to merge into persisted `allowed_commands` during the v3→v4 bump.
///
/// The target set mirrors the current default more closely than old v3 configs.
/// Some entries may already be present for customized users; the migration is
/// additive and skips duplicates.
const NEW_COMMANDS: &[&str] = &[
    "pnpm", "yarn", "make", "cmake", "sort", "uniq", "diff", "which", "uname", "basename",
    "dirname", "tr", "cut", "realpath", "readlink", "stat", "file", "mkdir", "touch", "cp", "mv",
    "ln", "date", "dir", "type", "where", "findstr", "more",
];

/// New auto-approve tools to merge into `auto_approve`.
///
/// These were added to the code default in PR #2500 but are absent from any
/// `config.toml` written before that change.
const NEW_AUTO_APPROVE_TOOLS: &[&str] = &["glob", "grep"];

/// Counters returned by [`run`] for diagnostics. Logged at INFO once per
/// successful migration run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationStats {
    /// Number of commands added to `allowed_commands`.
    pub commands_added: usize,
    /// Number of tools added to `auto_approve`.
    pub tools_added: usize,
    /// `true` when `max_actions_per_hour` was bumped from 20 to `u32::MAX`.
    pub max_actions_bumped: bool,
}

/// Run the autonomy defaults expansion migration on the given `Config`.
///
/// Synchronous — pure config mutation, no I/O. The caller
/// (`migrations::run_pending`) persists the result via `Config::save()` and
/// bumps `schema_version`.
pub fn run(config: &mut Config) -> anyhow::Result<MigrationStats> {
    let mut stats = MigrationStats::default();

    log::debug!(
        "[migrations][expand-autonomy-defaults] starting \
         allowed_commands.len={} auto_approve.len={} max_actions_per_hour={}",
        config.autonomy.allowed_commands.len(),
        config.autonomy.auto_approve.len(),
        config.autonomy.max_actions_per_hour,
    );

    // Merge new commands (additive only — never remove user entries).
    let commands_before = config.autonomy.allowed_commands.clone();
    let mut added_commands: Vec<&str> = Vec::new();
    for &cmd in NEW_COMMANDS {
        if !config.autonomy.allowed_commands.iter().any(|c| c == cmd) {
            log::debug!(
                "[migrations][expand-autonomy-defaults] adding command={:?} to allowed_commands",
                cmd
            );
            config.autonomy.allowed_commands.push(cmd.to_string());
            added_commands.push(cmd);
            stats.commands_added += 1;
        }
    }

    // Widening a command allowlist is a security-relevant change, and until now
    // it happened at DEBUG — invisible on a default log level, and invisible in
    // the file on disk, which still shows the narrow list the user wrote. Say it
    // at WARN, and name what was added, whenever the list being widened was not
    // empty: an empty list is a config that never expressed an opinion, but a
    // non-empty one is a list somebody chose.
    if !added_commands.is_empty() && !commands_before.is_empty() {
        log::warn!(
            "[migrations][expand-autonomy-defaults] widened a non-empty allowed_commands \
             from {} to {} entries; added: {}. If this list was curated deliberately, the \
             removals are NOT preserved — set `schema_version` in config.toml to opt out \
             of this migration.",
            commands_before.len(),
            config.autonomy.allowed_commands.len(),
            added_commands.join(", ")
        );
    }

    // Merge new auto-approve tools (additive only).
    for &tool in NEW_AUTO_APPROVE_TOOLS {
        if !config.autonomy.auto_approve.iter().any(|t| t == tool) {
            log::debug!(
                "[migrations][expand-autonomy-defaults] adding tool={:?} to auto_approve",
                tool
            );
            config.autonomy.auto_approve.push(tool.to_string());
            stats.tools_added += 1;
        }
    }

    // Bump max_actions_per_hour only when it still holds the old default.
    // Users who deliberately configured a different ceiling keep their value.
    if config.autonomy.max_actions_per_hour == OLD_DEFAULT_MAX_ACTIONS_PER_HOUR {
        log::info!(
            "[migrations][expand-autonomy-defaults] bumping max_actions_per_hour \
             {} -> u32::MAX (old hard-coded default, PR #2500 changed code default)",
            OLD_DEFAULT_MAX_ACTIONS_PER_HOUR
        );
        config.autonomy.max_actions_per_hour = u32::MAX;
        stats.max_actions_bumped = true;
    } else {
        log::debug!(
            "[migrations][expand-autonomy-defaults] max_actions_per_hour={} — \
             not the old default, leaving unchanged",
            config.autonomy.max_actions_per_hour
        );
    }

    log::info!(
        "[migrations][expand-autonomy-defaults] done \
         commands_added={} tools_added={} max_actions_bumped={}",
        stats.commands_added,
        stats.tools_added,
        stats.max_actions_bumped,
    );

    Ok(stats)
}

#[cfg(test)]
#[path = "expand_autonomy_defaults_tests.rs"]
mod tests;
