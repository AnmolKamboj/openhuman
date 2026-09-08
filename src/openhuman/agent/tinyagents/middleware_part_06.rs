// The two middlewares issue #6014 added, split out of `middleware_part_05.rs`
// when that file crossed the 750-line layout limit
// (`scripts/ci/check-openhuman-rust-layout.mjs`). Both are `before_model`
// hooks and both are registered by `assemble_turn_harness`; nothing about the
// split is semantic, and `middleware.rs` includes the parts in order so the
// module contents are unchanged.

// ── FinalCallWrapUpMiddleware (issue #6014) ──────────────────────────────────

/// Turns the **last permitted model call of a capped turn** into the turn's
/// conclusion, in the loop, instead of leaving the answer to an extra call
/// made after the loop has already exited.
///
/// # Why the loop and not afterwards
///
/// A capped turn used to end like this: the loop exits on the model-call cap,
/// and `Agent::summarize_turn_wrapup` then dispatches a second, out-of-band
/// request straight at the `ChatModel` asking for a checkpoint. Being outside
/// the harness, that request ran with **none** of the loop's context
/// management — no microcompact, no compression, no trim — while being built
/// from the largest transcript the turn would ever hold (every tool result a
/// full iteration budget produced). It was therefore the likeliest call of the
/// whole turn to overflow the window, and each of its failure paths returns
/// `("", None)` silently, so the answer degraded to a deterministic digest of
/// tool names exactly when the turn had the most to report. It also bypassed
/// usage accounting (folded back by hand at the call site) and the progress
/// bridge (re-implemented there as buffer-then-validate-then-forward).
///
/// Doing it here removes all of that rather than compensating for it: the
/// concluding call is an ordinary loop iteration, so it inherits the entire
/// middleware stack, its usage rides `UsageCarryMiddleware` like any other
/// call, and its text streams through the normal event bridge. It is also one
/// provider call cheaper — the wrap-up was an extra call *past* the cap the
/// operator configured.
///
/// # What "last permitted call" means, exactly
///
/// The loop records the model call **before** it builds the request
/// (`agent_loop::run_loop`), so by the time `before_model` runs for the Nth
/// call of an N-call budget, `remaining_model_calls()` is already `0`. That is
/// the trigger, and it needs no new plumbing or counter of its own.
///
/// The trade this makes is explicit: a 25-call budget becomes 24 tool rounds
/// plus a conclusion, rather than 25 tool rounds plus a 26th call nobody
/// budgeted for.
///
/// # Why the tools are cleared rather than merely discouraged
///
/// The instruction alone is a request the model may ignore — the out-of-band
/// wrap-up had to re-parse its own response through the dispatcher to catch a
/// model that emitted a tool call anyway. Removing the schemas from the
/// request makes it structural instead: there is nothing to call. `tool_choice`
/// is reset alongside them because a `Required` choice with an empty tool array
/// is a provider 400.
pub(crate) struct FinalCallWrapUpMiddleware {
    /// The synthetic user turn appended on the final call.
    instruction: &'static str,
    /// Every tool call's captured outcome, so the concluding call can be given
    /// back the results microcompact blanked (see `before_model`).
    outcomes: crate::openhuman::agent::tinyagents::ToolOutcomeSink,
    /// Set when the injection fires, so the caller can report the turn as
    /// capped. Necessary because this turn now ends *naturally* — the model
    /// returns text and requests no tools, which is the loop's ordinary
    /// terminal condition — so the old `final_response.is_none()` tell no
    /// longer distinguishes a capped turn from a finished one.
    fired: Arc<std::sync::atomic::AtomicBool>,
}

