import type {
  ThreadAssistantMessagePart,
  ThreadMessageLike,
  ThreadUserMessagePart,
} from '@assistant-ui/react';

import { parseMessageImages } from '../lib/attachments';
import { unwrapToolCallEnvelope } from '../lib/chat/toolCallEnvelope';
import type {
  ProcessingTranscriptItem,
  StreamingAssistantState,
  ToolTimelineEntry,
} from '../store/chatRuntimeSlice';
import type { ThreadMessage } from '../types/thread';

/**
 * Redux -> assistant-ui message mapping.
 *
 * assistant-ui is adopted as a *runtime* (semantics + API), never as a store:
 * `chatRuntimeSlice` and `threadSlice` remain the single source of truth for
 * messages, streaming, tool state, queueing and persistence. Everything here is
 * a pure, read-only projection of that state onto the shape the runtime wants.
 * Nothing in this module writes.
 *
 * The one property that matters for performance is stated as a test, not a
 * comment: converting the transcript while a token streams must not re-convert
 * the settled messages above the live tail. `ChatThreadView.renderPerf.test.tsx`
 * pins the equivalent property for the render tree; `assistantUiMessages.test.ts`
 * pins it for this projection.
 */

type ConversionCacheEntry = {
  timeline: readonly ToolTimelineEntry[];
  transcript: readonly ProcessingTranscriptItem[];
  converted: ThreadMessageLike;
};

/**
 * Cache keyed on the source message and its persisted process arrays. Socket
 * tokens only replace the live tail, so a settled message converts exactly
 * once while its transcript/timeline identities remain stable.
 */
const conversionCache = new WeakMap<ThreadMessage, ConversionCacheEntry>();

const EMPTY_TIMELINE: readonly ToolTimelineEntry[] = [];
const EMPTY_TRANSCRIPT: readonly ProcessingTranscriptItem[] = [];

const RECOVERED_TOOL_NAMES_KEY = 'assistantUiToolNames';

/** Synthetic id for the live streaming tail. Stable so React reconciles it. */
export const STREAMING_TAIL_ID = '__openhuman_streaming_tail__';

/**
 * Convert one persisted message.
 *
 * Agent content is passed through `unwrapToolCallEnvelope` for the same reason
 * the transcript renderer does it: a `{content, tool_calls}` provider envelope
 * must never reach a rendered surface as raw JSON. Tool *activity* is not
 * projected as assistant-ui tool-call parts — it lives in the far richer
 * `toolTimelineByThread` projection that `ToolTimelineBlock` renders, and
 * duplicating it here would paint every tool twice.
 */
function jsonObject(value: unknown): Record<string, never> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  try {
    return JSON.parse(JSON.stringify(value)) as Record<string, never>;
  } catch {
    return {};
  }
}

function toolArgs(entry: ToolTimelineEntry): Record<string, never> {
  if (!entry.argsBuffer) return {};
  try {
    return jsonObject(JSON.parse(entry.argsBuffer));
  } catch {
    return { raw: entry.argsBuffer } as never;
  }
}

/**
 * The `result` payload for a settled non-sub-agent tool part.
 *
 * assistant-ui's tool-call part has no status field, so a terminal status has
 * to travel inside `result` or not at all. It did not travel: the adapter fell
 * back to `result !== undefined`, which reads as success, and a failed or
 * cancelled tool rendered with a "done" label and a check.
 *
 * A tool that produced no output already reported `{ status, failure }` here,
 * so only the failed-*with*-output case needed a shape — `value` carries the
 * real output beside the status, and {@link isToolStatusEnvelope} unwraps it.
 * The success path is byte-identical to before, deliberately: every reader of
 * a successful result keeps seeing exactly what it saw.
 */
function toolResultPayload(entry: ToolTimelineEntry): unknown {
  const terminalFailure = entry.status === 'error' || entry.status === 'cancelled';
  if (!terminalFailure) return entry.result ?? { status: entry.status, failure: entry.failure };
  return {
    status: entry.status,
    failure: entry.failure,
    ...(entry.result !== undefined ? { value: entry.result } : {}),
  };
}

