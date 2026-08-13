//! Realtime voice-agent turn handler (#5399).
//!
//! The backend relays each turn of an ElevenLabs Agents session down the socket
//! as `voice:harness { correlationId, messages }` (see the backend's
//! `/voice-agent/chat/completions` Custom-LLM relay). We run the **local
//! orchestrator agent** — the same brain the chat UI and meet bot use, with the
//! user's tools/memory/MCP — and stream the reply back up as
//! `voice:harness:delta` / `voice:harness:done` (or `:error`). This is what
//! keeps a cloud realtime voice session backed by the desktop-local brain.
//!
//! Approval-gate origin: **ExternalChannel** — the turn text is user speech
//! arriving over a channel, so `external_effect` tools route through the
//! audit-trail path rather than running with trusted-CLI semantics.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use log::{info, warn};
use serde_json::{json, Value};
use tokio::sync::Semaphore;

use crate::openhuman::agent::harness::session::Agent;
use crate::openhuman::agent::progress::AgentProgress;
use crate::openhuman::agent::turn_origin::{with_origin, AgentTurnOrigin};
use crate::openhuman::platform::socket::manager::global_socket_manager;

/// Hard ceiling on the background turn — not on how long the caller waits, which
/// the ack deadline below caps at ~8s.
///
/// This governs the deferred half: the orchestrator keeps working after the voice
/// turn closes, and its answer is delivered into chat and read aloud if the call
/// is still up. At 90s that ceiling was cutting real answers. Observed on staging:
/// the same "summarize my emails" request finished in ~53s on one call and was
/// still running past 90s on the next, where it aborted and the caller got a
/// failure notice instead of their summary — a hard failure caused purely by
/// Composio round-trip variance, not by anything being wrong.
///
/// Kept in step with `DEFAULT_TIMEOUT_MS` in the backend's `relayService.ts`,
/// which must stay above this so the desktop's own specific error wins the race
/// rather than the relay's generic timeout.
const TURN_TIMEOUT_SECS: u64 = 180;

/// How long a voice turn may run before we stop making the caller wait and hand
/// the result off to chat. The cloud voice session cancels a turn with no spoken
/// token in ~11-12s, and a slow tool action (email/calendar summary can be
/// 20-30s of Composio round-trips) will never fit that window. Once this elapses
/// we close the voice turn cleanly — the caller has heard the relay's spoken
/// filler and is told the answer is still coming (see VOICE_HANDOFF_LINE) — and
/// let the orchestrator finish in the background,
/// delivering its answer into the user's in-app chat and, while the call is
/// still up, reading it aloud. Sits under the provider's cancel deadline so
/// `done` always beats the cut.
const VOICE_ACK_DEADLINE_SECS: u64 = 8;

/// Spoken when the ack deadline closes a turn that never produced text of its
/// own, so the caller is told the answer is still coming instead of being left on
/// a trail of filler.
///
/// The preface the voice directive asks the model for cannot be relied on to
/// arrive inside the window: building the per-turn orchestrator (config load,
/// tool registry, integration catalogue) and running memory recall takes seconds
/// before the first model token is even requested, and the model's first round is
/// often a tool call carrying no text at all. Measured against staging, the first
/// streamable token landed ~10.6s into a turn whose whole budget is 8s — so every
/// slow turn ended on the relay's ellipsis padding and nothing else, which sounds
/// like the assistant losing the thread rather than working on an answer.
///
/// Both delivery paths honour the promise: the answer is posted to chat and,
/// while the call is still up, read aloud.
const VOICE_HANDOFF_LINE: &str = "I'll have that for you in a moment. ";

/// Sent for a turn we have nothing to say to.
///
/// A turn that ends with no spoken content at all is not a valid answer to the
/// cloud session: it ends the whole call with `custom_llm_error: LLM Cascade
/// Error: Brain returned no response` (confirmed live — three recognition
/// artefacts answered with empty turns killed a working call). So even "nothing
/// to say" has to say something, and the least intrusive something is an ellipsis
/// pause, which the provider voices as a beat of silence rather than words.
const VOICE_SILENT_REPLY: &str = "… ";

/// Voice-scoped transcript namespace. Building a fresh orchestrator per turn
/// would otherwise resume the *chat* orchestrator's latest transcript by name,
/// bleeding an unrelated conversation into (or out of) the voice session. A
/// dedicated name isolates voice from chat; multi-turn context comes from the
/// relayed `messages` we seed below, not from this resume path.
const VOICE_AGENT_NAME: &str = "voice";

