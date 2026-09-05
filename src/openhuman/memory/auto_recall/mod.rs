//! Lane C — a gated, bounded pre-turn recall of facts about the user (#6040).
//!
//! The chat turn assembles a small per-turn context that rides on the user
//! message, never on the cached system-prompt prefix. Lane B, situational
//! preferences, has always been there. This lane sits beside it and answers the
//! one question the model could not answer on its own: *does memory hold
//! something about the user that this message is asking for?* Without it,
//! "who is my idol and why?" was answered from prompt history alone, and the
//! model — never told to look — replied "not stored" while the fact sat in the
//! memory tree.
//!
//! # Why this is not the lane that was removed
//!
//! `core_turn.rs` documents the old `memory_loader.load_context()` block: two
//! full scans of the `global` namespace on every turn, for at most nine lines,
//! most of them empty once ordinary chat crowded the ranking. This lane is a
//! different cost class on every axis that mattered there:
//!
//! - **It runs on a minority of turns.** [`gate_decision`] opens only for a
//!   message that asks about the user — first-person ownership plus a question
//!   or request shape. Small talk, code, weather and pastes never reach the
//!   store.
//! - **It walks the tree, not the pile.** [`GuardSource`] calls the bound
//!   driver's `fast_retrieve` through the guard: summary-first, and on an
//!   entity-less query the engine's dense path, with a limit of
//!   [`AUTO_RECALL_LIMIT`]. The work is bounded by the answer, not by
//!   everything the user ever said.
//! - **It is floored, capped and budgeted.** Hits below
//!   [`AUTO_RECALL_RELATIVE_FLOOR`] of the best score are dropped (retrieval
//!   scores are declared non-comparable across drivers, so the floor is
//!   relative, not absolute); at most [`AUTO_RECALL_LIMIT`] survive, each
//!   clipped to [`AUTO_RECALL_PER_HIT_CHARS`]; the whole block is clipped to
//!   the guard's `recall_max_chars`; and the lookup is abandoned after
//!   [`AUTO_RECALL_BUDGET`], so a memory module still downloading on a cold
//!   launch cannot stall the turn.
//!
//! # Switch
//!
//! `[subsystems.memory.hooks] auto_recall` (env
//! `OPENHUMAN_MEMORY_HOOKS_AUTO_RECALL`) is the kill-switch. The flag existed
//! before this lane — declared, parsed, and read by nothing — so turning the
//! lane off is a config edit on any install, with no new key to learn.
//!
//! # Failure mode
//!
//! Every failure is an ordinary turn without a block: a driver without the
//! retrieval family, a retrieval error, a budget timeout. None of them is an
//! error to the user, and each leaves an `[auto_recall]` line behind so the
//! thresholds can be tuned from real logs.

mod gate;
mod source;
pub mod warm;

pub use gate::{gate_decision, GateDecision};
pub use source::{AutoRecallSource, GuardSource};

use crate::openhuman::memory::api::provider::retrieval::{FastRetrieveQuery, RetrievalHit};
use crate::openhuman::memory::guard::MemoryGuard;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Most hits a block may carry. Three is enough to answer a question about the
/// user and small enough that a wrong guess costs a few lines, not a screen.
pub const AUTO_RECALL_LIMIT: usize = 3;

/// Characters kept per hit. A tree leaf is a chunk, not a sentence; the model
/// needs the fact, not the whole page it came from.
pub const AUTO_RECALL_PER_HIT_CHARS: usize = 400;

/// Characters kept of a hit's scope label (`folder:profile`, `slack:#eng`).
pub const AUTO_RECALL_SCOPE_CHARS: usize = 80;

/// How long the turn waits for the lookup before proceeding without it.
///
/// Wider than Lane B's 3 s on purpose. A warm `fast_retrieve` through the
/// module measured 2–3 s on a loaded 16 GB desktop (field test, 2026-09-05),
/// and at 3 s one of three real questions lost its block to the timeout — the
/// turn then answers "not stored" for a fact that is stored, which is the bug
/// this lane exists to fix. The lane only runs on gated turns, and the two
/// lanes run side by side, so the worst case a turn pays is this bound alone.
pub const AUTO_RECALL_BUDGET: Duration = Duration::from_secs(5);

/// A hit is kept only while its score is at least this fraction of the best
/// hit's. Scores are "higher is better" and nothing more across drivers, so an
/// absolute threshold would be a guess; a relative one drops the long tail of
/// weak matches without pretending to know the scale.
pub const AUTO_RECALL_RELATIVE_FLOOR: f32 = 0.5;

/// The banner that heads the injected block. Tests and the prompt snapshot
/// look for it; the model reads it as the section title.
pub const AUTO_RECALL_BANNER: &str = "## Relevant memory for this message";

/// The lane itself: a retrieval source, the switch, and the budgets.
pub struct AutoRecall {
    source: Arc<dyn AutoRecallSource>,
    enabled: bool,
    recall_max_chars: Option<usize>,
    budget: Duration,
}

impl AutoRecall {
    /// A lane over `source`. `enabled = false` makes [`Self::block_for`]
    /// answer `None` without touching the source; `recall_max_chars` clips the
    /// rendered block (the guard's `recall_max_chars` budget).
    pub fn new(
        source: Arc<dyn AutoRecallSource>,
        enabled: bool,
        recall_max_chars: Option<usize>,
    ) -> Self {
        Self {
            source,
            enabled,
            recall_max_chars,
            budget: AUTO_RECALL_BUDGET,
        }
    }

