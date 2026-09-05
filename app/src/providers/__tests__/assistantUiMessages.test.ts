import { describe, expect, it, vi } from 'vitest';

import type { ToolTimelineEntry } from '../../store/chatRuntimeSlice';
import type { ThreadMessage } from '../../types/thread';
import {
  buildRuntimeMessages,
  STREAMING_TAIL_ID,
  streamingTailMessage,
  toThreadMessageLike,
} from '../assistantUiMessages';

function msg(over: Partial<ThreadMessage> = {}): ThreadMessage {
  return {
    id: 'm1',
    content: 'hello',
    type: 'text',
    extraMetadata: {},
    sender: 'user',
    createdAt: '2026-01-01T00:00:00.000Z',
    ...over,
  };
}

function tool(over: Partial<ToolTimelineEntry> = {}): ToolTimelineEntry {
  return { id: 'call-1', name: 'web_search', round: 1, seq: 0, status: 'running', ...over };
}

describe('toThreadMessageLike', () => {
  it('maps sender to role', () => {
    expect(toThreadMessageLike(msg({ id: 'u' })).role).toBe('user');
    expect(toThreadMessageLike(msg({ id: 'a', sender: 'agent' })).role).toBe('assistant');
  });

  it('unwraps a tool-call envelope so raw JSON never reaches the runtime', () => {
    const envelope = JSON.stringify({
      content: 'Pulling that up now.',
      tool_calls: [{ id: 'c1', name: 'memory_search', arguments: '{}' }],
    });
    const converted = toThreadMessageLike(msg({ id: 'e', sender: 'agent', content: envelope }));
    expect(converted.content).toEqual([{ type: 'text', text: 'Pulling that up now.' }]);
  });

  it('leaves ordinary prose untouched', () => {
    const m = msg({ id: 'p', sender: 'agent', content: 'just prose { not json' });
    expect(toThreadMessageLike(m).content).toEqual([
      { type: 'text', text: 'just prose { not json' },
    ]);
  });

  it('yields an empty content array for an empty message', () => {
    expect(toThreadMessageLike(msg({ id: 'blank', content: '' })).content).toEqual([]);
  });

  it('carries extraMetadata through as custom metadata', () => {
    const m = msg({ id: 'meta', extraMetadata: { requestId: 'r1' } });
    expect(toThreadMessageLike(m).metadata?.custom).toMatchObject({
      extraMetadata: { requestId: 'r1' },
    });
  });

  it('returns the identical object for the same source message', () => {
    const m = msg({ id: 'cached' });
    expect(toThreadMessageLike(m)).toBe(toThreadMessageLike(m));
  });
});

describe('streamingTailMessage', () => {
  it('is null with no stream and with an empty stream', () => {
    expect(streamingTailMessage(null)).toBeNull();
    expect(streamingTailMessage({ requestId: 'r', content: '', thinking: '' })).toBeNull();
  });

  it('is a running assistant message when tokens have landed', () => {
    const tail = streamingTailMessage({ requestId: 'r', content: 'partial', thinking: '' });
    expect(tail).toMatchObject({
      id: STREAMING_TAIL_ID,
      role: 'assistant',
      status: { type: 'running' },
      content: [{ type: 'text', text: 'partial' }],
    });
  });

  it('projects streamed thinking as a reasoning part before visible text', () => {
    const tail = streamingTailMessage({ requestId: 'r', content: 'answer', thinking: 'reasoning' });
    expect(tail?.content).toEqual([
      { type: 'reasoning', text: 'reasoning' },
      { type: 'text', text: 'answer' },
    ]);
  });

  it('keeps a running delegation on args and adds result only when complete', () => {
    const subagent = {
      taskId: 'sub-1',
      agentId: 'researcher',
      toolCalls: [],
      transcript: [{ kind: 'thinking' as const, text: 'checking sources' }],
    };
    const running = streamingTailMessage(null, [
      tool({ id: 'sub-1', name: 'subagent:researcher', subagent }),
    ]);
    const runningPart = running?.content[0];
    expect(runningPart).toMatchObject({
      type: 'tool-call',
      toolName: 'task',
      args: { progress: subagent },
    });
    expect(runningPart).not.toHaveProperty('result');

    const complete = streamingTailMessage(null, [
      tool({ id: 'sub-1', name: 'subagent:researcher', status: 'success', subagent }),
    ]);
    expect(complete?.content[0]).toMatchObject({
      type: 'tool-call',
      toolName: 'task',
      result: subagent,
    });
  });
});