/// Model pinned for realtime voice turns. The cloud voice session cancels a turn
/// that has produced no spoken token in ~11-12s ("Generating the LLM response
/// took too long"), and the orchestrator's default reasoning model spends that
/// whole budget *thinking* before its first word. `chat-v1` (DeepSeek-V4-Flash,
/// thinking off) is a short-turn, tool-capable SKU: the master still routes
/// delegation through the prompt (per-turn classification is disabled — see the
/// model pin in `agent/harness/session/turn/core.rs`), so tool turns keep working
/// while spoken replies start in ~1s instead of ~6s. Reasoning models are the
/// wrong tool for a latency-capped realtime channel.
const VOICE_MODEL: &str = "chat-v1";

/// Instruction prefix used for "speak-back": when a deferred result is ready, the
/// renderer's live voice session sends it back as a user message wrapped with this
/// prefix so the agent reads it aloud verbatim. The core recognises the prefix to
/// avoid re-arming speak-back on the read-back turn itself (which would loop). MUST
/// match the string the renderer prepends (`useRealtimeVoiceSession.ts`).
const VOICE_READBACK_PREFIX: &str =
    "Please read the following to me, word for word, and say nothing else:";

/// Chat thread + client id the voice turn scopes as its approval / routing
/// surface, mirroring `deliver_voice_result_to_chat`. Setting these around the
/// turn (via `APPROVAL_CHAT_CONTEXT` + `with_thread_id`) is what makes the voice
/// orchestrator behave like the chat path for tools that need a *routable*
/// approval surface:
///
/// - `composio_connect` fails closed with a `[policy-denied] … needs an
///   interactive chat turn` message whenever `APPROVAL_CHAT_CONTEXT` is absent
///   (see `integrations/composio/tools.rs`). On voice that message got
///   paraphrased back to the user as "your Gmail connection is throwing an auth
///   error, reconnect it" — the exact voice-only Gmail-summary failure — even
///   though the same request works in tap-and-speak (a `WebChat` turn, which
///   installs this context). With the context set, the tool reaches its
///   already-connected short-circuit and returns success instead.
/// - external_effect tool approvals raised on the `ExternalChannel` turn now
///   have a thread card to route to (the same `proactive:voice` thread where
///   deferred voice answers land) rather than silently TTL-denying.
/// - `with_thread_id` gives async delegation (`spawn_async_subagent`) the
///   `parent_thread_id` it requires and aligns inference logs / KV-cache with
///   the voice thread.
const VOICE_CHAT_THREAD_ID: &str = "proactive:voice";
const VOICE_CHAT_CLIENT_ID: &str = "system";

/// Cap on concurrent local-agent turns driven by the relay. Each turn loads
/// config, builds a full orchestrator, and runs for up to `TURN_TIMEOUT_SECS`,
/// so an unbounded burst (or retry storm) would spawn unbounded heavy agent
/// sessions. Excess turns queue on the permit rather than piling up.
const MAX_CONCURRENT_VOICE_TURNS: usize = 3;

static VOICE_TURN_LIMITER: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(MAX_CONCURRENT_VOICE_TURNS)));

/// Correlation ids currently being processed, so a relay retry that re-delivers
/// the same `voice:harness` event is deduplicated instead of running the turn
/// (and charging/emitting) twice.
static IN_FLIGHT_CORRELATIONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// RAII claim on a correlation id: inserts on `claim`, removes on drop. `None`
/// from `claim` means a turn for that id is already in flight.
struct InFlightGuard(String);

impl InFlightGuard {
    fn claim(correlation_id: &str) -> Option<Self> {
        let mut set = IN_FLIGHT_CORRELATIONS
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if set.insert(correlation_id.to_string()) {
            Some(Self(correlation_id.to_string()))
        } else {
            None
        }
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        let mut set = IN_FLIGHT_CORRELATIONS
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        set.remove(&self.0);
    }
}

/// Convert an OpenAI-style `messages` array into `(role, content)` history pairs
/// for [`Agent::seed_resume_from_messages`]. Drops `system` turns — the relayed
/// system prompt is the ElevenLabs agent's, not ours — and flattens multimodal
/// content the same way [`extract_prompt`] does. Pure + unit-tested.
fn messages_to_history_pairs(messages: &[Value]) -> Vec<(String, String)> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            if role != "user" && role != "assistant" && role != "agent" {
                return None;
            }
            let text = content_to_text(msg.get("content"));
            if text.trim().is_empty() {
                return None;
            }
            Some((role.to_string(), text))
        })
        .collect()
}

/// Spoken-output directive appended to the orchestrator profile so replies read
/// naturally through TTS instead of as markdown.
///
/// The tool-preface clause is latency-critical, not cosmetic: the cloud realtime
/// session enforces a per-response time ceiling, and a turn that delegates (email,
/// calendar, files) produces no top-level assistant text until the sub-agent
/// returns 10-20s later — past the ceiling. Emitting one short spoken sentence
/// first makes the model stream an immediate `TextDelta`, so audio reaches the
/// caller right away and the turn stays alive while the tool runs.
const VOICE_DIRECTIVE: &str = "You are speaking aloud in a live voice conversation. \
Reply in natural, concise spoken sentences. Do not use markdown, code blocks, \
bullet lists, headings, or emoji. Before you use a tool or delegate (for example \
to check email, a calendar, files, or the web), first say one short spoken \
sentence telling the user what you are doing — and, since those actions sometimes \
take a while, that you'll follow up with the details in their chat IF it does — \
then proceed. Example: \"Sure, let me pull up your inbox — if it takes a moment \
I'll drop the summary in your chat.\" Keep that preface to one sentence so it \
starts speaking immediately.";

