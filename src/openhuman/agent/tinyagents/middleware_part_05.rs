
/// Inference/delegation **envelope** markers that prove a tool result came from a
/// delegated inference call (a sub-agent / provider round-trip) rather than from
/// arbitrary tool stderr. Every marker here is harness-generated (our own
/// reliable-chain rollup or sub-agent dispatch wrapper), NOT a provider HTTP body
/// that arbitrary tool stderr could forge. Ported from legacy `tool_loop`.
const INFERENCE_FAILURE_ENVELOPE_MARKERS: &[&str] = &[
    // Reliable-chain exhaustion rollup (reliable.rs::format_failure_aggregate).
    "all providers/models failed",
    "may not be available on your provider",
    // Sub-agent delegation failure wrapper (dispatch.rs::format_subagent_failure).
    "failed and did not complete",
];

/// True if `result` carries one of the inference/delegation envelope markers —
/// i.e. the failure demonstrably came from a delegated provider round-trip, not
/// arbitrary tool stderr. See [`INFERENCE_FAILURE_ENVELOPE_MARKERS`].
fn has_inference_failure_envelope(result: &str) -> bool {
    let lower = result.to_ascii_lowercase();
    INFERENCE_FAILURE_ENVELOPE_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Recognize a permanent (non-retryable) delegated-inference failure from a tool
/// result. Two-stage gate so a *recoverable* tool failure can't be misclassified:
/// (1) the result must carry a delegated-inference envelope
/// ([`has_inference_failure_envelope`]); (2) the trusted body is matched against
/// the two tight provider classifiers. Budget takes precedence if both match.
/// Ported from legacy `tool_loop::terminal_inference_failure_kind` (#3104).
pub(crate) fn terminal_inference_failure_kind(result: &str) -> Option<TerminalInferenceFailure> {
    use crate::openhuman::inference::provider::{
        is_budget_exhausted_message, is_provider_config_rejection_message,
    };
    if !has_inference_failure_envelope(result) {
        return None;
    }
    if is_budget_exhausted_message(result) {
        Some(TerminalInferenceFailure::BudgetExhausted)
    } else if is_provider_config_rejection_message(result) {
        Some(TerminalInferenceFailure::ProviderConfig)
    } else {
        None
    }
}

/// The actionable root-cause halt summary for a terminal delegated-inference
/// failure. Ported verbatim from the legacy loop.
fn terminal_inference_halt_summary(
    kind: TerminalInferenceFailure,
    tool: &str,
    result: &str,
) -> String {
    match kind {
        TerminalInferenceFailure::BudgetExhausted => format!(
            "Stopping: the `{tool}` step failed because the account is out of inference \
             budget/credits — every retry hits the same wall. Add credits to your account \
             (or, when using a custom/BYO provider, top up that provider's own account) and try \
             again. Details:\n{}",
            truncate_for_halt(result),
        ),
        TerminalInferenceFailure::ProviderConfig => format!(
            "Stopping: the `{tool}` step failed because the configured model/provider rejected the \
             request (e.g. an unknown model, a non-chat/embedding model, a missing credential, or \
             a region block) — retrying will not help. Fix the model or API key in Connections → API keys → LLM. \
             Details:\n{}",
            truncate_for_halt(result),
        ),
    }
}

/// Halt summary when a single recoverable `(tool, args)` call exhausts its
/// extended identical-retry headroom. Ported from the legacy loop.
fn recoverable_identical_halt_summary(tool: &str, count: u32, result: &str) -> String {
    format!(
        "Stopping: the `{tool}` call was retried {count} times with identical arguments and kept \
         failing — repeating it will not help. Last error:\n{}\n\nThis looked recoverable at \
         first, but the same call exhausted the extended transient-failure headroom. Report this \
         back instead of retrying.",
        truncate_for_halt(result),
    )
}

/// Halt summary when many recoverable-looking failures pile up with no progress.
/// Ported from the legacy loop.
fn recoverable_no_progress_halt_summary(consecutive: u32, tool: &str, result: &str) -> String {
    format!(
        "Stopping: {consecutive} recoverable-looking tool failures happened in a row with no \
         successful progress. Last error (from `{tool}`):\n{}\n\nThe turn is still bounded by the \
         iteration/cost limits, but this many consecutive transient failures means the goal is not \
         currently reachable. Report this back instead of retrying.",
        truncate_for_halt(result),
    )
}

/// Tools whose contract is to be re-invoked with identical arguments, so an
/// identical repeat is legitimate progress — not a no-progress loop. Today this
/// is `wait_subagent`, which polls a running async sub-agent and explicitly tells
/// the model to "call wait_subagent again" when a `timeout_secs` window elapses
/// while the sub-agent is still running. Without this exemption a task that
/// outlives two wait windows would have its third identical `wait_subagent`
/// halted by the no-progress breakers before it could collect the eventual
/// result. Ported from legacy `tool_loop::is_repeat_call_exempt` (Codex P1 on #4230).
pub(crate) fn is_repeat_call_exempt(tool: &str) -> bool {
    matches!(tool, "wait_subagent")
}

/// Extract the assistant's visible text (concatenated [`ContentBlock::Text`]
/// blocks) from a model response message, for the repeat-output signature.
fn assistant_visible_text(message: &tinyinference::message::AssistantMessage) -> String {
    let mut out = String::new();
    for block in &message.content {
        if let ContentBlock::Text(t) = block {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(t);
        }
    }
    out
}

/// Per-batch state the repeat-CALL guard needs but can only fully evaluate once
/// every tool result in the assistant's batch has come back: the canonical
/// `(tool, args)` signature captured at `after_model`, plus the running
/// success/remaining accounting folded in at each `after_tool`.
#[derive(Default)]
struct PendingCallBatch {
    /// Canonical `(tool, args)` signature of the batch, from `after_model`.
    call_sig: String,
    /// Tool results still outstanding for this batch.
    remaining: usize,
    /// `true` while every result so far in the batch has succeeded.
    all_ok: bool,
    /// `true` when every call in the batch is a polling/wait exemption.
    exempt: bool,
}

/// Host adapter for the crate's successful-repeat tracker (#4088 / #4095).
/// [`SuccessfulRepeatTracker`] owns the generic streak accounting; this adapter
/// builds canonical OpenHuman tool signatures, applies the product polling-tool
/// exemption, and maps a crate halt verdict into the shared halt summary and
/// steering pause:
///
/// - **Repeat-output** (`after_model`, checked before the tools run): halts when
///   the assistant's visible text + tool-call `(name, args)` batch is byte
///   identical [`DEFAULT_REPEAT_OUTPUT_THRESHOLD`] iterations in a row.
/// - **Repeat-call** (evaluated once the batch's tool results are all back, gated
///   on every call succeeding): halts when the `(tool, args)` batch alone repeats
///   [`DEFAULT_REPEAT_CALL_THRESHOLD`] times — catching successful no-op loops
///   that vary only their narration.
///
/// Polling/wait tools ([`is_repeat_call_exempt`]) are exempt from both: their
/// contract is to be re-invoked identically, so an all-poll batch resets the
/// streaks instead of recording. On a trip it writes the legacy root-cause
/// summary into the shared [`HaltSummarySlot`](super::HaltSummarySlot) and pauses
/// the run through the shared steering handle — the same halt mechanism as the
/// repeated-failure breaker.
pub(crate) struct RepeatProgressMiddleware {
    handle: SteeringHandle,
    halt_summary: super::HaltSummarySlot,
    tracker: SuccessfulRepeatTracker,
    /// Batch bookkeeping bridging `after_model` → `after_tool` for the call guard.
    pending: std::sync::Mutex<Option<PendingCallBatch>>,
}

impl RepeatProgressMiddleware {
    pub(crate) fn new(handle: SteeringHandle, halt_summary: super::HaltSummarySlot) -> Self {
        Self {
            handle,
            halt_summary,
            tracker: SuccessfulRepeatTracker::default(),
            pending: std::sync::Mutex::new(None),
        }
    }

    /// Latch a root-cause halt: record the summary the turn surfaces instead of an
    /// empty/last-model reply, and pause at the top of the next iteration (before
    /// the next model call), matching the repeated-failure breaker's halt path.
    fn halt(&self, summary: String) {
        if let Ok(mut slot) = self.halt_summary.lock() {
            *slot = Some(summary);
        }
        self.handle.send(SteeringCommand::Pause);
    }
}

#[async_trait]
impl Middleware<()> for RepeatProgressMiddleware {
    fn name(&self) -> &str {
        "repeat_progress"
    }

    async fn after_model(
        &self,
        _ctx: &mut RunContext<()>,
        _state: &(),
        response: &mut ModelResponse,
    ) -> TaResult<()> {
        let tool_calls = &response.message.tool_calls;
        if tool_calls.is_empty() {
            // A final answer (no tool calls) ends the loop; nothing to guard, and
            // there is no batch to track for the call guard.
            if let Ok(mut pending) = self.pending.lock() {
                *pending = None;
            }
            return Ok(());
        }

        // Polling/wait tools are contractually re-invoked with identical args +
        // narration each timeout while the work is still running, so an all-poll
        // batch is legitimate progress, not a no-progress repeat.
        let all_exempt = tool_calls.iter().all(|c| is_repeat_call_exempt(&c.name));

        // Canonical `(tool, args)` batch signature (call guard) and the broader
        // narration+call signature (output guard). Both fold each call in order
        // with a `\u{1}` separator, matching the legacy signatures.
        let mut call_sig = String::new();
        for call in tool_calls {
            call_sig.push('\u{1}');
            call_sig.push_str(&call.name);
            call_sig.push('\u{1}');
            call_sig.push_str(&call.arguments.to_string());
        }
        let output_sig = format!(
            "{}{}",
            assistant_visible_text(&response.message).trim(),
            call_sig
        );

        // Stage output with the crate tracker. Its halt verdict is intentionally
        // deferred until the matching tool batch is confirmed successful.
        let _ = self.tracker.record_output(&output_sig, all_exempt);

        // Stage the batch for the repeat-CALL guard, evaluated once every result
        // is back (gated on success) in `after_tool`.
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some(PendingCallBatch {
                call_sig,
                remaining: tool_calls.len(),
                all_ok: true,
                exempt: all_exempt,
            });
        }
        Ok(())
    }

    async fn after_tool(
        &self,
        _ctx: &mut RunContext<()>,
        _state: &(),
        result: &mut TaToolResult,
    ) -> TaResult<()> {
        // Fold this result into the pending batch; only act once the batch is
        // complete so the call guard sees whole-batch success.
        let completed = {
            let Ok(mut pending) = self.pending.lock() else {
                return Ok(());
            };
            let Some(batch) = pending.as_mut() else {
                return Ok(());
            };
            if result.error.is_some() {
                batch.all_ok = false;
            }
            batch.remaining = batch.remaining.saturating_sub(1);
            if batch.remaining == 0 {
                pending.take()
            } else {
                None
            }
        };
        let Some(batch) = completed else {
            return Ok(());
        };

        if let SuccessfulRepeat::Halt(summary) =
            self.tracker
                .record_call_batch(&batch.call_sig, batch.all_ok, batch.exempt)
        {
            tracing::warn!("[tinyagents::mw] crate successful-repeat tracker halted the run");
            self.halt(summary);
        }
        Ok(())
    }
}

