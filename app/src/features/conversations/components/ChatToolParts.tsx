import type { ToolCallMessagePartComponent } from '@assistant-ui/react';
import type { FC, PropsWithChildren } from 'react';

import type { ThreadGroupPart } from '../../../components/assistant-ui/thread';
import {
  ToolGroupContent,
  ToolGroupRoot,
  ToolGroupTrigger,
} from '../../../components/assistant-ui/tool-group';
import type { SubagentActivity } from '../../../store/chatRuntimeSlice';
import { AssistantUiSubagentCall, isActiveSubagentStatus } from './AssistantUiSubagentCall';
import { OpenHumanToolCall } from './AssistantUiToolCall';

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
  const { activity, running } = readSubagentState(args, result);
  const description = (args as { description?: string } | undefined)?.description;
  const fallbackAgent = (args as { subagent_type?: string } | undefined)?.subagent_type;
  const resolved = activity ?? {
    taskId: 'pending-subagent',
    agentId: fallbackAgent ?? 'subagent',
    toolCalls: [],
  };
  return (
    <AssistantUiSubagentCall activity={resolved} running={running} description={description} />
  );
};

/** Route every call through an assistant-ui-native rich renderer. */
export const ChatToolFallback: ToolCallMessagePartComponent = props =>
  props.toolName === 'task' ? <SubagentCall {...props} /> : <OpenHumanToolCall {...props} />;

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
