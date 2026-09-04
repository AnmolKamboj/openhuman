/**
 * The render half of the parked-approval repair.
 *
 * `AssistantUiChat` overrides assistant-ui's `ToolFallback` with
 * `ChatToolFallback`, and the component behind it destructured four fields and
 * dropped `status`, `approval` and `respondToApproval` — so even once the
 * runtime carried a decision, nothing on screen offered it. The kit's own
 * approval-capable fallback renders on no user-facing surface (only the dev
 * demo), which is why a grep for approval support in `components/assistant-ui`
 * looked healthy while the chat had none.
 */
import { combineReducers, configureStore } from '@reduxjs/toolkit';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider } from 'react-redux';
import { describe, expect, it, vi } from 'vitest';

import { AssistantUiRuntimeProvider } from '../../../providers/AssistantUiRuntimeProvider';
import chatRuntimeReducer, {
  type PendingApproval,
  setPendingApprovalForThread,
} from '../../../store/chatRuntimeSlice';
import threadReducer from '../../../store/threadSlice';
import { ChatToolFallback } from './ChatToolParts';

vi.mock('../../../services/api/threadApi', () => ({
  threadApi: {
    getDerivedTranscript: vi
      .fn()
      .mockResolvedValue({
        threadId: 't-1',
        items: [],
        total: 0,
        hasMore: false,
        hasTranscript: false,
      }),
  },
}));

const THREAD_ID = 't-1';
const REQUEST_ID = 'appr-1';

/** The option set the projection puts on a parked call. */
const OPTIONS = [
  { id: 'approve_once', kind: 'allow-once' as const },
  { id: 'approve_always_for_tool', kind: 'allow-always' as const },
  { id: 'deny', kind: 'reject-once' as const },
];

function gatedPart(over: Record<string, unknown> = {}) {
  return {
    type: 'tool-call' as const,
    toolName: 'shell',
    toolCallId: 'call-1',
    args: { command: 'ls -la' } as never,
    argsText: '{"command":"ls -la"}',
    result: undefined,
    status: { type: 'requires-action' as const, reason: 'interrupt' as const },
    approval: { id: REQUEST_ID, options: OPTIONS },
    addResult: () => {},
    resume: () => {},
    respondToApproval: () => {},
    ...over,
  };
}

function buildStore(approval?: PendingApproval) {
  const store = configureStore({
    reducer: combineReducers({ thread: threadReducer, chatRuntime: chatRuntimeReducer }),
    preloadedState: {
      thread: {
        threads: [],
        selectedThreadId: THREAD_ID,
        activeThreadIds: {},
        welcomeThreadId: null,
        messagesByThreadId: {},
        messages: [],
        isLoadingThreads: false,
        isLoadingMessages: false,
        messagesError: null,
      },
    } as never,
  });
  if (approval) store.dispatch(setPendingApprovalForThread({ threadId: THREAD_ID, approval }));
  return store;
}

/** Mounts the part under a runtime so `useAuiThreadId` resolves, as in the app. */
function renderInThread(node: React.ReactNode, approval?: PendingApproval) {
  return render(
    <Provider store={buildStore(approval)}>
      <AssistantUiRuntimeProvider>{node}</AssistantUiRuntimeProvider>
    </Provider>
  );
}

describe('ChatToolFallback — parked approval', () => {
  it('offers the decision on the gated call', () => {
    render(<ChatToolFallback {...gatedPart()} />);

    const bar = screen.getByTestId('assistant-ui-tool-approval');
    expect(bar).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Approve' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Always allow' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Deny' })).toBeInTheDocument();
  });

  it('says the call is awaiting input, not merely running', () => {
    // The `awaiting input` label existed but was unreachable: its only trigger
    // was a `status` the adapter forwards for `error` / `cancelled` alone.
    render(<ChatToolFallback {...gatedPart()} />);

    expect(screen.getByText('awaiting input')).toBeInTheDocument();
    expect(screen.queryByText('running')).not.toBeInTheDocument();
  });

  it('sends the chosen option id straight to the runtime', async () => {
    const respondToApproval = vi.fn();
    render(<ChatToolFallback {...gatedPart({ respondToApproval })} />);

    await userEvent.click(screen.getByRole('button', { name: 'Always allow' }));

    // The id is the core's own `approval_decide` decision literal, so the
    // adapter forwards it without a translation table.
    expect(respondToApproval).toHaveBeenCalledWith({ optionId: 'approve_always_for_tool' });
  });

  it('does not offer a second decision while the first is in flight', async () => {
    const respondToApproval = vi.fn();
    render(<ChatToolFallback {...gatedPart({ respondToApproval })} />);

    const approve = screen.getByRole('button', { name: 'Approve' });
    await userEvent.click(approve);
    await userEvent.click(approve);

    // A second decision throws "Tool call has no pending approval" inside the
    // runtime, in the window before the socket clears the gate.
    expect(respondToApproval).toHaveBeenCalledTimes(1);
  });

  it('re-offers the decision when the decide never lands', async () => {
    // `respondToApproval` returns void and the runtime swallows a rejection, so
    // a failed decide is indistinguishable from a slow one here. Coming back is
    // what keeps a still-parked turn answerable.
    vi.useFakeTimers();
    try {
      const respondToApproval = vi.fn();
      render(<ChatToolFallback {...gatedPart({ respondToApproval })} />);
      const approve = screen.getByRole('button', { name: 'Approve' });
      act(() => approve.click());
      expect(screen.getByRole('button', { name: 'Working…' })).toBeDisabled();

      act(() => vi.advanceTimersByTime(10_000));

      expect(screen.getByRole('button', { name: 'Approve' })).toBeEnabled();
    } finally {
      vi.useRealTimers();
    }
  });

  it('leaves an ordinary running tool alone', () => {
    // Every unsettled tool part in a `requires-action` message inherits that
    // status, so the prompt must key off the approval, not off the status.
    render(
      <ChatToolFallback
        {...gatedPart({ toolName: 'web_search', approval: undefined, toolCallId: 'call-2' })}
      />
    );

    expect(screen.queryByTestId('assistant-ui-tool-approval')).not.toBeInTheDocument();
    expect(screen.queryByText('awaiting input')).not.toBeInTheDocument();
  });

  it('gives composio_connect the OAuth affordance rather than approve/deny', () => {
    // `composio_connect` parks on the same gate, but approving it without
    // connecting resumes the agent against a toolkit with no credentials.
    renderInThread(
      <ChatToolFallback
        {...gatedPart({ toolName: 'composio_connect', args: { toolkit: 'googledrive' } })}
      />,
      {
        requestId: REQUEST_ID,
        toolName: 'composio_connect',
        message: 'Connect Google Drive?',
        toolkit: 'googledrive',
      }
    );

    expect(screen.getByTestId('assistant-ui-integration-connect')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /connect/i })).toBeInTheDocument();
    expect(screen.queryByTestId('assistant-ui-tool-approval')).not.toBeInTheDocument();
  });

  it('falls back to the ordinary card when the connect request is not the parked one', () => {
    // A stale part from an earlier turn must not adopt the live request.
    renderInThread(
      <ChatToolFallback
        {...gatedPart({
          toolName: 'composio_connect',
          approval: { id: 'appr-stale', options: OPTIONS },
        })}
      />,
      {
        requestId: REQUEST_ID,
        toolName: 'composio_connect',
        message: 'Connect Google Drive?',
        toolkit: 'googledrive',
      }
    );

    expect(screen.queryByTestId('assistant-ui-integration-connect')).not.toBeInTheDocument();
    expect(screen.getByTestId('assistant-ui-tool-approval')).toBeInTheDocument();
  });
});
