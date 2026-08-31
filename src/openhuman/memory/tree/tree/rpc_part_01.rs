use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::openhuman::config::Config;
use crate::openhuman::memory::api::provider::ChunkQuery;
use crate::rpc::RpcOutcome;
use tinymemory_api::chunks::{Chunk, DataSource, SourceKind, SourceRef};
use tinymemory_api::provider::types::{IngestItem, IngestOutcome};
use tinymemory_api::types::MemoryTaint;

// ── The `memory_tree_ingest` payload shapes (#5560) ──────────────────────────
//
// `ChatBatch`, `EmailThread` and `DocumentInput` were imported from
// `tinycortex::memory::ingest::canonicalize`. They are defined here now, and
// the reason is that **this host is their only reader**: they are the request
// half of an OpenHuman JSON-RPC method, deserialised by [`ingest_rpc`] below
// and turned into contract [`IngestItem`]s by the three mappers further down.
// Nothing on this side ever hands one of these structs to the engine — the
// items are what cross the bus, and the driver rebuilds its own copy of these
// shapes on the far end from those items. Two independent readers of one JSON
// wire format, which is the same arrangement `memory::rpc_models`,
// `memory::safety` and `memory::source_scope` already landed on.
//
// So there is no contract door to route this at, and asking for one would be
// asking `tinymemory-api` to carry a payload that never crosses the bus. What
// pins the *shape* is the wire, not the type: the driver's reconstruction is
// documented field-for-field on [`email_items`] and [`chat_items`], and
// `rpc_tests_part_01_tests` pins the serde tolerances. Change a field name
// here and the round trip breaks in exactly the way those tests describe —
// which is the same exposure the import had, since a rename upstream would
// have reshaped this RPC's published request body without anything here
// failing to compile.
//
// `Serialize` is derived alongside `Deserialize` because the engine derived
// both and the round-trip is what the tests assert; nothing in `src/`
// serialises one.

/// Serde default for the two chat/mail message timestamps.
///
/// A payload missing the field falls back to `now()` rather than rejecting the
/// whole batch, so a client with version skew (or a third-party integration
/// that never sent one) does not lose an entire thread to one absent key.
fn ingest_timestamp_now() -> DateTime<Utc> {
    Utc::now()
}

/// Serde default for [`DocumentInput::provider`].
fn default_document_provider() -> String {
    "unknown".to_string()
}

/// Accept a timestamp as epoch-milliseconds, an RFC 3339 / ISO-8601 string, or
/// `null`.
///
/// Three shapes because three generations of clients send three shapes, and
/// the alternative to accepting all of them is silently dropping whichever the
/// caller happens to use.
///
/// The near-epoch rejection is the load-bearing part: contemporary epoch
/// *seconds* are ten digits and epoch *milliseconds* are thirteen, so a
/// seconds value passed here would decode to 1970 and quietly poison ordering
/// and staleness. Rejecting the ambiguous range makes that a loud error at the
/// seam instead.
fn deserialize_flexible_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawTs {
        Millis(i64),
        Text(String),
        Null,
    }

    fn epoch_millis<E: serde::de::Error>(ms: i64) -> Result<DateTime<Utc>, E> {
        const MIN_PLAUSIBLE_EPOCH_MILLIS: u64 = 100_000_000_000;
        if ms.unsigned_abs() < MIN_PLAUSIBLE_EPOCH_MILLIS {
            return Err(E::custom(format!(
                "epoch-ms value {ms} is too small; pass milliseconds, not seconds"
            )));
        }
        chrono::TimeZone::timestamp_millis_opt(&Utc, ms)
            .single()
            .ok_or_else(|| E::custom(format!("invalid epoch-ms: {ms}")))
    }

    let raw = RawTs::deserialize(deserializer)?;
    match raw {
        RawTs::Null => Ok(Utc::now()),
        RawTs::Millis(ms) => epoch_millis(ms),
        RawTs::Text(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.with_timezone(&Utc));
            }
            if let Ok(ms) = s.parse::<i64>() {
                return epoch_millis(ms);
            }
            Err(serde::de::Error::custom(format!(
                "cannot parse '{s}' as RFC 3339 or epoch-ms"
            )))
        }
    }
}