// ── ImageAwareMessageTrimMiddleware ───────────────────────────────────────────

/// Flat token cost charged per image — an inline `[IMAGE:…]` marker or a native
/// [`ContentBlock::Image`] block — instead of counting the base64 payload as
/// text. Restores the legacy `harness/token_budget.rs` semantics (issue #4462):
/// the crate `estimate_tokens` prices text at chars/4, so a single large base64
/// image reads as ~2M tokens and the trim believes the context is massively over
/// budget, evicting the whole transcript (system messages included). Providers
/// bill an image at ≈85–1100 tokens by detail; 1200 is a conservative upper
/// bound that keeps the budget realistic without the base64 payload inflating it.
const IMAGE_MARKER_TOKEN_COST: u64 = 1_200;

/// Inline image-marker prefix produced by the multimodal composer
/// (`agent/multimodal.rs`, `compose_multimodal_message`). Priced at
/// [`IMAGE_MARKER_TOKEN_COST`] rather than by its base64 length.
const IMAGE_MARKER_PREFIX: &str = "[IMAGE:";

/// Minimum reply/output reserve — mirrors the legacy `MIN_OUTPUT_RESERVE_TOKENS`.
const MIN_OUTPUT_RESERVE_TOKENS: u64 = 512;

/// Upper anchor for the reply/output reserve — mirrors the legacy
/// `DEFAULT_OUTPUT_RESERVE_TOKENS`.
const DEFAULT_OUTPUT_RESERVE_TOKENS: u64 = 8_192;

