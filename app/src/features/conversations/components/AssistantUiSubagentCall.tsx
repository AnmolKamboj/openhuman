import { CheckIcon, ChevronDownIcon, CircleXIcon, Loader2Icon, WorkflowIcon } from 'lucide-react';

import { cn } from '../../../components/assistant-ui/lib/utils';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '../../../components/assistant-ui/ui/collapsible';
import Badge from '../../../components/ui/Badge';
import WorktreeActions from '../../../components/worktree/WorktreeActions';
import { useT } from '../../../lib/i18n/I18nContext';
import type {
  SubagentActivity,
  SubagentToolCallEntry,
  SubagentTranscriptItem,
} from '../../../store/chatRuntimeSlice';
import { basename } from '../../../utils/pathUtils';
import { stripToolCallEnvelopes } from '../../../utils/toolTimelineFormatting';
import { BubbleMarkdown } from './AgentMessageBubble';
import { AssistantUiToolCallCard } from './AssistantUiToolCall';

type ChildToolCall = SubagentToolCallEntry | Extract<SubagentTranscriptItem, { kind: 'tool' }>;

function ChildToolCallCard({ call }: { call: ChildToolCall }) {
  return (
    <AssistantUiToolCallCard
      toolName={call.toolName}
      args={call.args}
      result={call.result}
      status={call.status}
      displayName={call.displayName}
      detail={call.detail}
      elapsedMs={call.elapsedMs}
      failure={call.failure}
    />
  );
}

function Thought({ text }: { text: string }) {
  const clean = stripToolCallEnvelopes(text).trim();
  if (!clean) return null;
  return (
    <div
      data-testid="subagent-thought"
      className="my-0.5 wrap-break-word [&_.prose]:text-[12px] [&_.prose]:leading-relaxed [&_.prose]:text-content-muted [&_.prose_strong]:text-content-muted [&_.prose_:is(h1,h2,h3,h4,h5,h6)]:text-[12px] [&_.prose_:is(h1,h2,h3,h4,h5,h6)]:text-content-muted">
      <BubbleMarkdown content={clean} />
    </div>
  );
}

function SubagentDetails({
  subagent,
  onView,
}: {
  subagent: SubagentActivity;
  onView?: () => void;
}) {
  const { t } = useT();
  const headerBits: string[] = [];
  if (subagent.mode) headerBits.push(subagent.mode);
  if (subagent.dedicatedThread) headerBits.push(t('conversations.toolTimeline.workerThread'));
  if (subagent.childIteration != null) {
    headerBits.push(
      subagent.childMaxIterations != null
        ? `${t('conversations.toolTimeline.turn')} ${subagent.childIteration}/${subagent.childMaxIterations}`
        : `${t('conversations.toolTimeline.step')} ${subagent.childIteration}`
    );
  } else if (subagent.iterations != null) {
    headerBits.push(
      subagent.iterations === 1
        ? `${subagent.iterations} ${t('chat.turn')}`
        : `${subagent.iterations} ${t('chat.turns')}`
    );
  }
  if (subagent.elapsedMs != null) {
    headerBits.push(
      subagent.elapsedMs >= 1000
        ? `${(subagent.elapsedMs / 1000).toFixed(1)}s`
        : `${subagent.elapsedMs}ms`
    );
  }
  const transcript = subagent.transcript ?? [];

  return (
    <div
      className="mt-1 space-y-0.5 text-[12px] text-content-muted"
      data-testid="subagent-activity">
      {headerBits.length > 0 ? (
        <div className="flex flex-wrap items-center gap-1.5">
          {headerBits.map(bit => (
            <Badge key={bit} className="rounded-full">
              {bit}
            </Badge>
          ))}
        </div>
      ) : null}
      {transcript.length > 0 ? (
        <div className="ml-1 space-y-0.5" data-testid="subagent-transcript">
          {transcript.map((item, index) =>
            item.kind === 'tool' ? (
              <ChildToolCallCard key={item.callId} call={item} />
            ) : (
              <Thought key={`thought-${index}`} text={item.text} />
            )
          )}
        </div>
      ) : subagent.toolCalls.length > 0 ? (
        <div className="ml-1 space-y-0.5">
          {subagent.toolCalls.map(call => (
            <ChildToolCallCard key={call.callId} call={call} />
          ))}
        </div>
      ) : null}
      {subagent.worktreePath ? (
        <div
          className="mt-1 space-y-1 rounded-md border border-line bg-surface-muted/70 p-1.5"
          data-testid="subagent-worktree">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="font-medium text-content-secondary">{t('worktree.label')}</span>
            <span
              className="truncate font-mono text-[12px] text-content-muted"
              title={subagent.worktreePath}>
              {basename(subagent.worktreePath)}
            </span>
            <Badge variant={subagent.isDirty ? 'warning' : 'success'} className="rounded-full">
              {subagent.isDirty ? t('worktree.dirty') : t('worktree.clean')}
            </Badge>
            {subagent.changedFiles?.length ? (
              <span className="text-[11px] text-content-faint">
                {subagent.changedFiles.length}{' '}
                {subagent.changedFiles.length === 1
                  ? t('worktree.changedFile')
                  : t('worktree.changedFiles')}
              </span>
            ) : null}
          </div>
          <WorktreeActions path={subagent.worktreePath} isDirty={subagent.isDirty} compact />
        </div>
      ) : null}
      {onView ? (
        <button
          type="button"
          onClick={onView}
          data-testid="subagent-view-processing"
          className="mt-0.5 rounded-full px-1.5 py-0.5 text-[12px] font-medium text-primary-600 hover:bg-primary-50 dark:text-primary-300 dark:hover:bg-primary-500/15">
          {t('conversations.subagent.viewProcessing')} →
        </button>
      ) : null}
    </div>
  );
}