/// One chat message in a channel/group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Author display name or id.
    pub author: String,
    /// When the message was sent (epoch-ms integer or RFC 3339 string).
    /// When absent from the payload, defaults to `Utc::now()` — see
    /// [`ingest_timestamp_now`].
    #[serde(
        default = "ingest_timestamp_now",
        serialize_with = "chrono::serde::ts_milliseconds::serialize",
        deserialize_with = "deserialize_flexible_timestamp"
    )]
    pub timestamp: DateTime<Utc>,
    /// Plain text / markdown body.
    pub text: String,
    /// Optional per-message provenance pointer (permalink or `platform://...`).
    #[serde(default)]
    pub source_ref: Option<String>,
}

/// Adapter input — a batch of messages from one logical channel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBatch {
    /// Platform name (e.g. `slack`, `discord`, `telegram`). Crosses verbatim
    /// as [`IngestItem::platform`]; see [`chat_data_source`] for how it is
    /// additionally mapped onto a [`DataSource`].
    pub platform: String,
    /// Human-readable channel / group name.
    pub channel_label: String,
    /// Ordered messages (chronological; the adapter sorts defensively).
    pub messages: Vec<ChatMessage>,
}

/// One email in a thread.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailMessage {
    /// Sender address; rendered as the `From:` header and used as the
    /// participant key when bucketing a thread.
    pub from: String,
    /// Primary recipient addresses; the `To:` header (omitted when empty).
    #[serde(default)]
    pub to: Vec<String>,
    /// Carbon-copy recipient addresses; the `Cc:` header (omitted when empty).
    #[serde(default)]
    pub cc: Vec<String>,
    /// Per-message subject; the `Subject:` header.
    pub subject: String,
    /// When the message was sent (epoch-ms integer or RFC 3339 string).
    /// When absent, defaults to `Utc::now()` so one missing key does not
    /// reject the whole thread.
    #[serde(
        default = "ingest_timestamp_now",
        serialize_with = "chrono::serde::ts_milliseconds::serialize",
        deserialize_with = "deserialize_flexible_timestamp"
    )]
    pub sent_at: DateTime<Utc>,
    /// Plain-text or markdown body.
    pub body: String,
    /// Message-id header or provider URL; used for citation back to source.
    #[serde(default)]
    pub source_ref: Option<String>,
    /// `List-Unsubscribe` header. Carried through because an unsubscribe flow
    /// reads it back out of stored mail — dropping it makes that flow
    /// impossible, not merely poorer (see [`email_items`]).
    #[serde(default)]
    pub list_unsubscribe: Option<String>,
}

/// A whole email thread.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailThread {
    /// Provider name (e.g. `gmail`, `outlook`). See [`email_data_source`].
    pub provider: String,
    /// Thread subject (usually the subject of the first message).
    pub thread_subject: String,
    /// Ordered messages (chronological; the adapter sorts defensively).
    pub messages: Vec<EmailMessage>,
}

/// Adapter input for a single document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentInput {
    /// Provider name (e.g. `notion`, `drive`, `meeting_notes`). Defaults to
    /// `"unknown"` when absent. See [`document_data_source`].
    #[serde(default = "default_document_provider")]
    pub provider: String,
    /// Document title. Read only to decide whether the payload was wholly
    /// empty — it does not cross to the driver; see [`document_item`].
    pub title: String,
    /// Document body (markdown preferred; plain text also accepted).
    pub body: String,
    /// When the document was last modified at the source.
    ///
    /// Accepts an epoch-milliseconds integer (back-compat), an RFC 3339 /
    /// ISO-8601 string, or absent → `Utc::now()`.
    #[serde(
        default = "ingest_timestamp_now",
        deserialize_with = "deserialize_flexible_timestamp"
    )]
    pub modified_at: DateTime<Utc>,
    /// Optional pointer back to source (URL, file path, Notion page id).
    #[serde(default)]
    pub source_ref: Option<String>,
}

/// Unified ingest request. The `payload` shape is adapter-specific and is
/// validated inside the dispatch based on `source_kind`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Which kind of source the payload represents.
    pub source_kind: SourceKind,
    /// Logical source id (channel/group for chat, thread for email, doc id).
    pub source_id: String,
    /// Account/user this content belongs to.
    #[serde(default)]
    pub owner: String,
    /// Optional labels/tags carried through.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Adapter-specific payload — shape matches the canonicaliser for
    /// `source_kind`:
    /// - `chat`     → [`ChatBatch`]
    /// - `email`    → [`EmailThread`]
    /// - `document` → [`DocumentInput`]
    pub payload: Value,
}