describe('buildRuntimeMessages', () => {
  it('omits hidden messages', () => {
    const visible = msg({ id: 'v' });
    const hidden = msg({ id: 'h', extraMetadata: { hidden: true } });
    expect(buildRuntimeMessages([visible, hidden], null).map(m => m.id)).toEqual(['v']);
  });

  it('appends the live tail after the settled transcript', () => {
    const ids = buildRuntimeMessages([msg({ id: 'a' })], {
      requestId: 'r',
      content: 'tok',
      thinking: '',
    }).map(m => m.id);
    expect(ids).toEqual(['a', STREAMING_TAIL_ID]);
  });

  it('does not keep a synthetic thinking/tool tail running after lifecycle completion', () => {
    const projected = buildRuntimeMessages([msg({ id: 'answer', sender: 'agent' })], null, {
      isRunning: false,
      liveTimeline: [tool({ id: 'stale-tool', status: 'success' })],
      liveTranscript: [{ kind: 'thinking', round: 1, seq: 0, text: 'already finished thinking' }],
    });
    const ids = projected.map(message => message.id);

    expect(ids).toEqual(['answer']);
    expect(ids).not.toContain(STREAMING_TAIL_ID);
    expect(projected[0]?.content).toEqual([
      { type: 'reasoning', text: 'already finished thinking' },
      expect.objectContaining({ type: 'tool-call', toolCallId: 'stale-tool' }),
      { type: 'text', text: 'hello' },
    ]);
  });

  it('replays a settled turn reasoning and tool calls from its request id', () => {
    const answer = msg({
      id: 'answer',
      sender: 'agent',
      content: 'finished',
      extraMetadata: { requestId: 'req-1' },
    });
    const timeline = [tool({ id: 'call-1', status: 'success', result: 'found it' })];
    const transcript = [
      { kind: 'thinking' as const, round: 1, seq: 0, text: 'need to search' },
      { kind: 'narration' as const, round: 1, seq: 1, text: 'I will check the sources.' },
      { kind: 'toolCall' as const, round: 1, seq: 2, callId: 'call-1' },
    ];

    expect(
      buildRuntimeMessages([answer], null, {
        turnTimelines: { 'req-1': timeline },
        turnTranscripts: { 'req-1': transcript },
      })[0]?.content
    ).toEqual([
      { type: 'reasoning', text: 'need to search' },
      { type: 'text', text: 'I will check the sources.' },
      expect.objectContaining({
        type: 'tool-call',
        toolCallId: 'call-1',
        toolName: 'web_search',
        result: 'found it',
      }),
      { type: 'text', text: 'finished' },
    ]);
  });

  it('chronologically anchors persisted trails to async agent messages without request ids', () => {
    const acknowledgement = msg({
      id: 'ack',
      sender: 'agent',
      content: 'Accepted background work',
      extraMetadata: {},
    });
    const content = buildRuntimeMessages([acknowledgement], null, {
      isRunning: false,
      turnTimelines: { 'request-async': [tool({ id: 'async-tool', status: 'success' })] },
      turnTranscripts: {
        'request-async': [
          { kind: 'thinking', round: 1, seq: 0, text: 'delegate this research' },
          { kind: 'toolCall', round: 1, seq: 1, callId: 'async-tool' },
        ],
      },
    })[0]?.content;

    expect(content).toEqual([
      { type: 'reasoning', text: 'delegate this research' },
      expect.objectContaining({ type: 'tool-call', toolCallId: 'async-tool' }),
      { type: 'text', text: 'Accepted background work' },
    ]);
  });

  it('renders final streamed narration only once', () => {
    const finalText = 'hey! what is up?';
    const answer = msg({ id: 'answer', sender: 'agent', content: finalText });
    const content = buildRuntimeMessages([answer], null, {
      turnTranscripts: { request: [{ kind: 'narration', round: 1, seq: 0, text: finalText }] },
      turnTimelines: { request: [] },
    })[0]?.content;

    expect(content).toEqual([{ type: 'text', text: finalText }]);
  });

  it('coalesces legacy assistant segments into one bubble with one tool trail', () => {
    const requestId = 'legacy-segmented-request';
    const intro = "Here's the crypto picture today:";
    const finalText = `${intro}\n\nBitcoin is trading around $77,000.`;
    const messages = [
      msg({ id: 'user', content: 'What is happening with Bitcoin?' }),
      msg({
        id: 'tool-envelope',
        sender: 'agent',
        content: JSON.stringify({
          content: null,
          tool_calls: [
            { id: 'call-search', name: 'web_search_tool', arguments: '{"query":"bitcoin"}' },
          ],
        }),
        extraMetadata: { requestId },
      }),
      msg({ id: 'intro', sender: 'agent', content: intro, extraMetadata: { requestId } }),
      msg({ id: 'final', sender: 'agent', content: finalText, extraMetadata: { requestId } }),
    ];

    const projected = buildRuntimeMessages(messages, null, {
      turnTimelines: {
        [requestId]: [
          tool({ id: 'call-search', name: 'tool', status: 'success', result: 'market results' }),
        ],
      },
      turnTranscripts: {
        [requestId]: [{ kind: 'toolCall', round: 1, seq: 0, callId: 'call-search' }],
      },
    });

    expect(projected).toHaveLength(2);
    expect(projected[1]).toMatchObject({ id: 'final', role: 'assistant' });
    expect(projected[1]?.content).toEqual([
      expect.objectContaining({
        type: 'tool-call',
        toolCallId: 'call-search',
        toolName: 'web_search_tool',
      }),
      { type: 'text', text: finalText },
    ]);
  });

  it('does not coalesce adjacent assistant turns with different request ids', () => {
    const projected = buildRuntimeMessages(
      [
        msg({ id: 'first', sender: 'agent', content: 'first', extraMetadata: { requestId: 'r1' } }),
        msg({
          id: 'second',
          sender: 'agent',
          content: 'second',
          extraMetadata: { requestId: 'r2' },
        }),
      ],
      null
    );

    expect(projected.map(message => message.id)).toEqual(['first', 'second']);
  });

  it('keeps a scoped standalone delivery out of the adjacent legacy runs', () => {
    // Legacy segments carry no request id; a background delivery persisted by
    // the core carries no request id either but is stamped with a `scope`.
    // Without the marker the three rows read as one segmented answer and the
    // delivery's text and metadata would be folded into its neighbours.
    const projected = buildRuntimeMessages(
      [
        msg({ id: 'seg-a', sender: 'agent', content: 'first paragraph' }),
        msg({
          id: 'delivery',
          sender: 'agent',
          content: 'Background result: inbox digest',
          extraMetadata: { scope: 'autonomous_task_result', success: true },
        }),
        msg({ id: 'seg-b', sender: 'agent', content: 'a later paragraph' }),
      ],
      null
    );

    expect(projected.map(message => message.id)).toEqual(['seg-a', 'delivery', 'seg-b']);
    expect(projected[1]?.content).toEqual([
      { type: 'text', text: 'Background result: inbox digest' },
    ]);
  });

  it('does not let a scoped delivery absorb the identified segment before it', () => {
    const projected = buildRuntimeMessages(
      [
        msg({
          id: 'answer',
          sender: 'agent',
          content: 'answer',
          extraMetadata: { requestId: 'r1' },
        }),
        msg({
          id: 'delivery',
          sender: 'agent',
          content: 'worker output',
          extraMetadata: { scope: 'worker_thread', requestId: 'r-worker' },
        }),
      ],
      null
    );

    expect(projected.map(message => message.id)).toEqual(['answer', 'delivery']);
  });

  it('does not hand a later turn trail to an earlier trail-less answer', () => {
    // Two unanchored answers, one unclaimed trail. Positional pairing would
    // give the trail to `earlier` (which produced nothing) and leave `later`
    // (which actually used the tool) bare — a wrong attribution, not a loss.
    const projected = buildRuntimeMessages(
      [
        msg({ id: 'ask-1', content: 'first question' }),
        msg({ id: 'earlier', sender: 'agent', content: 'plain answer', extraMetadata: {} }),
        msg({ id: 'ask-2', content: 'second question' }),
        msg({ id: 'later', sender: 'agent', content: 'tool answer', extraMetadata: {} }),
      ],
      null,
      {
        isRunning: false,
        turnTimelines: { 'request-later': [tool({ id: 'later-tool', status: 'success' })] },
        turnTranscripts: {
          'request-later': [{ kind: 'toolCall', round: 1, seq: 0, callId: 'later-tool' }],
        },
      }
    );

    const toolBearing = projected
      .filter(
        message =>
          Array.isArray(message.content) && message.content.some(part => part.type === 'tool-call')
      )
      .map(message => message.id);
    expect(toolBearing).not.toContain('earlier');
    expect(projected[1]?.content).toEqual([{ type: 'text', text: 'plain answer' }]);
  });

  /**
   * The crash this guards: assistant-ui keys tool parts as `toolCallId-${id}`
   * and throws "Duplicate key … in useResources" on a repeat, taking the whole
   * thread render down on load. A provider that emits tool calls without ids
   * writes `''` for every one, so a settled turn can hold two transcript
   * pointers naming the same row.
   */
  it('emits one tool part per row when a turn has two pointers to the same call id', () => {
    const answer = msg({
      id: 'answer',
      sender: 'agent',
      content: 'done',
      extraMetadata: { requestId: 'req-1' },
    });
    const timeline = [tool({ id: '', status: 'success', result: 'once' })];
    const transcript = [
      { kind: 'toolCall' as const, round: 1, seq: 0, callId: '' },
      { kind: 'toolCall' as const, round: 1, seq: 1, callId: '' },
    ];

    const content = buildRuntimeMessages([answer], null, {
      turnTimelines: { 'req-1': timeline },
      turnTranscripts: { 'req-1': transcript },
    })[0]?.content as unknown as { type: string; toolCallId?: string }[];

    const toolIds = content.filter(part => part.type === 'tool-call').map(part => part.toolCallId);
    expect(toolIds).toHaveLength(1);
  });

  it('never repeats a toolCallId across the transcript and timeline passes', () => {
    const answer = msg({
      id: 'answer',
      sender: 'agent',
      content: 'done',
      extraMetadata: { requestId: 'req-1' },
    });
    const timeline = [
      tool({ id: 'c1', status: 'success', result: 'a' }),
      tool({ id: 'c2', status: 'success', result: 'b' }),
    ];
    const transcript = [{ kind: 'toolCall' as const, round: 1, seq: 0, callId: 'c1' }];

    const content = buildRuntimeMessages([answer], null, {
      turnTimelines: { 'req-1': timeline },
      turnTranscripts: { 'req-1': transcript },
    })[0]?.content as unknown as { type: string; toolCallId?: string }[];

    const toolIds = content.filter(part => part.type === 'tool-call').map(part => part.toolCallId);
    expect(toolIds).toEqual(['c1', 'c2']);
    expect(new Set(toolIds).size).toBe(toolIds.length);
  });

  it('re-converts only the tail as tokens land, never the settled transcript', () => {
    // The projection-level statement of the property `ChatThreadView.renderPerf`
    // pins for the render tree: streaming must not sweep the transcript.
    const settled = Array.from({ length: 40 }, (_, i) =>
      msg({ id: `m-${i}`, sender: i % 2 ? 'agent' : 'user', content: `prose ${i}` })
    );
    const parse = vi.spyOn(JSON, 'parse');

    buildRuntimeMessages(settled, null); // warm the identity cache
    parse.mockClear();

    let text = '';
    for (let i = 0; i < 5; i += 1) {
      text += ` tok${i}`;
      buildRuntimeMessages(settled, { requestId: 'r', content: text, thinking: '' });
    }

    // Zero: settled messages are cached by identity and the tail is plain text.
    expect(parse).not.toHaveBeenCalled();
    parse.mockRestore();
  });
});