/// Extract the user prompt from an OpenAI-style `messages` array: the content of
/// the last `user` message. Content may be a plain string or an array of
/// `{ type: 'text', text }` parts (multimodal shape). Pure + unit-tested.
pub fn extract_prompt(messages: &[Value]) -> String {
    for msg in messages.iter().rev() {
        if msg.get("role").and_then(Value::as_str) == Some("user") {
            return content_to_text(msg.get("content"));
        }
    }
    String::new()
}

fn content_to_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Handle one relayed voice turn end to end. The reply streams token-by-token
/// back up the socket as it is produced. A turn that finishes inside the voice
/// window is spoken in full; a slow tool action (email/calendar summary — tens of
/// seconds of Composio round-trips) is acknowledged aloud, then finishes in the
/// background and delivers its answer into the user's in-app chat. Never panics —
/// every failure path emits `voice:harness:error` (or a clean `done`) so the
/// backend relay ends the turn cleanly.
pub async fn handle_voice_harness_turn(correlation_id: String, messages: Vec<Value>) {
    let prompt = extract_prompt(&messages);
    if prompt.trim().is_empty() {
        emit_error(&correlation_id, "no user message in the relayed turn").await;
        return;
    }

    // A read-back turn hands us the very text it wants spoken. Running an
    // orchestrator turn to echo it costs a full agent build, memory recall and a
    // model round-trip — far more than the turn budget allows — so the caller
    // hears filler instead of the answer they are waiting for. Current backends
    // answer these at the relay and never wake us; an older one still relays them,
    // so answer from the prompt here too.
    if let Some(payload) = readback_payload(&prompt) {
        info!(
            "[voice-harness] read-back answered from the prompt correlation={correlation_id} chars={}",
            payload.chars().count()
        );
        // Always speak something, even for an empty payload — see VOICE_SILENT_REPLY.
        emit_event(
            "voice:harness:delta",
            json!({ "correlationId": correlation_id,
                    "text": if payload.is_empty() { VOICE_SILENT_REPLY } else { payload } }),
        )
        .await;
        emit_event(
            "voice:harness:done",
            json!({ "correlationId": correlation_id }),
        )
        .await;
        return;
    }

    // Speech recognition turns a pause during a filler-heavy turn into a "..."
    // user message, and the provider relays it as a real turn. Building an
    // orchestrator for it burns a concurrency slot and a model round-trip to
    // answer nothing — and its empty reply surfaced in chat as a failure notice
    // the user never asked for.
    if is_content_free(&prompt) {
        info!("[voice-harness] ignoring content-free turn correlation={correlation_id} prompt={prompt:?}");
        emit_event(
            "voice:harness:delta",
            json!({ "correlationId": correlation_id, "text": VOICE_SILENT_REPLY }),
        )
        .await;
        emit_event(
            "voice:harness:done",
            json!({ "correlationId": correlation_id }),
        )
        .await;
        return;
    }

    // Deduplicate a re-delivered turn (relay retry) before doing any work. Owned
    // so it can move into the background continuation and track the real turn.
    let Some(in_flight) = InFlightGuard::claim(&correlation_id) else {
        warn!("[voice-harness] duplicate turn correlation={correlation_id} already in flight — dropping");
        return;
    };

    // Stream the reply token-by-token: the orchestrator emits `TextDelta` as it
    // generates, and `forward_reply_deltas` relays each as a `voice:harness:delta`
    // so the reply leaves the desktop as it is produced.
    let streamed = Arc::new(AtomicBool::new(false));
    let (progress_tx, progress_rx) = tokio::sync::mpsc::channel::<AgentProgress>(256);
    let forwarder = tokio::spawn(forward_reply_deltas(
        progress_rx,
        correlation_id.clone(),
        streamed.clone(),
    ));

    // Run the turn on a detached task so it can outlive the spoken ack. A slow
    // delegation keeps working after we close the voice turn and delivers its
    // result into the user's chat. The task owns the agent, the concurrency
    // permit, and the in-flight guard so that bookkeeping tracks the real turn.
    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
    // Captured before the prompt moves into the task, so the foreground can still
    // tell whether the user is waiting on an answer if that task dies.
    let answerable = is_answerable_prompt(&prompt);
    let turn_cid = correlation_id.clone();
    let turn_messages = messages;
    let turn_prompt = prompt;
    tokio::spawn(async move {
        let _in_flight = in_flight;
        // Bound concurrent heavy agent turns; excess turns queue HERE, inside the
        // detached task, never in front of the ack deadline below. Acquiring in the
        // foreground meant a turn queued behind slower ones started no clock and
        // emitted no `done` at all, and the provider ended the whole session over
        // it. Held for the REAL turn lifetime (including any background tail), so
        // it still caps a retry storm.
        let _permit = match VOICE_TURN_LIMITER.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                warn!("[voice-harness] turn limiter unavailable correlation={turn_cid}");
                if is_answerable_prompt(&turn_prompt) {
                    deliver_voice_failure_to_chat(&turn_cid);
                }
                return;
            }
        };
        let outcome = run_voice_turn(&turn_cid, &turn_messages, &turn_prompt, progress_tx).await;
        // Hand the result to the foreground. If it already deferred (dropped the
        // receiver at the ack deadline), the send fails and we deliver the reply
        // into the user's chat instead.
        if let Err(unsent) = result_tx.send(outcome) {
            match unsent {
                Ok(reply) => {
                    // A read-back turn only re-reads an answer already delivered to
                    // chat; delivering it again would duplicate the chat message, and
                    // re-arming speak-back would loop. So deliver + arm speak-back ONLY
                    // for a genuine deferred answer, and skip the whole delivery for a
                    // read-back echo turn.
                    if should_arm_speak_back(&turn_prompt) {
                        deliver_voice_result_to_chat(&turn_cid, reply, true);
                    } else {
                        info!("[voice-harness] deferred read-back turn carries no new content; skipping chat delivery correlation={turn_cid}");
                    }
                }
                Err(err) => {
                    // The spoken turn already closed with `done` and the preface may
                    // have promised a chat follow-up, so a silent failure would leave
                    // the user waiting for a message that never arrives. Post a brief
                    // failure notice to the same thread — but only for a turn whose
                    // answer the user is actually waiting on. A read-back or a
                    // recognition artefact has nothing to deliver, and a notice for
                    // one reads as the assistant failing a request nobody made.
                    warn!("[voice-harness] deferred turn failed correlation={turn_cid}: {err}");
                    if is_answerable_prompt(&turn_prompt) {
                        deliver_voice_failure_to_chat(&turn_cid);
                    }
                }
            }
        }
    });

    // Race the turn against the spoken-ack deadline.
    match tokio::time::timeout(Duration::from_secs(VOICE_ACK_DEADLINE_SECS), result_rx).await {
        Ok(Ok(outcome)) => {
            // Finished inside the voice window — deltas already streamed. Join the
            // forwarder so every delta is out before `done`, and learn whether
            // anything streamed (fallback for a non-streaming reply).
            let streamed_any = forwarder.await.unwrap_or(false);
            match outcome {
                Ok(reply) => {
                    if !streamed_any {
                        // A turn that produced no text still has to say something, or
                        // the provider ends the call (see VOICE_SILENT_REPLY).
                        let spoken = reply.trim();
                        let spoken = if spoken.is_empty() {
                            VOICE_SILENT_REPLY
                        } else {
                            spoken
                        };
                        emit_event(
                            "voice:harness:delta",
                            json!({ "correlationId": correlation_id, "text": spoken }),
                        )
                        .await;
                    }
                    emit_event(
                        "voice:harness:done",
                        json!({ "correlationId": correlation_id }),
                    )
                    .await;
                }
                Err(err) => {
                    warn!("[voice-harness] turn failed correlation={correlation_id}: {err}");
                    emit_error(&correlation_id, &err).await;
                }
            }
        }
        Ok(Err(_recv)) => {
            // Sender dropped without a value (task aborted or panicked). A silent
            // turn with no spoken text and no chat delivery is hard to trace, so
            // log with the correlation id before ending cleanly.
            warn!(
                "[voice-harness] turn task ended without a result (panicked or aborted) correlation={correlation_id}"
            );
            // Unlike the ack-deadline path, nothing else will deliver here: the task
            // died before reaching its own chat-delivery branch. The turn may already
            // have promised a follow-up in chat, so post the notice from this side —
            // for a turn the user is actually waiting on (see is_answerable_prompt).
            if answerable {
                deliver_voice_failure_to_chat(&correlation_id);
            }
            // Closing on nothing at all would end the whole call (see
            // VOICE_SILENT_REPLY), so a lost turn still ends with a spoken beat.
            if !streamed.load(Ordering::SeqCst) {
                emit_event(
                    "voice:harness:delta",
                    json!({ "correlationId": correlation_id, "text": VOICE_SILENT_REPLY }),
                )
                .await;
            }
            emit_event(
                "voice:harness:done",
                json!({ "correlationId": correlation_id }),
            )
            .await;
        }
        Err(_deadline) => {
            // Still running (a slow tool action). Close the voice turn cleanly; the
            // detached task keeps going and delivers its answer into the user's
            // chat. `timeout` consumed `result_rx`, so the task's send fails and
            // takes the chat-delivery path. The forwarder is left running to keep
            // draining progress — any late deltas reach a settled relay turn and
            // are dropped harmlessly.
            info!("[voice-harness] ack deadline reached, handing off to chat correlation={correlation_id}");
            // If the turn never said anything of its own, the caller has heard only
            // the relay's filler. Ending there sounds like the assistant lost the
            // thread, so say the answer is still coming — it is, on both delivery
            // paths (chat, and read aloud while the call is up).
            if !streamed.load(Ordering::SeqCst) {
                emit_event(
                    "voice:harness:delta",
                    json!({ "correlationId": correlation_id, "text": VOICE_HANDOFF_LINE }),
                )
                .await;
            }
            emit_event(
                "voice:harness:done",
                json!({ "correlationId": correlation_id }),
            )
            .await;
        }
    }
}