/// Response body of the `memory_tree_ingest` RPC.
///
/// Declared here rather than returned as the engine's own summary type,
/// because this is a wire shape the frontend reads: a body owned by a foreign
/// crate is one an upstream field rename can reshape without anything in this
/// repository failing to compile. Every key and JSON type is what that summary
/// serialised and must stay that way —
/// `the_response_body_serialises_exactly_as_the_engine_summary` is the pin, and
/// it is what the chat and document arms' move onto the driver contract had to
/// keep true. Those two build this body from an `IngestOutcome` now, mail still
/// from the pipeline's summary, and both spell the same six keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestResponse {
    /// Logical source id the ingest was scoped to — the one the caller
    /// supplied, echoed back so a batched caller can pair a reply with its
    /// request.
    pub source_id: String,
    /// Units persisted by this call.
    pub chunks_written: usize,
    /// Units produced and not admitted. Dropped units only: a call refused
    /// outright is [`Self::already_ingested`], not a drop of everything.
    pub chunks_dropped: usize,
    /// Ids of the units this call produced. A caller fetches a chunk back by
    /// these, so a write that names none is unusable even when the count is
    /// right.
    pub chunk_ids: Vec<String>,
    /// Follow-up extraction jobs this call scheduled. Read next to
    /// [`Self::chunks_written`] it answers whether the material just handed
    /// over will be picked up at all — rows can land with nothing scheduled to
    /// derive from them, and the write count alone reports that as success.
    pub extract_jobs_enqueued: usize,
    /// True when the call was a no-op because `(source_kind, source_id)` had
    /// been ingested before.
    ///
    /// Distinct from a zero-write result, and the distinction is the point:
    /// only a refusal is a reason to go and clear the source gate. The gate is
    /// keyed on the logical source rather than on the content, so re-sending
    /// *changed* material under a claimed `source_id` also writes nothing.
    pub already_ingested: bool,
}

/// Build the validation error returned when an ingest payload does not match
/// the canonicaliser schema for its `source_kind`.
///
/// Kept as the single construction site so the wording cannot drift away from
/// [`is_invalid_ingest_payload_message`], which the transport layer uses to
/// pick the Sentry severity. Same emit-site/classifier pairing as
/// `dispatch::UNKNOWN_METHOD_PREFIX` / `dispatch::unknown_method_name`.
fn invalid_payload_message(source_kind: SourceKind, err: &serde_json::Error) -> String {
    format!("invalid {} payload: {err}", source_kind.as_str())
}

/// Returns `true` when `message` is an ingest-payload schema-validation
/// failure produced by `invalid_payload_message`.
///
/// Such a failure is a **caller** error — the submitted JSON does not match
/// the canonicaliser's shape — not a core defect. The handler already returns
/// a precise, actionable JSON-RPC error naming the offending field, and no
/// core-side change can fix a producer that sends the wrong shape. Reporting
/// it at Sentry *error* severity therefore pages on someone else's payload
/// bug: #5169 (`CORE-RUST-1P0`) was 14 such events for a chat batch whose
/// messages omitted `timestamp`.
///
/// The transport layer demotes these to a warn-level capture — still recorded
/// for triage, because a spike genuinely means a producer regressed, but not
/// an error event. See `core::jsonrpc::rpc_handler`.
///
/// Anchored on the exact `invalid <kind> payload: ` prefix rather than a
/// loose `"invalid"` substring so unrelated failures keep paging.
pub fn is_invalid_ingest_payload_message(message: &str) -> bool {
    let Some(rest) = message.strip_prefix("invalid ") else {
        return false;
    };
    // Enumerated rather than parsed so a new `SourceKind` that forgets to
    // update this list stays *loud* (keeps paging) instead of silently
    // inheriting the demotion. The `all_source_kinds_are_recognised_*` test
    // below pins that every variant reachable from `ingest_rpc` is covered.
    [SourceKind::Chat, SourceKind::Email, SourceKind::Document]
        .iter()
        .any(|k| rest.starts_with(&format!("{} payload: ", k.as_str())))
}