function toolPart(entry: ToolTimelineEntry): ThreadAssistantMessagePart {
  const running = entry.status === 'running' || entry.status === 'awaiting_user';
  const isSubagent = entry.name.startsWith('subagent:') || entry.subagent !== undefined;
  const args = isSubagent
    ? jsonObject({
        subagent_type: entry.subagent?.agentId ?? entry.name.replace(/^subagent:/, ''),
        description: entry.detail,
        ...(running ? { progress: entry.subagent } : {}),
      })
    : toolArgs(entry);

  return {
    type: 'tool-call',
    toolCallId: entry.id,
    toolName: isSubagent ? 'task' : entry.name,
    args,
    argsText: JSON.stringify(args, null, 2),
    ...(!running
      ? {
          result: isSubagent
            ? (entry.subagent ?? { status: entry.status })
            : toolResultPayload(entry),
        }
      : {}),
  };
}

/**
 * Project one assistant message into assistant-ui parts.
 *
 * **Every tool part must have a distinct `toolCallId`.** assistant-ui keys them
 * as `toolCallId-${id}` and *throws* on a repeat ("Duplicate key … in
 * useResources"), which takes the whole thread render down rather than dropping
 * a row — so this is a hard invariant, not a tidiness rule, and it is enforced
 * here at the boundary as well as at each producer. `emittedToolIds` guards
 * both passes below; the sources upstream (the live Redux slice and the derived
 * transcript mapper) also mint unique ids, but threads persisted before those
 * fixes still carry colliding ones.
 */
function assistantParts(
  text: string,
  timeline: readonly ToolTimelineEntry[],
  transcript: readonly ProcessingTranscriptItem[]
): ThreadAssistantMessagePart[] {
  const parts: ThreadAssistantMessagePart[] = [];
  const timelineById = new Map(timeline.map(entry => [entry.id, entry]));
  const emittedToolIds = new Set<string>();

  for (const item of transcript) {
    if (item.kind === 'thinking') {
      if (item.text.trim().length > 0) parts.push({ type: 'reasoning', text: item.text });
      continue;
    }
    if (item.kind === 'toolCall') {
      const entry = timelineById.get(item.callId);
      // Guarded here too, not only in the timeline pass below: two transcript
      // pointers can name the same `callId` (a provider that emits tool calls
      // without ids writes the empty string for all of them), and both resolve
      // to the same timeline row.
      if (entry && !emittedToolIds.has(entry.id)) {
        emittedToolIds.add(entry.id);
        parts.push(toolPart(entry));
      }
    }
    if (
      item.kind === 'narration' &&
      item.text.trim().length > 0 &&
      !text.trim().includes(item.text.trim())
    ) {
      // Narration emitted before a tool call is assistant content in its own
      // right. Keep it inline in assistant-ui's ordered part stream; the final
      // answer is appended separately below. A final-round narration is the
      // same streamed bytes as that answer and must not render twice —
      // `includes`, not equality, because `mergedAssistantText` prefers the
      // longest text when it *contains* every segment, so the duplicate is a
      // substring rather than an exact match.
      parts.push({ type: 'text', text: item.text });
    }
  }

  for (const entry of [...timeline].sort((a, b) => a.seq - b.seq)) {
    if (emittedToolIds.has(entry.id)) continue;
    emittedToolIds.add(entry.id);
    parts.push(toolPart(entry));
  }
  if (text.length > 0) parts.push({ type: 'text', text });
  return parts;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : [];
}

function requestIdOf(message: ThreadMessage): string | undefined {
  const requestId = message.extraMetadata?.requestId;
  return typeof requestId === 'string' && requestId.length > 0 ? requestId : undefined;
}

function isGenericToolName(name: string): boolean {
  return ['', 'tool', 'unknown', 'unknown_tool'].includes(name.trim().toLowerCase());
}

function recoverTimelineToolNames(
  timeline: readonly ToolTimelineEntry[],
  recoveredNames: readonly string[]
): readonly ToolTimelineEntry[] {
  if (recoveredNames.length === 0 || !timeline.some(entry => isGenericToolName(entry.name))) {
    return timeline;
  }
  // Advance only when a name is actually consumed. `recoveredNames` comes from
  // tool-call envelopes, so it is not positionally aligned with the whole
  // timeline: incrementing on every entry made `[read_file, tool, tool]` +
  // `[web_search, web_fetch]` mis-assign `web_fetch` to the first generic row
  // and leave the second one named `tool`.
  let recoveredIndex = 0;
  return timeline.map(entry => {
    if (!isGenericToolName(entry.name)) return entry;
    const recovered = recoveredNames[recoveredIndex];
    if (!recovered) return entry;
    recoveredIndex += 1;
    return { ...entry, name: recovered };
  });
}