    /// The production lane: reads through `guard`, switched by the guard
    /// policy's `hooks.auto_recall`, clipped by its `recall_max_chars`.
    pub fn from_guard(guard: Arc<MemoryGuard>) -> Self {
        let (enabled, recall_max_chars) = {
            let policy = guard.policy();
            (policy.hooks().auto_recall, policy.recall_budget())
        };
        log::debug!(
            "[auto_recall] lane bound driver={} enabled={enabled} recall_max_chars={recall_max_chars:?}",
            guard.policy().driver_id()
        );
        Self::new(Arc::new(GuardSource::new(guard)), enabled, recall_max_chars)
    }

    /// Overrides the lookup budget. Tests use it to make the timeout path
    /// reachable without sleeping for seconds.
    #[must_use]
    pub fn with_budget(mut self, budget: Duration) -> Self {
        self.budget = budget;
        self
    }

    /// Whether the lane will run at all.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The block to prepend to `user_message`, or `None` when the lane is off,
    /// the gate is closed, nothing relevant came back, or the lookup failed or
    /// timed out. Never an error: a turn without a block is an ordinary turn.
    pub async fn block_for(&self, user_message: &str) -> Option<String> {
        if !self.enabled {
            log::debug!("[auto_recall] disabled by hooks.auto_recall; skipping");
            return None;
        }
        let decision = gate_decision(user_message);
        let GateDecision::Open(reason) = decision else {
            log::debug!(
                "[auto_recall] gate=closed reason={} chars={}",
                decision.reason(),
                user_message.chars().count()
            );
            return None;
        };

        let started = Instant::now();
        let query = FastRetrieveQuery {
            limit: AUTO_RECALL_LIMIT,
            ..FastRetrieveQuery::default()
        };
        let response =
            match tokio::time::timeout(self.budget, self.source.fast_retrieve(user_message, query))
                .await
            {
                Ok(Ok(response)) => response,
                Ok(Err(err)) => {
                    log::warn!(
                        "[auto_recall] gate=open reason={reason} retrieval failed after {}ms; \
                     continuing without a memory block: {err}",
                        started.elapsed().as_millis()
                    );
                    return None;
                }
                Err(_elapsed) => {
                    log::warn!(
                        "[auto_recall] gate=open reason={reason} retrieval exceeded {:?}; \
                     continuing without a memory block",
                        self.budget
                    );
                    return None;
                }
            };

        let total = response.total;
        let hits = select_hits(response.hits);
        if hits.is_empty() {
            log::info!(
                "[auto_recall] gate=open reason={reason} hits=0 total={total} elapsed_ms={}",
                started.elapsed().as_millis()
            );
            return None;
        }
        let block = render_block(&hits, self.recall_max_chars);
        log::info!(
            "[auto_recall] gate=open reason={reason} hits={} total={total} elapsed_ms={} chars={}",
            hits.len(),
            started.elapsed().as_millis(),
            block.chars().count()
        );
        Some(block)
    }
}

/// Ranks `hits` best-first, drops empty content and the weak tail below the
/// relative floor, and keeps at most [`AUTO_RECALL_LIMIT`].
pub(crate) fn select_hits(mut hits: Vec<RetrievalHit>) -> Vec<RetrievalHit> {
    hits.retain(|hit| !hit.content.trim().is_empty());
    // Best first; a NaN score sorts last rather than poisoning the order.
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    if let Some(top) = hits.first().map(|hit| hit.score) {
        if top > 0.0 {
            let floor = top * AUTO_RECALL_RELATIVE_FLOOR;
            hits.retain(|hit| hit.score >= floor);
        }
    }
    hits.truncate(AUTO_RECALL_LIMIT);
    hits
}

/// The block the turn prepends: the banner, one line per hit, clipped to
/// `recall_max_chars` when the guard sets one.
pub(crate) fn render_block(hits: &[RetrievalHit], recall_max_chars: Option<usize>) -> String {
    let mut block = String::from(AUTO_RECALL_BANNER);
    block.push_str("\n\n");
    for hit in hits {
        block.push_str("- ");
        block.push_str(&one_line(&hit.content, AUTO_RECALL_PER_HIT_CHARS));
        // The scope is driver metadata (`folder:profile`, `slack:#eng`), but
        // it lands in the prompt like the content does, so it gets the same
        // one-line treatment and a short cap rather than a trusted pass-through.
        let scope = one_line(&hit.tree_scope, AUTO_RECALL_SCOPE_CHARS);
        if !scope.is_empty() {
            block.push_str(" (from ");
            block.push_str(&scope);
            block.push(')');
        }
        block.push('\n');
    }
    block.push('\n');
    if let Some(max_chars) = recall_max_chars {
        if block.chars().count() > max_chars {
            block = block.chars().take(max_chars).collect();
            block.push_str("…\n\n");
        }
    }
    block
}

/// `text` with its whitespace collapsed onto one line and clipped to
/// `max_chars`, so a multi-paragraph chunk stays one bullet.
fn one_line(text: &str, max_chars: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = collapsed.chars().take(max_chars).collect();
    if collapsed.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
#[path = "auto_recall_tests.rs"]
mod tests;