/// Which `Ingest` member a canonicalised request lands on.
///
/// One type rather than two resolve sites, so the family lookup and its
/// refusal wording exist once: the arms differ only in which member they call.
enum DriverIngest {
    Chat(Vec<IngestItem>),
    Email(Vec<IngestItem>),
    Document(IngestItem),
}

/// Hand canonicalised items to the bound driver's `Ingest` family.
///
/// A missing family is **refused**, not degraded. The read handlers below
/// answer empty for one because "this driver stores no chunks" is a true
/// answer to what they were asked; this is a write, and the only empty answer
/// available to it — zero written, zero dropped — is byte-identical to a
/// successful ingest of nothing, so the content would go on the floor while
/// the caller was told it landed.
///
/// Resolved on `provider()`, the same seam [`store_stats`] and [`queue_stats`]
/// use, and deliberately not on `binding.guard()`: the guard's `Ingest`
/// decorator re-stamps `taint`, and `ExternalSync` — what it stamps whenever a
/// source scope is active — is exactly what the driver's own
/// `validate_ingest_item` refuses for the chunk tier, so routing this path
/// through the guard would fail every ingest issued during a sync.
async fn ingest_through_driver(
    config: &Config,
    call: DriverIngest,
) -> Result<IngestOutcome, String> {
    let binding = crate::openhuman::memory::binding::for_config(config)?;
    let Some(ingest) = binding.provider().as_ingest() else {
        return Err(format!(
            "ingest: driver '{}' does not serve Ingest",
            binding.driver_id()
        ));
    };
    match call {
        DriverIngest::Chat(messages) => ingest.ingest_chat(messages).await,
        DriverIngest::Email(messages) => ingest.ingest_email(messages).await,
        DriverIngest::Document(item) => ingest.ingest_document(item).await,
    }
    .map_err(|e| format!("ingest: {e}"))
}

/// The [`DataSource`] a chat payload's `platform` string names.
///
/// The verbatim string still crosses as `IngestItem::platform`, which is what
/// the driver rebuilds `ChatBatch::platform` from, so this enum only has to
/// agree with the arm it came from. An unrecognised platform is `Conversation`
/// — the generic chat member — rather than a rejection: this RPC has always
/// taken any platform string, and a new integration must not start failing at
/// the seam because its name is not in an enum here.
fn chat_data_source(platform: &str) -> DataSource {
    match DataSource::parse(platform) {
        Ok(source) if source.kind() == SourceKind::Chat => source,
        _ => DataSource::Conversation,
    }
}

/// As [`chat_data_source`], for a mail payload's `provider`.
///
/// `OtherEmail` is the fallback because it is the mail member that means "an
/// email provider this enum does not name", which is the honest reading of a
/// provider string that did not parse. Demoting to a non-mail member would put
/// the thread under the wrong `SourceKind` entirely.
fn email_data_source(provider: &str) -> DataSource {
    match DataSource::parse(provider) {
        Ok(source) if source.kind() == SourceKind::Email => source,
        _ => DataSource::OtherEmail,
    }
}

/// Canonicalise a mail thread into contract items.
///
/// **This is the exact inverse of the driver's reconstruction**, and it has to
/// be: the driver rebuilds an `EmailThread` from these fields and then calls
/// the same `ingest_pipeline::ingest_email` this arm used to call directly, so
/// any field that does not round-trip changes what is stored rather than
/// failing. Every one it reads through `unwrap_or` is therefore sent as
/// `Some`, so no fallback on the far side can substitute a different value:
///
/// | driver reads | sent here |
/// | --- | --- |
/// | `platform.unwrap_or(source)` | `platform: Some(provider)` |
/// | `channel_label.unwrap_or(source_id)` | `channel_label: Some(thread_subject)` |
/// | `author.unwrap_or(owner)` | `author: Some(from)` |
/// | `subject.unwrap_or(thread_subject)` | `subject: Some(subject)` |
/// | `timestamp.unwrap_or(now)` | `timestamp: Some(sent_at)` |
///
/// `to`, `cc` and `list_unsubscribe` cross verbatim. The unsubscribe header
/// matters more than it looks: an unsubscribe flow reads it back out of stored
/// mail, so dropping it makes that flow impossible rather than merely poorer.
///
/// # Empty bodies are filtered, not refused
///
/// `validate_ingest_item` answers `Invalid` for empty content, and the driver
/// validates every item before ingesting any — so one body-less message would
/// fail the whole thread, where the in-process pipeline wrote the rest of it.
/// A header-only message is entirely plausible in mail, so filtering first
/// (exactly as [`chat_items`] does) keeps the old behaviour instead of turning
/// a survivable thread into a rejected one.
fn email_items(
    source_id: &str,
    owner: &str,
    tags: &[String],
    thread: EmailThread,
) -> Vec<IngestItem> {
    let EmailThread {
        provider,
        thread_subject,
        messages,
    } = thread;
    let source = email_data_source(&provider);
    messages
        .into_iter()
        .filter(|message| !message.body.trim().is_empty())
        .map(|message| IngestItem {
            namespace: None,
            source,
            source_id: source_id.to_string(),
            owner: owner.to_string(),
            source_ref: message.source_ref.map(SourceRef::new),
            content: message.body,
            mime: None,
            timestamp: Some(message.sent_at),
            tags: tags.to_vec(),
            taint: MemoryTaint::Internal,
            path_scope: None,
            author: Some(message.from),
            channel_label: Some(thread_subject.clone()),
            to: message.to,
            cc: message.cc,
            subject: Some(message.subject),
            list_unsubscribe: message.list_unsubscribe,
            platform: Some(provider.clone()),
        })
        .collect()
}

