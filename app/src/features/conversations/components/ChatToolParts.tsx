import { type ToolCallMessagePartComponent, useAui } from '@assistant-ui/react';
import { type FC, type PropsWithChildren, useCallback } from 'react';

import type { ThreadGroupPart } from '../../../components/assistant-ui/thread';
import {
  ToolGroupContent,
  ToolGroupRoot,
  ToolGroupTrigger,
} from '../../../components/assistant-ui/tool-group';
import IntegrationConnectCard from '../../../components/chat/IntegrationConnectCard';
import { useAuiThreadId } from '../../../providers/AssistantUiRuntimeProvider';
import type { SubagentActivity } from '../../../store/chatRuntimeSlice';
import { useAppSelector } from '../../../store/hooks';
import { AssistantUiSubagentCall, isActiveSubagentStatus } from './AssistantUiSubagentCall';
import { isApprovalPending, OpenHumanToolCall } from './AssistantUiToolCall';

function asSubagentActivity(value: unknown): SubagentActivity | undefined {
  if (!value || typeof value !== 'object') return undefined;
  const candidate = value as Partial<SubagentActivity>;
  if (
    typeof candidate.taskId !== 'string' ||
    typeof candidate.agentId !== 'string' ||
    !Array.isArray(candidate.toolCalls)
  ) {
    return undefined;
  }
  return candidate as SubagentActivity;
}

function readSubagentState(
  args: unknown,
  result: unknown
): { activity: SubagentActivity | undefined; running: boolean } {
  const completed = asSubagentActivity(result);
  // A settled part carries the activity, but "settled" is not "succeeded":
  // ask the activity's own status so a `failed` delegation is not rendered as
  // a completed one.
  if (completed) return { activity: completed, running: isActiveSubagentStatus(completed.status) };
  const progress =
    args && typeof args === 'object'
      ? asSubagentActivity((args as { progress?: unknown }).progress)
      : undefined;
  return { activity: progress, running: result === undefined };
}

/** Adapt an assistant-ui `task` part onto the shared delegation card. */
export const SubagentCall: ToolCallMessagePartComponent = ({ args, result }) => {
  const aui = useAui();
  const { activity, running } = readSubagentState(args, result);
  const description = (args as { description?: string } | undefined)?.description;
  const fallbackAgent = (args as { subagent_type?: string } | undefined)?.subagent_type;
  const resolved = activity ?? {
    taskId: 'pending-subagent',
    agentId: fallbackAgent ?? 'subagent',
    toolCalls: [],
  };
  // A delegation parked on `ask_user_clarification` is unblocked by an ordinary
  // user turn: the orchestrator is holding the `[SUBAGENT_AWAITING_USER]`
  // envelope and resumes the child with `continue_subagent` once the user
  // answers. Appending through the runtime routes to the external store's
  // `onNew` and out to the registered chat surface, i.e. the same entry point
  // as the composer's Send, so queueing behind an in-flight turn is decided in
  // one place rather than duplicated here.
  const answer = useCallback(
    (text: string) => {
      void aui.thread.append({ role: 'user', content: [{ type: 'text', text }] });
    },
    [aui]
  );
  return (
    <AssistantUiSubagentCall
      activity={resolved}
      running={running}
      description={description}
      onAnswer={answer}
    />
  );
};

/** The tool that parks on the ApprovalGate but needs OAuth, not approve/deny. */
const COMPOSIO_CONNECT_TOOL = 'composio_connect';

/**
 * A parked `composio_connect` call.
 *
 * It arrives over the same `approval_request` path as every other gated tool,
 * but "Approve" is the wrong affordance: approving without connecting resumes
 * the agent against a toolkit that still has no credentials. The existing
 * connect card runs the OAuth handoff, polls until the toolkit is live, and
 * only then resolves the gate with `approve_once` (or `deny` on cancel/timeout)
 * — so it is reused verbatim rather than reimplemented against
 * `respondToApproval`.
 *
 * Falls through to the ordinary card once the approval is resolved, or when the
 * request is not the one the store holds: `PendingApproval.toolkit` names the
 * integration to connect and lives in Redux, not on the part.
 */
const ComposioConnectCall: ToolCallMessagePartComponent = props => {
  const threadId = useAuiThreadId();
  const approval = useAppSelector(state =>
    threadId ? (state.chatRuntime.pendingApprovalByThread?.[threadId] ?? null) : null
  );
  const gated =
    isApprovalPending(props.approval) &&
    approval != null &&
    approval.requestId === props.approval?.id;
  if (!gated || !threadId || !approval) return <OpenHumanToolCall {...props} />;
  return (
    <div data-testid="assistant-ui-integration-connect">
      {/* Keyed by request id so a second parked connect remounts the card with
          fresh phase / field / poll state, matching the legacy placement. */}
      <IntegrationConnectCard key={approval.requestId} threadId={threadId} approval={approval} />
    </div>
  );
};

/** Route every call through an assistant-ui-native rich renderer. */
export const ChatToolFallback: ToolCallMessagePartComponent = props => {
  if (props.toolName === 'task') return <SubagentCall {...props} />;
  if (props.toolName === COMPOSIO_CONNECT_TOOL) return <ComposioConnectCall {...props} />;
  return <OpenHumanToolCall {...props} />;
};

/** Keep the assistant-ui tool cards visible; each card owns its detail collapse. */
export const ChatToolGroup: FC<PropsWithChildren<{ group: ThreadGroupPart }>> = ({
  group,
  children,
}) => {
  const running = group.status.type === 'running';
  return (
    <ToolGroupRoot variant="ghost" defaultOpen>
      <ToolGroupTrigger count={group.indices.length} active={running} />
      <ToolGroupContent>{children}</ToolGroupContent>
    </ToolGroupRoot>
  );
};
