import Badge, { type BadgeVariant } from '../../../components/ui/Badge';
import { useT } from '../../../lib/i18n/I18nContext';
import type {
  SubagentTranscriptItem,
  ToolTimelineEntryStatus,
} from '../../../store/chatRuntimeSlice';
import { AssistantUiToolCallCard } from './AssistantUiToolCall';

/** Human-readable elapsed time for a sub-agent run or one of its tool calls. */
export function formatElapsed(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
}

/**
 * Map a sub-agent lifecycle status to the shared {@link Badge} variant used
 * for it everywhere in the conversation panel, so the drawer header pill, the
 * transcript tool rows and the inline timeline all read as one system. Kept in
 * one place because a status that renders `warning` in one surface and
 * `neutral` in another is a bug nobody notices until a screenshot diff.
 */
export function subagentStatusVariant(status: ToolTimelineEntryStatus | undefined): BadgeVariant {
  switch (status) {
    case 'success':
      return 'success';
    case 'error':
      return 'danger';
    case 'cancelled':
      return 'neutral';
    default:
      // running / awaiting_user — in flight, needs attention.
      return 'warning';
  }
}

/** Localised label for a sub-agent lifecycle status. */
export function useSubagentStatusLabel(status: ToolTimelineEntryStatus | undefined): string {
  const { t } = useT();
  switch (status) {
    case 'success':
      return t('conversations.subagent.statusCompleted');
    case 'error':
      return t('conversations.subagent.statusFailed');
    case 'cancelled':
      return t('conversations.subagent.statusCancelled');
    case 'awaiting_user':
      return t('conversations.subagent.statusAwaitingUser');
    default:
      return t('conversations.subagent.statusRunning');
  }
}

type SubagentToolItem = Extract<SubagentTranscriptItem, { kind: 'tool' }>;

/**
 * Pretty-print a tool's input arguments for display. Objects/arrays are
 * rendered as indented JSON; a string is shown verbatim. Returns `null` when
 * there are no arguments to show (e.g. a tool called with no input, or a
 * transcript reopened from memory where args weren't persisted).
 */
export function formatArgs(args: unknown): string | null {
  if (args == null) return null;
  if (typeof args === 'string') return args.length > 0 ? args : null;
  try {
    return JSON.stringify(args, null, 2);
  } catch {
    return String(args);
  }
}

/**
 * Drawer child calls use the exact assistant-ui card used by parent and inline
 * tools. The drawer no longer owns a parallel tool-call component hierarchy.
 */
export function SubagentToolCallRow({ item }: { item: SubagentToolItem }) {
  return (
    <AssistantUiToolCallCard
      toolName={item.toolName}
      args={item.args}
      result={item.result}
      status={item.status}
      displayName={item.displayName}
      detail={item.detail}
      elapsedMs={item.elapsedMs}
      failure={item.failure}
    />
  );
}