function mergedAssistantText(messages: readonly ThreadMessage[]): string {
  const texts = messages
    .map(message => unwrapToolCallEnvelope(message.content ?? '').text)
    .filter(text => text.trim().length > 0)
    .filter((text, index, all) => all.indexOf(text) === index);
  if (texts.length < 2) return texts[0] ?? '';

  // Legacy web delivery persisted both paragraph-sized segments and the full
  // final response. Prefer the complete response when it contains every
  // segment; otherwise retain each distinct assistant emission in order.
  const longest = [...texts].sort((left, right) => right.length - left.length)[0] ?? '';
  if (texts.every(text => longest.includes(text.trim()))) return longest;
  return texts.join('\n\n');
}

function mergeAssistantRun(messages: readonly ThreadMessage[]): ThreadMessage {
  if (messages.length === 1) return messages[0];
  const first = messages[0];
  const last = messages[messages.length - 1];
  const extraMetadata = Object.assign({}, ...messages.map(message => message.extraMetadata));
  const requestId = messages.map(requestIdOf).find(Boolean);
  const toolNames = messages.flatMap(
    message => unwrapToolCallEnvelope(message.content ?? '').toolNames
  );
  if (requestId) extraMetadata.requestId = requestId;
  if (toolNames.length > 0) extraMetadata[RECOVERED_TOOL_NAMES_KEY] = toolNames;
  return {
    ...last,
    content: mergedAssistantText(messages),
    createdAt: first.createdAt,
    extraMetadata,
  };
}

/**
 * A row the core persisted as a complete, self-contained delivery — an
 * autonomous task result, a worker-thread hand-off, a workflow proposal. Core
 * writers stamp `extraMetadata.scope` on every such row; the legacy segmented
 * path never did. That positive marker is what tells an async delivery apart
 * from a paragraph of the answer next to it, since neither carries a request
 * id on the wire.
 */
function isStandaloneDelivery(message: ThreadMessage): boolean {
  return typeof message.extraMetadata?.scope === 'string';
}

/**
 * Collapse legacy paragraph/tool-envelope rows into one assistant turn.
 *
 * The old interactive-web delivery path persisted each segment as a separate
 * agent message. Consecutive assistant rows cannot cross a user turn; when
 * both rows carry request ids, a differing id is the explicit boundary, and a
 * scoped standalone delivery ({@link isStandaloneDelivery}) is always its own
 * turn — it neither joins the run before it nor seeds the run after it.
 */
function coalesceAssistantSegments(messages: readonly ThreadMessage[]): ThreadMessage[] {
  const out: ThreadMessage[] = [];
  let run: ThreadMessage[] = [];
  let runRequestId: string | undefined;

  const flush = () => {
    if (run.length > 0) out.push(mergeAssistantRun(run));
    run = [];
    runRequestId = undefined;
  };

  for (const message of messages) {
    if (message.sender !== 'agent' || message.extraMetadata?.hidden) {
      flush();
      out.push(message);
      continue;
    }
    if (isStandaloneDelivery(message)) {
      flush();
      run.push(message);
      flush();
      continue;
    }
    const requestId = requestIdOf(message);
    if (run.length > 0 && runRequestId && requestId && runRequestId !== requestId) flush();
    run.push(message);
    runRequestId ??= requestId;
  }
  flush();
  return out;
}

function mimeTypeFromDataUri(dataUri: string): string {
  return dataUri.match(/^data:([^;,]+)/i)?.[1] ?? 'application/octet-stream';
}