/// Forward the orchestrator's streamed assistant text to the relay socket, one
/// `voice:harness:delta` per top-level `AgentProgress::TextDelta`, until the
/// turn's progress channel closes (the agent drops its sender when the turn
/// ends). Returns whether any non-empty delta was streamed, so the caller can
/// fall back to emitting the whole reply for a turn that produced text off the
/// streaming path. Only top-level assistant text is voiced — sub-agent narration,
/// thinking, tool-call args, and lifecycle events are deliberately not spoken.
async fn forward_reply_deltas(
    mut progress_rx: tokio::sync::mpsc::Receiver<AgentProgress>,
    correlation_id: String,
    streamed: Arc<AtomicBool>,
) -> bool {
    let mut streamed_any = false;
    while let Some(progress) = progress_rx.recv().await {
        let Some(text) = spoken_delta(&progress) else {
            continue;
        };
        // Skip only truly empty deltas — whitespace carries word boundaries and
        // must be forwarded so the concatenated speech isn't run together.
        if text.is_empty() {
            continue;
        }
        streamed_any = true;
        // Published for the ack + handoff decisions, which need to know whether the
        // orchestrator is talking *while* the turn is still open.
        streamed.store(true, Ordering::SeqCst);
        emit_event(
            "voice:harness:delta",
            json!({ "correlationId": correlation_id, "text": text }),
        )
        .await;
    }
    streamed_any
}