/// Rough token estimate (~4 characters per token) with inline `[IMAGE:…]`
/// markers charged a flat [`IMAGE_MARKER_TOKEN_COST`] instead of their base64
/// length. Mirrors the deleted `token_budget::estimate_tokens` (issue #4462).
/// Markerless text takes the fast char/4 path.
fn estimate_text_tokens(text: &str) -> u64 {
    if !text.contains(IMAGE_MARKER_PREFIX) {
        return (text.len() as u64).saturating_add(3) / 4;
    }
    let mut text_bytes: u64 = 0;
    let mut images: u64 = 0;
    let mut cursor = 0usize;
    while let Some(rel) = text[cursor..].find(IMAGE_MARKER_PREFIX) {
        let start = cursor + rel;
        text_bytes = text_bytes.saturating_add((start - cursor) as u64); // preceding text
        let after = start + IMAGE_MARKER_PREFIX.len();
        match text[after..].find(']') {
            Some(rel_end) => {
                images += 1;
                cursor = after + rel_end + 1; // skip the whole marker payload
            }
            None => {
                // Unterminated marker — count the remainder as text and stop.
                text_bytes = text_bytes.saturating_add((text.len() - start) as u64);
                cursor = text.len();
                break;
            }
        }
    }
    text_bytes = text_bytes.saturating_add((text.len() - cursor) as u64); // trailing text
    (text_bytes.saturating_add(3) / 4)
        .saturating_add(images.saturating_mul(IMAGE_MARKER_TOKEN_COST))
}