function userParts(msg: ThreadMessage): ThreadUserMessagePart[] {
  const parsed = parseMessageImages(msg.content ?? '');
  const metadata = msg.extraMetadata ?? {};
  const kinds = stringArray(metadata.attachmentKinds);
  const names = stringArray(metadata.attachmentNames);
  const posters = stringArray(metadata.attachmentPosters);
  const metadataUris = stringArray(metadata.attachmentDataUris);
  const dataUris = metadataUris.length > 0 ? metadataUris : parsed.dataUris;
  const parts: ThreadUserMessagePart[] = [];

  if (parsed.text.length > 0) parts.push({ type: 'text', text: parsed.text });

  if (kinds.length === 0) {
    for (const [index, image] of dataUris.entries()) {
      parts.push({ type: 'image', image, filename: names[index] });
    }
    return parts;
  }

  for (const [index, kind] of kinds.entries()) {
    const filename = names[index];
    if (kind === 'image') {
      const image = dataUris[index];
      if (image) parts.push({ type: 'image', image, filename });
      continue;
    }
    if (kind === 'video') {
      const image = posters[index];
      if (image) parts.push({ type: 'image', image, filename });
      else parts.push({ type: 'file', filename, data: '', mimeType: 'video/mp4' });
      continue;
    }
    const data = dataUris[index] ?? '';
    parts.push({ type: 'file', filename, data, mimeType: mimeTypeFromDataUri(data) });
  }
  return parts;
}

export function toThreadMessageLike(
  msg: ThreadMessage,
  timeline: readonly ToolTimelineEntry[] = EMPTY_TIMELINE,
  transcript: readonly ProcessingTranscriptItem[] = EMPTY_TRANSCRIPT
): ThreadMessageLike {
  const cached = conversionCache.get(msg);
  if (cached?.timeline === timeline && cached.transcript === transcript) return cached.converted;

  const unwrapped = unwrapToolCallEnvelope(msg.content ?? '');
  const text = msg.sender === 'agent' ? unwrapped.text : (msg.content ?? '');
  const recoveredToolNames = [
    ...unwrapped.toolNames,
    ...stringArray(msg.extraMetadata?.[RECOVERED_TOOL_NAMES_KEY]),
  ];
  const effectiveTimeline = recoverTimelineToolNames(timeline, recoveredToolNames);

  const converted: ThreadMessageLike = {
    id: msg.id,
    role: msg.sender === 'agent' ? 'assistant' : 'user',
    content:
      msg.sender === 'agent' ? assistantParts(text, effectiveTimeline, transcript) : userParts(msg),
    createdAt: new Date(msg.createdAt),
    ...(msg.sender === 'agent' && msg.extraMetadata?.stopped === true
      ? { status: { type: 'incomplete' as const, reason: 'cancelled' as const } }
      : {}),
    metadata: { custom: { extraMetadata: msg.extraMetadata ?? {}, sourceType: msg.type } },
  };

  conversionCache.set(msg, { timeline, transcript, converted });
  return converted;
}

/**
 * The live tail as a running assistant message.
 *
 * The tail is deliberately NOT part of `thread.messagesByThreadId` — Redux keeps
 * the settled transcript and the in-flight preview in separate slices, which is
 * exactly what keeps settled message identities stable while tokens land. Here
 * that separation is re-joined for the runtime's benefit: one fresh object per
 * token, and only that one object is ever re-converted.
 */
export function streamingTailMessage(
  streaming: StreamingAssistantState | null,
  timeline: readonly ToolTimelineEntry[] = EMPTY_TIMELINE,
  transcript: readonly ProcessingTranscriptItem[] = EMPTY_TRANSCRIPT
): ThreadMessageLike | null {
  if (!streaming && timeline.length === 0 && transcript.length === 0) return null;
  const text = streaming?.content ?? '';
  const parts = assistantParts(text, timeline, transcript);
  if (streaming?.thinking.trim()) {
    const hasTranscriptThinking = transcript.some(item => item.kind === 'thinking');
    if (!hasTranscriptThinking) parts.unshift({ type: 'reasoning', text: streaming.thinking });
  }
  if (parts.length === 0) return null;
  return {
    id: STREAMING_TAIL_ID,
    role: 'assistant',
    content: parts,
    status: { type: 'running' },
    metadata: { custom: { requestId: streaming?.requestId, streaming: true } },
  };
}

