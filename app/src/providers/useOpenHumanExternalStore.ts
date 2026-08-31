import type { AppendMessage } from '@assistant-ui/react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { mapDisplayItems } from '../features/conversations/derived/mapDisplayItems';
import { threadApi } from '../services/api/threadApi';
import { useAppSelector } from '../store/hooks';
import type { ThreadMessage } from '../types/thread';
import { buildRuntimeMessages } from './assistantUiMessages';
import { getChatSurface } from './chatSurfaceHandlers';

const EMPTY_MESSAGES: ThreadMessage[] = [];
const EMPTY_TIMELINE: never[] = [];
const EMPTY_TRANSCRIPT: never[] = [];
const EMPTY_TURN_MAP = {};

type CoreTranscriptProjection = {
  threadId: string | null;
  timelines: ReturnType<typeof mapDisplayItems>['timelines'];
  transcripts: ReturnType<typeof mapDisplayItems>['transcripts'];
};

const EMPTY_CORE_TRANSCRIPT: CoreTranscriptProjection = {
  threadId: null,
  timelines: EMPTY_TURN_MAP,
  transcripts: EMPTY_TURN_MAP,
};

/**
 * Read settled process history straight from the core's transcript projection.
 * The Rust side owns a bounded, mtime-keyed LRU, so this hook deliberately does
 * not establish a second Redux transcript store or duplicate cache policy.
 */
export function useCoreTranscriptProjection(
  threadId: string | null,
  revision: string,
  liveRequestId: string | undefined
): CoreTranscriptProjection {
  const [projection, setProjection] = useState<CoreTranscriptProjection>(EMPTY_CORE_TRANSCRIPT);

  useEffect(() => {
    if (!threadId) {
      setProjection(EMPTY_CORE_TRANSCRIPT);
      return;
    }
    // Defensive for narrow test/embedder shims that expose only a subset of
    // threadApi. Production builds always provide this method.
    if (typeof threadApi.getDerivedTranscript !== 'function') {
      setProjection({ threadId, timelines: EMPTY_TURN_MAP, transcripts: EMPTY_TURN_MAP });
      return;
    }
    let cancelled = false;
    void threadApi
      .getDerivedTranscript(threadId, { limit: 500 })
      .then(page => {
        if (cancelled) return;
        if (!page.hasTranscript) {
          setProjection({ threadId, timelines: EMPTY_TURN_MAP, transcripts: EMPTY_TURN_MAP });
          return;
        }
        const skipRequestIds = liveRequestId ? new Set([liveRequestId]) : undefined;
        const mapped = mapDisplayItems(page.items, { skipRequestIds });
        setProjection({ threadId, timelines: mapped.timelines, transcripts: mapped.transcripts });
      })
      .catch(() => {
        // A missing/older core has no settled process trail; message text and
        // the live socket projection remain usable. Navigation must not fail.
        if (!cancelled) {
          setProjection({ threadId, timelines: EMPTY_TURN_MAP, transcripts: EMPTY_TURN_MAP });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [liveRequestId, revision, threadId]);

  return projection.threadId === threadId ? projection : EMPTY_CORE_TRANSCRIPT;
}

/** Flatten an assistant-ui append payload down to the plain text our core takes. */
function appendMessageText(message: AppendMessage): string {
  return message.content
    .map(part => (part.type === 'text' ? part.text : ''))
    .join('')
    .trim();
}

/**
 * Build the `ExternalStoreAdapter` that backs `useExternalStoreRuntime`.
 *
 * Settled messages and live deltas remain in their existing UI stores, while
 * reasoning/tool/sub-agent history comes directly from the core transcript
 * projection. Redux is not a second transcript database.
 */
export function useOpenHumanExternalStore(threadId: string | null) {
  const messages = useAppSelector(state =>
    threadId ? (state.thread.messagesByThreadId[threadId] ?? EMPTY_MESSAGES) : EMPTY_MESSAGES
  );
  const streaming = useAppSelector(state =>
    threadId ? (state.chatRuntime.streamingAssistantByThread?.[threadId] ?? null) : null
  );
  const lifecycle = useAppSelector(state =>
    threadId ? (state.chatRuntime.inferenceTurnLifecycleByThread?.[threadId] ?? null) : null
  );
  const isLoading = useAppSelector(state => Boolean(threadId && state.thread.isLoadingMessages));
  const liveTimeline = useAppSelector(state =>
    threadId
      ? (state.chatRuntime.toolTimelineByThread?.[threadId] ?? EMPTY_TIMELINE)
      : EMPTY_TIMELINE
  );
  const liveTranscript = useAppSelector(state =>
    threadId
      ? (state.chatRuntime.processingByThread?.[threadId] ?? EMPTY_TRANSCRIPT)
      : EMPTY_TRANSCRIPT
  );
  const settledRevision = `${messages.at(-1)?.id ?? ''}:${messages.at(-1)?.content.length ?? 0}:${lifecycle ?? ''}`;
  const coreTranscript = useCoreTranscriptProjection(
    threadId,
    settledRevision,
    streaming?.requestId
  );

  // `started` and `streaming` are both in-flight. A completed turn can retain
  // its tool/reasoning arrays while the persisted projection catches up; those
  // arrays must not mint a forever-running assistant-ui tail.
  const isRunning = lifecycle === 'started' || lifecycle === 'streaming';

  // Recomputed only when the settled transcript or the live tail changes.
  // Settled messages are converted through an identity-keyed cache, so a token
  // landing on the tail re-converts exactly one message, never the transcript.
  const runtimeMessages = useMemo(
    () =>
      buildRuntimeMessages(messages, streaming, {
        isRunning,
        liveTimeline,
        liveTranscript,
        turnTimelines: coreTranscript.timelines,
        turnTranscripts: coreTranscript.transcripts,
      }),
    [messages, streaming, isRunning, liveTimeline, liveTranscript, coreTranscript]
  );

  const onNew = useCallback(
    async (message: AppendMessage) => {
      const surface = getChatSurface(threadId);
      // Fail loudly. A silent no-op here would look like a dropped message.
      if (!surface) {
        throw new Error(`No chat surface registered for thread ${threadId ?? '(none)'}`);
      }
      const text = appendMessageText(message);
      if (text.length === 0) return;
      await surface.send(text);
    },
    [threadId]
  );

  const onCancel = useCallback(async () => {
    await getChatSurface(threadId)?.cancel?.();
  }, [threadId]);

  return useMemo(
    () => ({
      messages: runtimeMessages,
      isRunning,
      isLoading,
      // Already `ThreadMessageLike`; the runtime's converter is the identity.
      convertMessage: (m: (typeof runtimeMessages)[number]) => m,
      onNew,
      onCancel,
    }),
    [runtimeMessages, isRunning, isLoading, onNew, onCancel]
  );
}