/// As [`chat_data_source`], for a document payload's `provider`.
///
/// `Upload` is the fallback because it is the member that means "handed to the
/// memory layer directly, with no upstream to re-read it from", which is what
/// a document arriving over this RPC is.
fn document_data_source(provider: &str) -> DataSource {
    match DataSource::parse(provider) {
        Ok(source) if source.kind() == SourceKind::Document => source,
        _ => DataSource::Upload,
    }
}

/// Canonicalise a chat batch into the contract's items.
///
/// The attribution trio is what keeps the stored rows identical to the ones
/// the in-process pipeline wrote. The driver rebuilds a `ChatBatch` from the
/// **first** item's `platform` and `channel_label` and from each item's
/// `author`, falling back to values that are not this payload's (the
/// `DataSource` name and the `source_id`), so all three are set on every item
/// rather than only where they differ.
///
/// Messages whose text trims to empty are dropped before the call.
/// `validate_ingest_item` answers `Invalid` for empty content and the driver
/// checks every item before ingesting any, so one attachment-only message
/// would fail the whole batch where the in-process pipeline wrote the rest of
/// it. What the filter costs is that message's bare `## <ts> — <author>`
/// header, which carried no content. Same filter, for the same reason, as
/// `agent::harness::archivist::tree_ingest`.
fn chat_items(source_id: &str, owner: &str, tags: &[String], batch: ChatBatch) -> Vec<IngestItem> {
    let ChatBatch {
        platform,
        channel_label,
        messages,
    } = batch;
    let source = chat_data_source(&platform);
    messages
        .into_iter()
        .filter(|message| !message.text.trim().is_empty())
        .map(|message| IngestItem {
            namespace: None,
            source,
            source_id: source_id.to_string(),
            owner: owner.to_string(),
            source_ref: message.source_ref.map(SourceRef::new),
            content: message.text,
            // The payload carries no MIME. `None` is the honest answer — the
            // driver validates what it is told, and naming a type here would
            // be asserting one the caller never claimed.
            mime: None,
            timestamp: Some(message.timestamp),
            tags: tags.to_vec(),
            taint: MemoryTaint::Internal,
            path_scope: None,
            author: Some(message.author),
            channel_label: Some(channel_label.clone()),
            // Mail-only fields; this source is not mail, and the contract
            // documents empty/absent as the same statement as "not mail".
            to: Vec::new(),
            cc: Vec::new(),
            subject: None,
            list_unsubscribe: None,
            platform: Some(platform.clone()),
        })
        .collect()
}