export type AssistantUiProjection = {
  /** Whether the synthetic live tail has an active core turn driving it. */
  isRunning?: boolean;
  liveTimeline?: readonly ToolTimelineEntry[];
  liveTranscript?: readonly ProcessingTranscriptItem[];
  turnTimelines?: Readonly<Record<string, readonly ToolTimelineEntry[]>>;
  turnTranscripts?: Readonly<Record<string, readonly ProcessingTranscriptItem[]>>;
};

/**
 * The full thread as assistant-ui sees it: settled transcript, then the live
 * tail when one is in flight.
 *
 * Hidden messages are filtered the same way the transcript filters them, so the
 * runtime's view of the thread and the rendered view cannot disagree about what
 * the conversation contains.
 */
export function buildRuntimeMessages(
  messages: readonly ThreadMessage[],
  streaming: StreamingAssistantState | null,
  projection: AssistantUiProjection = {}
): ThreadMessageLike[] {
  const coalescedMessages = coalesceAssistantSegments(messages);
  const out: ThreadMessageLike[] = [];
  const claimedRequestIds = new Set(
    coalescedMessages.flatMap(message =>
      message.sender === 'agent' && typeof message.extraMetadata?.requestId === 'string'
        ? [message.extraMetadata.requestId]
        : []
    )
  );
  const projectedRequestIds = [
    ...new Set([
      ...Object.keys(projection.turnTimelines ?? {}),
      ...Object.keys(projection.turnTranscripts ?? {}),
    ]),
  ].filter(requestId => !claimedRequestIds.has(requestId));
  // Async acknowledgements/background deliveries can be persisted without
  // message-level request metadata. The transcript maps are chronological and
  // request-keyed, so unclaimed trails can be paired with unanchored agent
  // messages in order — but only when the two sets are the same size. With a
  // surplus of unanchored messages, positional pairing hands a later turn's
  // tools to an earlier trail-less answer and leaves the real answer bare;
  // rendering those trails nowhere is the lesser wrong.
  const unanchoredAgentCount = coalescedMessages.filter(
    message =>
      message.sender === 'agent' &&
      !message.extraMetadata?.hidden &&
      typeof message.extraMetadata?.requestId !== 'string'
  ).length;
  const pairOrphanTrails =
    projectedRequestIds.length > 0 && projectedRequestIds.length === unanchoredAgentCount;
  let orphanRequestCursor = 0;
  const lastVisibleAgentId = [...coalescedMessages]
    .reverse()
    .find(message => message.sender === 'agent' && !message.extraMetadata?.hidden)?.id;
  for (const msg of coalescedMessages) {
    if (msg.extraMetadata?.hidden) continue;
    const requestId =
      msg.sender === 'agent' && typeof msg.extraMetadata?.requestId === 'string'
        ? msg.extraMetadata.requestId
        : undefined;
    const effectiveRequestId =
      requestId ??
      (msg.sender === 'agent' && pairOrphanTrails
        ? projectedRequestIds[orphanRequestCursor++]
        : undefined);
    const persistedTimeline = effectiveRequestId
      ? projection.turnTimelines?.[effectiveRequestId]
      : undefined;
    const persistedTranscript = effectiveRequestId
      ? projection.turnTranscripts?.[effectiveRequestId]
      : undefined;
    // `chat_done` clears the active lifecycle before the completed snapshot is
    // indexed into the request maps. Keep the just-settled tools/reasoning on
    // the final assistant message during that handoff; never mint a running
    // synthetic tail for them.
    const useSettledLiveFallback =
      projection.isRunning === false &&
      msg.id === lastVisibleAgentId &&
      !persistedTimeline &&
      !persistedTranscript;
    out.push(
      toThreadMessageLike(
        msg,
        persistedTimeline ??
          (useSettledLiveFallback ? (projection.liveTimeline ?? EMPTY_TIMELINE) : EMPTY_TIMELINE),
        persistedTranscript ??
          (useSettledLiveFallback
            ? (projection.liveTranscript ?? EMPTY_TRANSCRIPT)
            : EMPTY_TRANSCRIPT)
      )
    );
  }
  const tail =
    projection.isRunning === false
      ? null
      : streamingTailMessage(
          streaming,
          projection.liveTimeline ?? EMPTY_TIMELINE,
          projection.liveTranscript ?? EMPTY_TRANSCRIPT
        );
  if (tail) out.push(tail);
  return out;
}