/// The spoken text carried by a progress event, or `None` for events that must
/// not be voiced. Only the top-level assistant `TextDelta` is spoken; sub-agent
/// deltas, thinking, tool-call args, and lifecycle events are internal. Pure +
/// unit-tested.
fn spoken_delta(progress: &AgentProgress) -> Option<&str> {
    match progress {
        AgentProgress::TextDelta { delta, .. } => Some(delta),
        _ => None,
    }
}

/// The answer a read-back turn is asking to have spoken, or `None` for an
/// ordinary turn. Leading whitespace is tolerated because the renderer joins the
/// prefix and payload with a blank line. Pure + unit-tested.
fn readback_payload(prompt: &str) -> Option<&str> {
    let trimmed = prompt.trim_start();
    trimmed.strip_prefix(VOICE_READBACK_PREFIX).map(str::trim)
}

/// Whether a prompt carries nothing to answer. Speech recognition emits `"..."`
/// (and similar punctuation-only artefacts) for a pause, and the provider relays
/// those as real turns. Anything with a letter or a digit in it — in any script —
/// is a genuine prompt. Pure + unit-tested.
fn is_content_free(prompt: &str) -> bool {
    !prompt.chars().any(char::is_alphanumeric)
}

/// Whether the user is waiting on this turn's answer, and so should be told when
/// it fails. False for a read-back (its answer is already in chat) and for a
/// recognition artefact (nothing was asked). Pure + unit-tested.
fn is_answerable_prompt(prompt: &str) -> bool {
    !is_content_free(prompt) && should_arm_speak_back(prompt)
}

/// Whether a completed voice turn should arm speak-back — i.e. push its deferred
/// answer back into the live session to be read aloud. A read-back turn is itself
/// a verbatim-read request (its prompt is wrapped with [`VOICE_READBACK_PREFIX`]
/// by the renderer), so re-arming speak-back on it would deliver the spoken copy
/// to a turn that then asks to read it again — an unbounded loop. Suppress those.
/// Pure + unit-tested; leading whitespace is tolerated because the renderer joins
/// the prefix and payload with a blank line.
fn should_arm_speak_back(prompt: &str) -> bool {
    !prompt.trim_start().starts_with(VOICE_READBACK_PREFIX)
}