/// Count native [`ContentBlock::Image`] blocks on a message. `Message::text()`
/// concatenates only text blocks, so a native multimodal image would otherwise
/// contribute zero tokens; we charge each one [`IMAGE_MARKER_TOKEN_COST`].
fn count_native_image_blocks(msg: &TaMessage) -> u64 {
    let content = match msg {
        TaMessage::System(m) => &m.content,
        TaMessage::User(m) => &m.content,
        TaMessage::Assistant(m) => &m.content,
        TaMessage::Tool(m) => &m.content,
    };
    content
        .iter()
        .filter(|b| matches!(b, ContentBlock::Image(_)))
        .count() as u64
}

/// Estimate the tokens of a crate [`TaMessage`]: image-aware text tokens, a flat
/// [`IMAGE_MARKER_TOKEN_COST`] per native image block, and the assistant's
/// tool-call name/arguments (which `Message::text()` drops). Mirrors the legacy
/// `estimate_conversation_message_tokens` (issue #4462).
fn estimate_message_tokens(msg: &TaMessage) -> u64 {
    let mut total = estimate_text_tokens(&msg.text());
    total = total
        .saturating_add(count_native_image_blocks(msg).saturating_mul(IMAGE_MARKER_TOKEN_COST));
    if let TaMessage::Assistant(m) = msg {
        for call in &m.tool_calls {
            total = total.saturating_add(estimate_text_tokens(&call.name));
            total = total.saturating_add(estimate_text_tokens(&call.arguments.to_string()));
        }
    }
    total
}

/// Reply/output reserve, mirroring the legacy proportional clamp
/// `clamp(window/10, ≥512, ≤max(8192, window/4))`. Restores the small-window
/// budget the fixed `window − AGENT_TURN_MAX_OUTPUT_TOKENS` regressed (issue
/// #4462): an 8k model reserves ~819 tokens (input budget ~7373), not
/// 16384 → floored 1024.
fn legacy_output_reserve_tokens(window: u64) -> u64 {
    let pct = window / 10;
    pct.max(MIN_OUTPUT_RESERVE_TOKENS)
        .min(DEFAULT_OUTPUT_RESERVE_TOKENS.max(window / 4))
}