/// Canonicalise a document payload into the contract's single item.
///
/// `title` does not cross, and the contract's `ingest_document` hard-codes an
/// empty one on the way back in. Nothing is lost by that: the document
/// canonicaliser writes the body alone into the stored markdown and carries
/// the title nowhere else — it reads it only to decide whether the payload was
/// wholly empty, which [`ingest_rpc`] now answers before the call.
fn document_item(source_id: &str, owner: &str, tags: &[String], doc: DocumentInput) -> IngestItem {
    IngestItem {
        namespace: None,
        source: document_data_source(&doc.provider),
        source_id: source_id.to_string(),
        owner: owner.to_string(),
        source_ref: doc.source_ref.map(SourceRef::new),
        content: doc.body,
        mime: None,
        timestamp: Some(doc.modified_at),
        tags: tags.to_vec(),
        taint: MemoryTaint::Internal,
        // This handler called `ingest_document_with_scope(.., None)`, so the
        // scope stays unset rather than falling back to `source_id`.
        path_scope: None,
        author: None,
        channel_label: None,
        // Mail-only fields; this source is not mail, and the contract
        // documents empty/absent as the same statement as "not mail".
        to: Vec::new(),
        cc: Vec::new(),
        subject: None,
        list_unsubscribe: None,
        platform: None,
    }
}

/// Map the driver's outcome onto the wire body.
///
/// `source_id` comes from the request, not from the outcome echoing it back:
/// the caller supplied it, and reading it off the producer is what would let a
/// reply be paired with the wrong request if a producer ever normalised it.
///
/// `written` / `skipped` are the contract's names for the two counts this wire
/// calls `chunks_written` / `chunks_dropped`, and they carry the same two
/// facts: `skipped` counts dropped units only, a refused call being
/// `already_ingested` instead, which is what `chunks_dropped` has always meant
/// here.
fn response_from_outcome(source_id: String, outcome: IngestOutcome) -> IngestResponse {
    IngestResponse {
        source_id,
        chunks_written: outcome.written as usize,
        chunks_dropped: outcome.skipped as usize,
        chunk_ids: outcome.ids,
        extract_jobs_enqueued: outcome.extract_jobs_enqueued as usize,
        already_ingested: outcome.already_ingested,
    }
}