/// Build the fresh voice orchestrator, attach the streaming sink, run one turn
/// under the hard per-turn ceiling, then detach the sink so the forwarder's
/// channel closes. Runs entirely on the background task, so the ack deadline in
/// the caller covers both the build and the model round-trips.
async fn run_voice_turn(
    correlation_id: &str,
    messages: &[Value],
    prompt: &str,
    progress_tx: tokio::sync::mpsc::Sender<AgentProgress>,
) -> Result<String, String> {
    let mut agent = build_voice_agent(correlation_id, messages, prompt).await?;

    // Attach the streaming sink before the turn: its presence switches the harness
    // onto the true per-token streaming path, and each `AgentProgress::TextDelta`
    // is forwarded to the relay socket by `forward_reply_deltas`.
    agent.set_on_progress(Some(progress_tx));

    let outcome = run_single_with_timeout(&mut agent, correlation_id, prompt).await;

    // Detach the sink so the forwarder's channel closes the moment the turn ends,
    // deterministically rather than waiting on `agent`'s drop.
    agent.set_on_progress(None);
    outcome
}

/// Construct the per-turn voice orchestrator: load config, pin the fast voice
/// model, isolate the transcript namespace, and seed the relayed history.
async fn build_voice_agent(
    correlation_id: &str,
    messages: &[Value],
    prompt: &str,
) -> Result<Agent, String> {
    let config = crate::openhuman::config::ops::load_config_with_timeout().await?;
    let mut agent = Agent::from_config_for_agent_with_profile(
        &config,
        "orchestrator",
        None,
        Some(VOICE_DIRECTIVE.to_string()),
        None,
    )
    .map_err(|e| format!("orchestrator build failed: {e}"))?;
    agent.set_event_context(format!("voice_{correlation_id}"), "voice_agent");
    // Isolate the voice transcript namespace from the chat orchestrator so a
    // fresh-per-turn agent can't resume an unrelated conversation by name.
    agent.set_agent_definition_name(VOICE_AGENT_NAME);
    // Pin a fast, non-thinking model so the first spoken token lands inside the
    // realtime session's response-time ceiling (see VOICE_MODEL).
    agent.set_model_name(VOICE_MODEL);

    // Seed the authoritative prior turns the relay carries (OpenAI `messages`),
    // so follow-ups like "what about tomorrow?" keep their context. No-ops when
    // there is nothing prior to the current user message.
    let history = messages_to_history_pairs(messages);
    if let Err(e) = agent.seed_resume_from_messages(history, prompt) {
        warn!("[voice-harness] seed prior messages failed correlation={correlation_id}: {e}");
    }

    info!(
        "[voice-harness] orchestrator turn correlation={correlation_id} prompt_chars={} history_msgs={}",
        prompt.chars().count(),
        messages.len()
    );
    Ok(agent)
}

/// Run the orchestrator turn under the hard per-turn ceiling. The streaming sink
/// must already be attached; deltas flow out while this runs.
async fn run_single_with_timeout(
    agent: &mut Agent,
    correlation_id: &str,
    prompt: &str,
) -> Result<String, String> {
    // Scope the turn with the SAME chat context the web-chat path installs
    // (`APPROVAL_CHAT_CONTEXT` + `with_thread_id`), so approval-surfaced tools
    // behave identically on voice. Without it `composio_connect` fails closed
    // for lack of a routable surface, which the model paraphrases to the user as
    // a confabulated "reconnect your Gmail" mid email-summary (#5399). See
    // VOICE_CHAT_THREAD_ID for the full rationale. Nesting mirrors web chat:
    // origin (outer) → approval context → thread id → the agent run.
    let approval_ctx = crate::openhuman::security::approval::ApprovalChatContext {
        thread_id: VOICE_CHAT_THREAD_ID.to_string(),
        client_id: VOICE_CHAT_CLIENT_ID.to_string(),
    };
    let scoped_run = crate::openhuman::agent::tinyagents::thread_context::with_thread_id(
        VOICE_CHAT_THREAD_ID,
        agent.run_single(prompt),
    );
    let fut = with_origin(
        AgentTurnOrigin::ExternalChannel {
            channel: "voice".to_string(),
            sender: None,
            reply_target: correlation_id.to_string(),
            message_id: format!("voice-{correlation_id}"),
        },
        crate::openhuman::security::approval::APPROVAL_CHAT_CONTEXT.scope(approval_ctx, scoped_run),
    );

    match tokio::time::timeout(Duration::from_secs(TURN_TIMEOUT_SECS), fut).await {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(format!("orchestrator run_single failed: {e}")),
        Err(_) => Err(format!(
            "orchestrator turn timed out after {TURN_TIMEOUT_SECS}s"
        )),
    }
}

