//! Task-local [`AgentProgress`] sink — lets an **in-process embedder** observe a
//! turn it drives through an RPC that only returns a final string.
//!
//! The agent harness already emits everything an embedder needs
//! ([`AgentProgress`], attached with
//! [`Agent::set_on_progress`](crate::openhuman::agent::Agent::set_on_progress)),
//! but RPC entry points such as
//! [`agent_chat`](crate::openhuman::inference::local::ops::agent_chat) build the
//! [`Agent`](crate::openhuman::agent::Agent) internally and drop it, so there is
//! no handle to attach a sink to. A host embedding the core in-process therefore
//! sees nothing at all for the whole turn — no tool calls, no deltas — and can
//! only guess whether a long turn is alive.
//!
//! This module closes that gap with the same shape as
//! [`turn_origin`](crate::openhuman::agent::turn_origin): the embedder scopes a
//! channel around the future it awaits, and the entry point picks it up out of
//! the ambient task-local. Because tokio task-locals are inherited by the future
//! being awaited (not by detached `tokio::spawn`s), the sink reaches exactly the
//! turn the embedder is awaiting and no other — unlike a process-global, which
//! would cross-talk between concurrent turns.
//!
//! ```no_run
//! # async fn demo() {
//! let (tx, mut rx) = tokio::sync::mpsc::channel(256);
//! let turn = openhuman_core::agent_progress::with_progress_sink(tx, async {
//!     // any future that ends up in `agent_chat`
//! });
//! # let _ = (turn, rx.recv());
//! # }
//! ```
//!
//! An entry point that reads the sink must not clobber a sink an existing
//! caller already attached explicitly (web chat, the platform socket, flows,
//! skills all call `set_on_progress` themselves): where both could apply, the
//! **explicitly-set sink wins** and the scoped one is ignored.

use crate::openhuman::agent::progress::AgentProgress;

/// The channel an embedder hands to [`with_progress_sink`], and the same type
/// [`Agent::set_on_progress`](crate::openhuman::agent::Agent::set_on_progress)
/// takes — so a sink read back out of the task-local can be attached directly.
///
/// Use a bounded channel with headroom: the turn loop sends from inside the
/// agent, so a slow or absent receiver applies backpressure to the turn itself.
pub type ProgressSink = tokio::sync::mpsc::Sender<AgentProgress>;

tokio::task_local! {
    /// Per-turn progress sink scoped by an in-process embedder around the
    /// future it awaits. Read by entry points that build an
    /// [`Agent`](crate::openhuman::agent::Agent) internally.
    pub static AGENT_PROGRESS_SINK: ProgressSink;
}

/// Scope `sink` for the duration of `fut`.
///
/// The inner future is `Box::pin`-ed before entering the task-local scope for
/// the same reason [`turn_origin::with_origin`](crate::openhuman::agent::turn_origin::with_origin)
/// does it: the agent loop downstream (tool dispatch, recursive sub-agents, LLM
/// streaming) is deep enough that stacking task-local scopes plus the loop on a
/// 2 MiB worker stack overflows. Box-pinning keeps the combined state machine on
/// the heap for every caller.
pub async fn with_progress_sink<F: std::future::Future>(sink: ProgressSink, fut: F) -> F::Output {
    AGENT_PROGRESS_SINK.scope(sink, Box::pin(fut)).await
}

/// Read the sink scoped by the current task, if any.
///
/// `None` outside any [`with_progress_sink`] scope — which is every existing
/// caller, so behaviour with no sink scoped is unchanged.
pub fn current_progress_sink() -> Option<ProgressSink> {
    AGENT_PROGRESS_SINK.try_with(|s| s.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink() -> (ProgressSink, tokio::sync::mpsc::Receiver<AgentProgress>) {
        tokio::sync::mpsc::channel(8)
    }

    #[tokio::test]
    async fn current_returns_none_outside_scope() {
        assert!(current_progress_sink().is_none());
    }

    #[tokio::test]
    async fn with_progress_sink_scopes_and_unscopes() {
        let (tx, mut rx) = sink();

        let observed = with_progress_sink(tx, async { current_progress_sink() }).await;
        let observed = observed.expect("a sink is visible inside the scope");
        observed
            .send(AgentProgress::TurnStarted)
            .await
            .expect("the scoped sink is the caller's live channel");
        assert!(matches!(rx.recv().await, Some(AgentProgress::TurnStarted)));

        // The scope does not leak past its future.
        assert!(current_progress_sink().is_none());
    }

    #[tokio::test]
    async fn nested_scope_yields_the_inner_sink() {
        let (outer, mut outer_rx) = sink();
        let (inner, mut inner_rx) = sink();

        let observed = with_progress_sink(outer, async {
            with_progress_sink(inner, async { current_progress_sink() }).await
        })
        .await
        .expect("a sink is visible inside the nested scope");

        observed
            .send(AgentProgress::TurnCompleted { iterations: 1 })
            .await
            .expect("send on the innermost sink");
        assert!(matches!(
            inner_rx.recv().await,
            Some(AgentProgress::TurnCompleted { iterations: 1 })
        ));
        // The outer sink saw nothing — the inner scope shadows it.
        assert!(outer_rx.try_recv().is_err());
    }
}