impl FinalCallWrapUpMiddleware {
    pub(crate) fn new(
        instruction: &'static str,
        outcomes: crate::openhuman::agent::tinyagents::ToolOutcomeSink,
    ) -> Self {
        Self {
            instruction,
            outcomes,
            fired: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// The shared flag, for the run loop to read after the drive future returns.
    pub(crate) fn fired(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.fired.clone()
    }
}

#[async_trait]
impl Middleware<()> for FinalCallWrapUpMiddleware {
    fn name(&self) -> &str {
        "final_call_wrap_up"
    }

    async fn before_model(
        &self,
        ctx: &mut RunContext<()>,
        _state: &(),
        request: &mut ModelRequest,
    ) -> TaResult<()> {
        if ctx.limits.remaining_model_calls() > 0 {
            return Ok(());
        }
        // A budget of one call would make the very first call the concluding
        // one, so the turn could never run a tool at all. That is a
        // misconfiguration rather than a cap being reached, and silently
        // answering it with a "you have run out of tool calls" instruction
        // would misreport it — leave such a run alone.
        if ctx.limits.limits().max_model_calls <= 1 {
            return Ok(());
        }
        tracing::info!(
            model_calls = ctx.limits.model_calls(),
            max_model_calls = ctx.limits.limits().max_model_calls,
            tools_withdrawn = request.tools.len(),
            "[tinyagents::mw] final permitted model call — withdrawing tools and asking for the \
             turn's conclusion"
        );
        request.tools.clear();
        request.tool_choice = tinyinference::model::ToolChoice::None;
        // Give the concluding call back the results microcompact blanked.
        //
        // `MicrocompactMiddleware` replaces every tool-result body past the
        // most recent `keep_recent` (5, by default) with `CLEARED_PLACEHOLDER`,
        // and — constructed without a token budget — it does so on every call,
        // not only under context pressure. That is right for an intermediate
        // call, which needs recent context to choose the next tool and nothing
        // more. It is exactly wrong for this one: a turn that spent 24 rounds
        // gathering would be asked to report its findings with 19 rounds of
        // them replaced by "[Old tool result content cleared]", which is the
        // same empty-handed answer this whole mechanism exists to prevent,
        // arrived at from the other direction.
        //
        // Restored from the captured outcomes rather than by exempting the
        // turn from microcompact, because the blanking has already happened by
        // the time this runs (registration order: microcompact is installed by
        // `context_mw.install`, this middleware immediately after it) and
        // because the sink is the honest source — it holds each result as it
        // entered the transcript, after the per-result byte cap.
        //
        // Only a body that IS the placeholder is replaced, so a result the
        // model legitimately saw in full is never rewritten.
        //
        // This deliberately runs BEFORE the compression and trim middlewares,
        // which are installed after it: restoring can make the request large,
        // and those two are what bound it. The resulting degradation ladder is
        // the one this call wants — everything when it fits, an LLM summary of
        // the older slice when it does not, and oldest-first eviction only in
        // extremis. What it never does again is silently blank the middle.
        let restored = match self.outcomes.lock() {
            Ok(outcomes) => {
                let mut restored = 0usize;
                for message in request.messages.iter_mut() {
                    let TaMessage::Tool(tool) = message else {
                        continue;
                    };
                    if tool.content.iter().any(|block| !matches!(block, ContentBlock::Text(_))) {
                        continue;
                    }
                    let body: String = tool
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text(text) => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    if body.trim() != CLEARED_PLACEHOLDER {
                        continue;
                    }
                    let Some(outcome) = self_outcome_for(&outcomes, &tool.tool_call_id) else {
                        continue;
                    };
                    if outcome.trim().is_empty() {
                        continue;
                    }
                    tool.content = vec![ContentBlock::Text(outcome)];
                    restored += 1;
                }
                restored
            }
            Err(_) => {
                tracing::warn!(
                    "[tinyagents::mw] tool-outcome sink poisoned; concluding without restoring \
                     cleared tool results"
                );
                0
            }
        };
        if restored > 0 {
            tracing::info!(
                restored,
                "[tinyagents::mw] restored cleared tool results for the concluding call"
            );
        }
        request
            .messages
            .push(TaMessage::user(self.instruction.to_string()));
        self.fired
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

/// The captured content for one tool call id, if the sink holds it.
///
/// A free function so the borrow of the locked sink stays scoped to the lookup
/// rather than being held across the mutation of `request.messages`.
fn self_outcome_for(
    outcomes: &[crate::openhuman::agent::tinyagents::ToolCallOutcome],
    call_id: &str,
) -> Option<String> {
    outcomes
        .iter()
        .find(|outcome| outcome.call_id == call_id)
        .map(|outcome| outcome.content.clone())
}

// ── ArtifactIndexTocMiddleware (issue #6014) ─────────────────────────────────

/// Namespace `ToolOutputMiddleware` writes each persisted artifact under.
const ARTIFACT_INDEX_NAMESPACE: &str = "tool_results";

/// Renders the run's persisted-artifact index into the request as a short
/// contents list, so the model can always see what this turn has gathered and
/// where the full copy lives.
///
/// # The reference used to live somewhere it could be destroyed
///
/// A tool result over the per-result budget is written to disk and replaced by
/// a preview plus an `artifact_path` pointer (`apply_per_result_persistence`).
/// That pointer's only home was the tool-result message itself — and tool-result
/// messages are exactly what the two reduction steps act on. Microcompact
/// replaces a body with `CLEARED_PLACEHOLDER`; compression folds the older slice
/// into a summary that *should* carry paths forward ("prefer concrete facts —
/// paths, names, values") but is a model call, not a guarantee.
///
/// Either way the outcome is the same and it is silent: the data sits intact on
/// disk with nothing in context saying it exists. Every component did its job —
/// the result was persisted, the transcript was reduced — and the turn quietly
/// lost the ability to reach its own findings. No log fires, because nothing
/// failed.
///
/// The index has been maintained all along (`ToolResultArtifactIndexStore`,
/// registered on `RunContext.stores`, written on every persist). Nothing read
/// it. This reads it.
///
/// # Why re-rendered rather than injected once
///
/// The contents list is built fresh into each request and never enters the
/// loop's own transcript, which is what makes it un-destroyable rather than
/// merely durable: there is no stored copy for a later pass to blank, fold or
/// evict, and it cannot accumulate into a stack of stale lists. It is also
/// always current — an artifact persisted on the previous round appears on the
/// next call with no bookkeeping.
///
/// Emitted as a **system** message for the same reason: compression keeps every
/// system message verbatim and the trim never drops one, so the one thing that
/// says where the data went is the one thing the ladder may not take. It is
/// appended at the tail rather than the head so the cacheable prompt prefix is
/// untouched.
pub(crate) struct ArtifactIndexTocMiddleware;

#[async_trait]
impl Middleware<()> for ArtifactIndexTocMiddleware {
    fn name(&self) -> &str {
        "artifact_index_toc"
    }

    async fn before_model(
        &self,
        ctx: &mut RunContext<()>,
        _state: &(),
        request: &mut ModelRequest,
    ) -> TaResult<()> {
        let Some(store) = ctx
            .stores
            .get(crate::openhuman::agent::harness::tool_result_artifacts::TINYAGENTS_TOOL_RESULT_ARTIFACT_STORE)
        else {
            return Ok(());
        };
        let keys = store
            .list(ARTIFACT_INDEX_NAMESPACE)
            .await
            .unwrap_or_default();
        if keys.is_empty() {
            // No result has been offloaded, so there is nothing to point at and
            // no reason to spend context saying so.
            return Ok(());
        }

        let mut rows: Vec<String> = Vec::new();
        for key in &keys {
            let Ok(Some(entry)) = store.get(ARTIFACT_INDEX_NAMESPACE, key).await else {
                continue;
            };
            let tool = entry
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool");
            let Some(path) = entry.get("artifact_path").and_then(|v| v.as_str()) else {
                continue;
            };
            let bytes = entry
                .get("original_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            rows.push(format!("- `{tool}` → `{path}` ({bytes} bytes)"));
        }
        if rows.is_empty() {
            return Ok(());
        }
        // Sorted so the list is stable between calls when nothing was added —
        // the index is a `HashMap`, whose iteration order is not. An unstable
        // ordering would rewrite this message on every call for no reason,
        // costing cache and making the diff unreadable in a trace.
        rows.sort();
        let count = rows.len();
        tracing::debug!(
            artifacts = count,
            "[tinyagents::mw] rendering the persisted-artifact contents list"
        );
        request.messages.push(TaMessage::system(format!(
            "## Stored results from this turn\n\n\
             {count} tool result(s) were too large to keep inline and were written to disk. The \
             text you saw for them is a preview; the full content is at the path below and can be \
             read with the file-reading tool when you need detail the preview does not carry.\n\n\
             {}",
            rows.join("\n")
        )));
        Ok(())
    }
}
