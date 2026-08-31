import { AssistantRuntimeProvider, useExternalStoreRuntime } from '@assistant-ui/react';
import debugFactory from 'debug';
import type { ReactNode } from 'react';

import { useAppSelector } from '../store/hooks';
import { useOpenHumanExternalStore } from './useOpenHumanExternalStore';

const debug = debugFactory('openhuman:assistant-ui');

/**
 * Mounts assistant-ui's runtime over one OpenHuman thread.
 *
 * Settled process history (reasoning, tools and sub-agents) is read directly
 * from the core transcript RPC, whose Rust projection cache is authoritative.
 * Redux only supplies the existing message list and live socket deltas.
 *
 * ## Why the thread is a prop
 *
 * It used to read `state.thread.selectedThreadId` itself. That is wrong for any
 * surface whose thread is NOT the selected one, and there is exactly such a
 * surface: the Workflow Copilot (`WorkflowCopilotPanel`) renders the shared
 * `ChatThreadView` against its own dedicated builder thread, which is never
 * equal to `selectedThreadId`. While nothing inside `ChatThreadView` read
 * assistant-ui context the mismatch was invisible; the moment the transcript
 * renders from `ThreadPrimitive`/`MessagePrimitive` it would paint the HOME
 * chat's messages inside the copilot. So the thread is chosen by whoever mounts
 * the runtime, and two instances with different thread ids can coexist —
 * assistant-ui's `AssistantRuntimeProvider` is ordinary React context, so the
 * nearest one wins for each subtree.
 *
 * ## The default
 *
 * `threadId` is optional and defaults to `selectedThreadId`, which is what the
 * app-wide mount in `ChatRuntimeProvider` wants: it sits above the home chat and
 * must follow the user's thread selection. `undefined` (prop omitted) means
 * "follow the selection"; an explicit `null` means "this surface has no thread
 * yet" (the copilot before its first send creates one) and is NOT a request to
 * fall back — falling back there is precisely the bug above.
 */
export function AssistantUiRuntimeProvider({
  threadId,
  children,
}: {
  /**
   * The thread this runtime instance represents. Omit to follow the globally
   * selected thread; pass `null` for a surface that owns a thread but has not
   * created it yet.
   */
  threadId?: string | null;
  children: ReactNode;
}) {
  const selectedThreadId = useAppSelector(state => state.thread.selectedThreadId);
  const effectiveThreadId = threadId === undefined ? selectedThreadId : threadId;
  debug(
    '[assistant-ui] runtime scope thread=%s source=%s',
    effectiveThreadId ?? '(none)',
    threadId === undefined ? 'selection' : 'explicit'
  );
  const adapter = useOpenHumanExternalStore(effectiveThreadId);
  const runtime = useExternalStoreRuntime(adapter);
  return <AssistantRuntimeProvider runtime={runtime}>{children}</AssistantRuntimeProvider>;
}

export default AssistantUiRuntimeProvider;