/// Deliver a deferred voice turn's answer into the user's in-app chat. Publishes
/// a `proactive_message` on the web-channel event bus — the same seam cron and the
/// subconscious use — which the frontend renders as an assistant message in a
/// visible thread. Web-only: it does not fan out to external channels (#5399).
fn deliver_voice_result_to_chat(correlation_id: &str, reply: String, allow_speak_back: bool) {
    let spoken = reply.trim();
    if spoken.is_empty() {
        warn!("[voice-harness] deferred turn produced no text correlation={correlation_id}");
        // The spoken ack already promised a chat follow-up, so an empty deferred
        // reply must still surface a message rather than leave the user waiting.
        deliver_voice_failure_to_chat(correlation_id);
        return;
    }
    info!(
        "[voice-harness] delivering deferred result to chat correlation={correlation_id} chars={} speak_back={allow_speak_back}",
        spoken.chars().count()
    );
    crate::openhuman::web_chat::publish_web_channel_event(crate::core::socketio::WebChannelEvent {
        event: "proactive_message".to_string(),
        client_id: VOICE_CHAT_CLIENT_ID.to_string(),
        thread_id: VOICE_CHAT_THREAD_ID.to_string(),
        full_response: Some(spoken.to_string()),
        success: Some(true),
        ..Default::default()
    });

    // Speak-back: push the finished answer to the renderer's LIVE voice session so
    // the agent can read it aloud. The frontend voice hook listens for `voice_speak`
    // and, only while the call is still open, sends it back into the ElevenLabs
    // session (a fast read-back turn). Skipped for read-back turns themselves to
    // avoid a loop; harmless if the call already ended (nobody is subscribed).
    if allow_speak_back {
        crate::openhuman::web_chat::publish_web_channel_event(
            crate::core::socketio::WebChannelEvent {
                event: "voice_speak".to_string(),
                client_id: VOICE_CHAT_CLIENT_ID.to_string(),
                full_response: Some(spoken.to_string()),
                success: Some(true),
                ..Default::default()
            },
        );
    }
}

/// Deliver a short "couldn't complete" notice to the voice chat thread for a
/// deferred turn that produced no answer the user can see — either it errored,
/// or it completed past the ack deadline with empty text (on this path
/// `run_single`'s returned text is the sole answer channel: the orchestrator
/// folds any tool/subagent output into its final reply, so an empty reply means
/// nothing was produced for the user, not that the answer went elsewhere).
/// Because the spoken preface may have told the user their answer would land in
/// chat, staying silent would leave them waiting on a message that never comes —
/// this makes the promised message always appear. Delivered as a normal
/// assistant message (not spoken) on the same `proactive:voice` surface as a
/// successful deferred answer.
fn deliver_voice_failure_to_chat(correlation_id: &str) {
    info!(
        "[voice-harness] delivering deferred failure notice to chat correlation={correlation_id}"
    );
    crate::openhuman::web_chat::publish_web_channel_event(crate::core::socketio::WebChannelEvent {
        event: "proactive_message".to_string(),
        client_id: VOICE_CHAT_CLIENT_ID.to_string(),
        thread_id: VOICE_CHAT_THREAD_ID.to_string(),
        full_response: Some(
            "Sorry — I couldn't finish that request just now. Please try again.".to_string(),
        ),
        success: Some(false),
        ..Default::default()
    });
}

async fn emit_event(event: &str, payload: Value) {
    match global_socket_manager() {
        Some(mgr) => {
            if let Err(e) = mgr.emit(event, payload).await {
                warn!("[voice-harness] emit {event} failed: {e}");
            }
        }
        None => warn!("[voice-harness] no socket manager; dropping {event}"),
    }
}