/// Unified ingest RPC handler. Dispatches on `source_kind`.
///
/// Chat and document go to the bound driver's `Ingest` family. At the v1.3.0
/// module pin they could not: the contract's `IngestOutcome` carried neither
/// `already_ingested` nor `extract_jobs_enqueued` (both would have decoded to
/// their serde defaults, reporting every duplicate submission as a plain empty
/// write, forever) and its `skipped` counted duplicate-refusals rather than
/// dropped units, so `chunks_dropped` would have carried a different fact
/// under the same name. v1.4.0 closes all three, which is why the swap is a
/// swap and not a wire change —
/// `the_response_body_serialises_exactly_as_the_engine_summary` still holds.
///
/// **Mail is still on the in-process pipeline, and it is the last thing in this
/// file that is** (#5560). This paragraph used to list two blockers, and
/// **both are closed** — re-checked 2026-08-25 rather than inherited, because
/// leaving them stated is what would stop the next reader from finishing it:
///
/// 1. *"`IngestItem` carries no recipient list, no per-message subject and no
///    `List-Unsubscribe`."* It carries all of them now, plus `platform`, and
///    the contract's own field docs are written for this path — they say
///    dropping the unsubscribe header makes the unsubscribe flow impossible
///    rather than merely less complete. `chat_items` and `document_item`
///    already spell the five mail fields as their not-mail values.
/// 2. *"`ModuleMemoryProvider` does not forward `IngestEmail`."* It does —
///    `modules::memory` implements `ingest_email` beside `ingest_document` and
///    `ingest_chat`, and the module declares the member.
///
/// The driver's mapping is a lossless inverse: it rebuilds `EmailThread` with
/// `provider` from `platform`, `thread_subject` from `channel_label`, `from`
/// from `author`, and `to` / `cc` / `subject` / `list_unsubscribe` verbatim,
/// then calls the same `ingest_pipeline::ingest_email` this arm calls now.
///
/// So what is left is host work plus one verification, and neither is a
/// contract change:
///
/// - An `email_items(source_id, owner, tags, EmailThread)` mapper that is the
///   **exact** inverse of that reconstruction (`platform: Some(provider)`,
///   `channel_label: Some(thread_subject)`, `author: Some(from)`,
///   `subject: Some(msg.subject)` — always `Some`, so no `unwrap_or` fallback
///   on the far side can substitute a different value), a `DataSource` chooser
///   in the shape of `chat_data_source`, and a `DriverIngest::Email` arm.
/// - A decision on empty bodies, which is a real behaviour delta and wants its
///   own test: `validate_ingest_item` answers `Invalid` for empty content and
///   the driver validates every item before ingesting any, so one body-less
///   message fails the whole thread where the in-process pipeline writes the
///   rest of it. The chat arm answers this by filtering first; mail has to
///   choose the same, and a header-only message is more plausible in mail.
/// - **Check the released artifact, not the submodule.** The five mail fields
///   are `#[serde(default)]`, so a pinned module that predates them decodes
///   every one to its default and mail loses its headers **silently** — the
///   capability check stays green because the family and the member both
///   exist. `vendor/tinymemory` is currently *behind* the pinned release, so
///   grep the tag.
pub async fn ingest_rpc(
    config: &Config,
    req: IngestRequest,
) -> Result<RpcOutcome<IngestResponse>, String> {
    let IngestRequest {
        source_kind,
        source_id,
        owner,
        tags,
        payload,
    } = req;

    log::debug!(
        "[memory::rpc] ingest kind={} source_id={}",
        source_kind.as_str(),
        source_id
    );

    let response = match source_kind {
        SourceKind::Chat => {
            let batch: ChatBatch = serde_json::from_value(payload).map_err(|e| {
                let msg = invalid_payload_message(SourceKind::Chat, &e);
                log::warn!("[memory::rpc] invalid payload for chat");
                msg
            })?;
            let messages = chat_items(&source_id, &owner, &tags, batch);
            if messages.is_empty() {
                // Answered here rather than handed over. The in-process
                // pipeline returned an empty summary for a batch it could not
                // canonicalise, and "the driver returns zeros for zero items"
                // is a behaviour of the one driver we ship, not something the
                // contract promises of the next one.
                response_from_outcome(source_id.clone(), IngestOutcome::default())
            } else {
                let outcome = ingest_through_driver(config, DriverIngest::Chat(messages))
                    .await
                    .inspect_err(|_| {
                        log::warn!("[memory::rpc] chat ingestion failed");
                    })?;
                response_from_outcome(source_id.clone(), outcome)
            }
        }
        SourceKind::Email => {
            let thread: EmailThread = serde_json::from_value(payload).map_err(|e| {
                let msg = invalid_payload_message(SourceKind::Email, &e);
                log::warn!("[memory::rpc] invalid payload for email");
                msg
            })?;
            let messages = email_items(&source_id, &owner, &tags, thread);
            if messages.is_empty() {
                // The chat arm's answer, for the same reason: a thread whose
                // every message was body-less canonicalises to nothing, and
                // "the driver returns zeros for zero items" is a behaviour of
                // the one driver we ship rather than a contract promise.
                response_from_outcome(source_id.clone(), IngestOutcome::default())
            } else {
                let outcome = ingest_through_driver(config, DriverIngest::Email(messages))
                    .await
                    .inspect_err(|_| {
                        log::warn!("[memory::rpc] email ingestion failed");
                    })?;
                response_from_outcome(source_id.clone(), outcome)
            }
        }
        SourceKind::Document => {
            let doc: DocumentInput = serde_json::from_value(payload).map_err(|e| {
                let msg = invalid_payload_message(SourceKind::Document, &e);
                log::warn!("[memory::rpc] invalid payload for document");
                msg
            })?;
            let item = document_item(&source_id, &owner, &tags, doc);
            if item.content.trim().is_empty() {
                // The chat arm's filter, on this arm's single item. Empty
                // content is `Invalid` at the driver, so a body-less document
                // would arrive as a failed call; the in-process pipeline
                // instead canonicalised it to a lone whitespace chunk. Neither
                // is worth keeping — "nothing to ingest" is. What it gives up
                // is that a body-less payload under an already-claimed
                // `source_id` used to answer `already_ingested`, and there is
                // nothing behind that gate to go and clear when the body was
                // empty.
                response_from_outcome(source_id.clone(), IngestOutcome::default())
            } else {
                let outcome = ingest_through_driver(config, DriverIngest::Document(item))
                    .await
                    .inspect_err(|_| {
                        log::warn!("[memory::rpc] document ingestion failed");
                    })?;
                response_from_outcome(source_id.clone(), outcome)
            }
        }
    };

    Ok(RpcOutcome::single_log(
        response,
        format!(
            "memory_tree: ingest kind={} source_id={source_id}",
            source_kind.as_str()
        ),
    ))
}