/**
 * Statuses that mean the delegation is still in flight.
 *
 * `SubagentActivity.status` carries `running` | `awaiting_user` | `completed` |
 * `failed`, and collapsing that to a boolean is what produced two opposite
 * rendering bugs: a caller that omitted `running` showed a *failed* delegation
 * with a success check, while `status !== 'completed'` gave the same row an
 * endless spinner. Both call sites now ask this one question.
 */
export function isActiveSubagentStatus(status: string | undefined): boolean {
  return status === 'running' || status === 'awaiting_user';
}

/** Statuses that mean the delegation stopped without succeeding. */
function isFailedSubagentStatus(status: string | undefined): boolean {
  return status === 'failed' || status === 'cancelled';
}

export function AssistantUiSubagentCall({
  activity,
  running,
  description,
  onView,
  defaultOpen = false,
}: {
  activity: SubagentActivity;
  running?: boolean;
  description?: string;
  onView?: () => void;
  defaultOpen?: boolean;
}) {
  const name = activity.displayName ?? activity.agentId ?? 'subagent';
  // Default to the activity's own lifecycle rather than `false`: most call
  // sites pass no `running` prop at all, and treating every non-running
  // activity as finished-successfully is what rendered a failed delegation
  // with a success check.
  const active = running ?? isActiveSubagentStatus(activity.status);
  const failed = !active && isFailedSubagentStatus(activity.status);
  return (
    <Collapsible
      defaultOpen={defaultOpen}
      data-slot="aui_subagent-call"
      data-testid="assistant-ui-subagent-call"
      data-status={activity.status ?? (active ? 'running' : 'completed')}
      className={cn(
        'aui-subagent-call border-border/60 dark:border-muted-foreground/15 rounded-xl border',
        active && 'border-dashed'
      )}>
      <CollapsibleTrigger className="group/subagent text-muted-foreground hover:text-foreground flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors">
        <WorkflowIcon className="size-4 shrink-0" />
        <span className="text-start leading-none">
          Delegated to <b className="text-foreground">{name}</b>
        </span>
        {active ? (
          <span className="bg-muted text-muted-foreground flex shrink-0 items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px] leading-none">
            <Loader2Icon className="size-3 animate-spin [animation-duration:0.6s]" /> running
          </span>
        ) : (
          <span className="text-muted-foreground flex shrink-0 items-center gap-1.5 text-[11px] leading-none">
            {failed ? <CircleXIcon className="size-3.5" /> : <CheckIcon className="size-3.5" />}
            {failed ? <span>{activity.status}</span> : null}
            {activity.elapsedMs != null ? (
              <span className="tabular-nums">{(activity.elapsedMs / 1000).toFixed(1)}s</span>
            ) : null}
          </span>
        )}
        <ChevronDownIcon className="ml-auto size-4 shrink-0 -rotate-90 transition-transform group-data-[state=open]/subagent:rotate-0" />
      </CollapsibleTrigger>
      <CollapsibleContent className="px-3 pb-3">
        {description ? <p className="text-muted-foreground text-xs">{description}</p> : null}
        <SubagentDetails subagent={activity} onView={onView} />
      </CollapsibleContent>
    </Collapsible>
  );
}