async fn emit_error(correlation_id: &str, message: &str) {
    emit_event(
        "voice:harness:error",
        json!({ "correlationId": correlation_id, "message": message }),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_the_last_user_string_message() {
        let messages = vec![
            json!({ "role": "system", "content": "be nice" }),
            json!({ "role": "user", "content": "first" }),
            json!({ "role": "assistant", "content": "ok" }),
            json!({ "role": "user", "content": "what is the weather" }),
        ];
        assert_eq!(extract_prompt(&messages), "what is the weather");
    }

    #[test]
    fn joins_multimodal_text_parts() {
        let messages = vec![json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "hello" },
                { "type": "text", "text": "there" },
            ],
        })];
        assert_eq!(extract_prompt(&messages), "hello there");
    }

    #[test]
    fn returns_empty_when_no_user_message() {
        let messages = vec![json!({ "role": "assistant", "content": "hi" })];
        assert_eq!(extract_prompt(&messages), "");
        assert_eq!(extract_prompt(&[]), "");
    }

    #[test]
    fn history_pairs_keep_user_and_assistant_turns_and_drop_system() {
        let messages = vec![
            json!({ "role": "system", "content": "eleven agent prompt" }),
            json!({ "role": "user", "content": "what is the weather" }),
            json!({ "role": "assistant", "content": "sunny" }),
            json!({ "role": "user", "content": "what about tomorrow" }),
        ];
        let pairs = messages_to_history_pairs(&messages);
        assert_eq!(
            pairs,
            vec![
                ("user".to_string(), "what is the weather".to_string()),
                ("assistant".to_string(), "sunny".to_string()),
                ("user".to_string(), "what about tomorrow".to_string()),
            ]
        );
    }

    #[test]
    fn history_pairs_flatten_multimodal_and_skip_empty() {
        let messages = vec![
            json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] }),
            json!({ "role": "assistant", "content": "   " }),
        ];
        let pairs = messages_to_history_pairs(&messages);
        assert_eq!(pairs, vec![("user".to_string(), "hello".to_string())]);
    }

    #[test]
    fn spoken_delta_forwards_only_top_level_assistant_text() {
        assert_eq!(
            spoken_delta(&AgentProgress::TextDelta {
                delta: "hey there".to_string(),
                iteration: 1,
            }),
            Some("hey there")
        );
        // Whitespace-only deltas carry word boundaries and are still spoken text —
        // the empty-skip lives in the forwarder, not here.
        assert_eq!(
            spoken_delta(&AgentProgress::TextDelta {
                delta: " ".to_string(),
                iteration: 2,
            }),
            Some(" ")
        );
    }

    #[test]
    fn spoken_delta_suppresses_internal_events() {
        // Reasoning must never be voiced.
        assert_eq!(
            spoken_delta(&AgentProgress::ThinkingDelta {
                delta: "let me think".to_string(),
                iteration: 1,
            }),
            None
        );
        // A delegated sub-agent's narration is internal, not the spoken answer.
        assert_eq!(
            spoken_delta(&AgentProgress::SubagentTextDelta {
                agent_id: "a".to_string(),
                task_id: "t".to_string(),
                delta: "fetching inbox".to_string(),
                iteration: 1,
            }),
            None
        );
        // Lifecycle events carry no spoken text.
        assert_eq!(
            spoken_delta(&AgentProgress::TurnCompleted { iterations: 1 }),
            None
        );
    }

    #[test]
    fn speak_back_armed_for_a_genuine_answer_turn() {
        assert!(should_arm_speak_back("summarize my unread emails"));
        assert!(should_arm_speak_back("what's on my calendar tomorrow?"));
    }

    #[test]
    fn speak_back_suppressed_for_a_read_back_turn() {
        // The bare prefix, and the real renderer shape (prefix + blank line +
        // payload, possibly with leading whitespace) must both be recognised so
        // the spoken copy never re-arms into an unbounded loop.
        assert!(!should_arm_speak_back(VOICE_READBACK_PREFIX));
        assert!(!should_arm_speak_back(&format!(
            "{VOICE_READBACK_PREFIX}\n\nHere is your inbox summary."
        )));
        assert!(!should_arm_speak_back(&format!(
            "   \n{VOICE_READBACK_PREFIX} trailing payload"
        )));
    }

    #[test]
    fn read_back_payload_is_the_text_to_speak() {
        assert_eq!(
            readback_payload(&format!(
                "{VOICE_READBACK_PREFIX}\n\nHere is your inbox summary."
            )),
            Some("Here is your inbox summary.")
        );
        // The renderer may prepend whitespace; the payload must survive it intact.
        assert_eq!(
            readback_payload(&format!(
                "  \n{VOICE_READBACK_PREFIX} two things need attention"
            )),
            Some("two things need attention")
        );
        // A prefix with nothing behind it is still a read-back — it just has
        // nothing to say, and must not be relayed as a question.
        assert_eq!(readback_payload(VOICE_READBACK_PREFIX), Some(""));
    }

    #[test]
    fn ordinary_prompts_are_not_read_backs() {
        assert_eq!(readback_payload("summarize my emails"), None);
        assert_eq!(readback_payload("please read my emails to me"), None);
    }

    #[test]
    fn recognition_artefacts_are_content_free() {
        // What the provider actually relayed for a pause during a filler-heavy
        // turn, plus the shapes next to it.
        assert!(is_content_free("..."));
        assert!(is_content_free("…"));
        assert!(is_content_free(" ? "));
        assert!(is_content_free("-"));
    }

    #[test]
    fn real_questions_are_not_content_free() {
        assert!(!is_content_free("summarize my emails"));
        // A single digit is a real answer to "how many?" — and scripts other than
        // Latin must never be mistaken for punctuation.
        assert!(!is_content_free("3"));
        assert!(!is_content_free("मेरे ईमेल पढ़ो"));
        assert!(!is_content_free("总结我的邮件"));
    }

    #[test]
    fn failure_notice_is_limited_to_turns_the_user_is_waiting_on() {
        assert!(is_answerable_prompt("summarize my emails"));
        // A read-back's answer is already in chat; a notice would report a failure
        // for a request the user never made.
        assert!(!is_answerable_prompt(&format!(
            "{VOICE_READBACK_PREFIX}\n\nHere is your inbox summary."
        )));
        assert!(!is_answerable_prompt("..."));
    }
}