/// Query shape for the `list_chunks` RPC.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ListChunksRequest {
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub since_ms: Option<i64>,
    #[serde(default)]
    pub until_ms: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Response shape for the `list_chunks` RPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListChunksResponse {
    pub chunks: Vec<Chunk>,
}

/// `list_chunks` RPC handler. Filters and returns persisted chunks ordered by
/// timestamp DESC.
///
/// `scope` is `None`, which is not an oversight: this listing has never
/// applied the per-turn source allowlist — the engine query it replaced set
/// `source_scope: None` — and it is reached by inspection surfaces rather than
/// by an agent turn. Narrowing it here would be a policy change wearing a
/// routing change's clothes.
pub async fn list_chunks_rpc(
    config: &Config,
    req: ListChunksRequest,
) -> Result<RpcOutcome<ListChunksResponse>, String> {
    // Parsed before the driver is resolved so an unknown kind stays a caller
    // error naming the offending value, rather than a driver round trip that
    // returns nothing and looks like an empty store.
    let query = ChunkQuery {
        source_kind: match req.source_kind.as_deref() {
            None => None,
            Some(s) => Some(SourceKind::parse(s)?),
        },
        source_id: req.source_id,
        owner: req.owner,
        since_ms: req.since_ms,
        until_ms: req.until_ms,
        limit: req.limit,
        offset: None,
        exclude_dropped: false,
        // The filtered-listing predicates this request does not carry. An empty
        // predicate is unfiltered, so the defaults leave the query exactly as
        // narrow as the fields above already make it.
        ..Default::default()
    };

    // No `spawn_blocking`: the driver owns whether its own reads block, and the
    // module's do not run on this thread at all.
    let binding = crate::openhuman::memory::binding::for_config(config)?;
    let rows = match binding.provider().as_chunks() {
        Some(chunks) => chunks
            .list_chunks(&query, None)
            .await
            .map_err(|e| format!("list_chunks: {e}"))?,
        // Read-only, so an empty page is the honest answer: a driver with no
        // chunk tier holds no rows to list, which is a true statement about it
        // rather than a fault the caller can act on.
        None => {
            log::debug!(
                "[memory-tree][rpc] list_chunks: driver '{}' does not serve Chunks; reporting empty",
                binding.driver_id()
            );
            Vec::new()
        }
    };

    let n = rows.len();
    Ok(RpcOutcome::single_log(
        ListChunksResponse { chunks: rows },
        format!("memory_tree: list_chunks n={n}"),
    ))
}

/// Request shape for the `get_chunk` RPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetChunkRequest {
    pub id: String,
}

/// Response shape for the `get_chunk` RPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetChunkResponse {
    pub chunk: Option<Chunk>,
}

/// `get_chunk` RPC handler. Returns the chunk identified by `id`, or `None`.
pub async fn get_chunk_rpc(
    config: &Config,
    req: GetChunkRequest,
) -> Result<RpcOutcome<GetChunkResponse>, String> {
    let binding = crate::openhuman::memory::binding::for_config(config)?;
    let chunk = match binding.provider().as_chunks() {
        Some(chunks) => chunks
            .get_chunk(&req.id)
            .await
            .map_err(|e| format!("get_chunk: {e}"))?,
        // `None` is already this handler's answer for an id the store does not
        // hold, and a driver with no chunk tier holds none — so the degrade is
        // indistinguishable from the ordinary miss, which is what makes it safe
        // here and not on a write.
        None => {
            log::debug!(
                "[memory-tree][rpc] get_chunk: driver '{}' does not serve Chunks; reporting none",
                binding.driver_id()
            );
            None
        }
    };
    Ok(RpcOutcome::single_log(
        GetChunkResponse { chunk },
        format!("memory_tree: get_chunk id={}", req.id),
    ))
}

// ── Driver diagnostics ───────────────────────────────────────────────────
//
// The numbers below used to come from `SELECT`s against TinyCortex's tables.
// They come from the bound driver now, which is what lets a workspace run on
// a driver that is not TinyCortex and still answer "how far behind is the
// pipeline".

/// The driver's identifier for a re-embed backfill job.
///
/// Job kinds are the driver's own vocabulary, not the contract's — a driver
/// that never enqueues this one answers zero for it, which is the honest
/// count and exactly what a status poll wants to hear.
const REEMBED_BACKFILL_KIND: &str = "reembed_backfill";