/// Input-prompt token budget after reserving room for the reply. Public to the
/// seam so the install site (and tests) can assert the legacy proportional
/// formula (issue #4462).
pub(super) fn legacy_max_input_tokens(window: u64) -> u64 {
    window.saturating_sub(legacy_output_reserve_tokens(window))
}

/// Deterministic history trim that replaces the crate `MessageTrimMiddleware`
/// (issue #4462), restoring three regression guards the crate trim lost:
///
/// 1. **Image-aware token estimate** — inline `[IMAGE:…]` markers and native
///    image blocks are each charged a flat [`IMAGE_MARKER_TOKEN_COST`] instead
///    of their base64 length, so one large image can no longer read as ~2M
///    tokens and evict the whole transcript.
/// 2. **System messages never dropped** — only non-system history is evictable;
///    the crate trim reorders system messages to the front and drops them as a
///    last resort.
/// 3. **Order preserved + observable** — retained messages keep their original
///    relative order, leading orphaned tool results are snapped past (so no
///    provider 400), and any eviction logs a grep-able `warn` carrying
///    (messages dropped, messages/tokens before-and-after).
pub(crate) struct ImageAwareMessageTrimMiddleware {
    /// Input-prompt token budget (already net of the proportional reply reserve).
    budget: u64,
}

impl ImageAwareMessageTrimMiddleware {
    /// Build a trim middleware whose budget is the legacy proportional
    /// [`legacy_max_input_tokens`] for `window` (issue #4462) — NOT the crate's
    /// fixed `window − 16384`. Floored at 1 so the budget is always positive.
    pub(crate) fn for_context_window(window: u64) -> Self {
        Self {
            budget: legacy_max_input_tokens(window).max(1),
        }
    }
}

#[async_trait]
impl Middleware<()> for ImageAwareMessageTrimMiddleware {
    fn name(&self) -> &str {
        "image_aware_message_trim"
    }

    async fn before_model(
        &self,
        _ctx: &mut RunContext<()>,
        _state: &(),
        request: &mut ModelRequest,
    ) -> TaResult<()> {
        let messages = &mut request.messages;
        let original_tokens: u64 = messages.iter().map(estimate_message_tokens).sum();
        if original_tokens <= self.budget {
            return Ok(());
        }
        let original_len = messages.len();

        // Evict oldest non-system messages first, preserving the relative order
        // of every retained message (rebuilding as `system ++ other` would
        // reorder history when a system message appears after non-system ones —
        // exactly the crate-trim regression). System messages are NEVER dropped.
        let mut removable_positions: Vec<usize> = messages
            .iter()
            .enumerate()
            .filter_map(|(idx, m)| (!matches!(m, TaMessage::System(_))).then_some(idx))
            .collect();

        let mut removed = 0usize;
        while !removable_positions.is_empty() {
            let total: u64 = messages.iter().map(estimate_message_tokens).sum();
            if total <= self.budget {
                break;
            }
            let absolute_idx = removable_positions.remove(0);
            // Subsequent positions shift left by one for every prior removal.
            let remove_at = absolute_idx - removed;
            messages.remove(remove_at);
            removed += 1;
        }

        // Snap the window forward past any leading orphaned tool results: dropping
        // an `assistant(tool_calls)` while keeping its `tool` answer leaves the
        // transcript opening on a tool message with no preceding tool-call, which
        // native providers reject with a 400. Drop leading tool results until the
        // first non-system message is a clean turn boundary.
        while let Some(first_non_system) = messages
            .iter()
            .position(|m| !matches!(m, TaMessage::System(_)))
        {
            if matches!(messages[first_non_system], TaMessage::Tool(_)) {
                messages.remove(first_non_system);
                removed += 1;
            } else {
                break;
            }
        }

        if removed > 0 {
            let final_tokens: u64 = messages.iter().map(estimate_message_tokens).sum();
            tracing::warn!(
                messages_dropped = removed,
                messages_before = original_len,
                messages_after = messages.len(),
                tokens_before = original_tokens,
                tokens_after = final_tokens,
                budget = self.budget,
                "[tinyagents::mw] message_trim evicted oldest history to fit the token budget"
            );
        }
        Ok(())
    }
}

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