describe('recovered tool names', () => {
  it('does not consume a recovered name on an entry it does not rename', () => {
    // `recoveredNames` comes from tool-call envelopes, so it only ever holds
    // names for the *generic* rows. Advancing the cursor on every entry made a
    // named row eat the first recovered name: the first generic row then took
    // the second name and the last one kept the placeholder.
    const converted = toThreadMessageLike(
      msg({
        id: 'a',
        sender: 'agent',
        content: 'done',
        extraMetadata: { assistantUiToolNames: ['web_search', 'web_fetch'] },
      }),
      [
        tool({ id: 'c1', name: 'read_file', seq: 0, status: 'success' }),
        tool({ id: 'c2', name: 'tool', seq: 1, status: 'success' }),
        tool({ id: 'c3', name: 'tool', seq: 2, status: 'success' }),
      ]
    );
    const names = (converted.content as unknown as { type: string; toolName?: string }[])
      .filter(part => part.type === 'tool-call')
      .map(part => part.toolName);
    expect(names).toEqual(['read_file', 'web_search', 'web_fetch']);
  });
});

describe('terminal tool status', () => {
  it('carries a failed tool status through to the rendered part', () => {
    // assistant-ui's tool-call part has no status field, so a failed tool that
    // produced output used to arrive as a bare result and read as success.
    const converted = toThreadMessageLike(msg({ id: 'a', sender: 'agent', content: 'done' }), [
      tool({ id: 'c1', name: 'web_search', seq: 0, status: 'error', result: 'boom' }),
    ]);
    const part = (converted.content as unknown as { type: string; result?: unknown }[]).find(
      candidate => candidate.type === 'tool-call'
    );
    expect(part?.result).toMatchObject({ status: 'error', value: 'boom' });
  });

  it('leaves a successful tool result untouched', () => {
    const converted = toThreadMessageLike(msg({ id: 'a', sender: 'agent', content: 'done' }), [
      tool({ id: 'c1', name: 'web_search', seq: 0, status: 'success', result: 'the answer' }),
    ]);
    const part = (converted.content as unknown as { type: string; result?: unknown }[]).find(
      candidate => candidate.type === 'tool-call'
    );
    expect(part?.result).toBe('the answer');
  });
});

describe('narration merged into the final answer', () => {
  it('does not render narration that the final text already contains', () => {
    // `mergedAssistantText` prefers the longest text when it *contains* every
    // segment, so the duplicate is a substring rather than an exact match and
    // the old equality guard let it through twice.
    const finalText = 'I will check the sources. Here is what I found.';
    const converted = toThreadMessageLike(
      msg({ id: 'a', sender: 'agent', content: finalText }),
      [],
      [{ kind: 'narration', round: 1, seq: 0, text: 'I will check the sources.' }]
    );
    const texts = (converted.content as unknown as { type: string; text?: string }[])
      .filter(part => part.type === 'text')
      .map(part => part.text);
    expect(texts).toEqual([finalText]);
  });
});
