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

/**
 * The agent's reasoning or visible narration, surfaced inline in the timeline
 * as quoted/italic prose at the position it streamed — so a thought shows up
 * wherever it occurred between tool calls. Shown directly (no "Thoughts"
 * heading, no collapse). Both `thinking` and `text` transcript items render
 * through here. Renders nothing for an all-whitespace delta so a half-streamed
 * item never flashes an empty quote.
 */
export function ThoughtBlock({ text }: { text: string }) {
  // Drop any inline `<tool_call>…</tool_call>` envelope the model emitted as
  // text — the call already shows as its own row. Keep the original newlines
  // (only trim the ends) so the markdown renderer can see headings, lists,
  // code fences and emphasis instead of flattening them to one plain line.
  const clean = stripToolCallEnvelopes(text).trim();
  if (!clean) return null;
  // Rendered through the shared `BubbleMarkdown` so a thought formats markdown
  // (bold, code, lists) — but scaled back to the original quiet thought look:
  // small (12px) and light/muted, not the larger, darker agent-bubble prose.
  // Descendant overrides on `.prose` beat the typography plugin's base sizing;
  // code keeps its accent colour so inline `tool_names` still read clearly.
  return (
    <div
      data-testid="subagent-thought"
      className="my-0.5 wrap-break-word [&_.prose]:text-[12px] [&_.prose]:leading-relaxed [&_.prose]:text-content-muted [&_.prose_strong]:text-content-muted [&_.prose_:is(h1,h2,h3,h4,h5,h6)]:text-[12px] [&_.prose_:is(h1,h2,h3,h4,h5,h6)]:text-content-muted">
      <BubbleMarkdown content={clean} />
    </div>
  );
}

/**
 * Render the live activity of one running (or completed) sub-agent inside its
 * parent timeline row — the mode/dedicated-thread badge, the child iteration
 * counter, the final-run statistics, and the ordered transcript of child tool
 * calls interleaved with the agent's "Thoughts" (reasoning + narration).
 *
 * Kept as a sibling of the existing worker-thread / detail block so the
 * surrounding disclosure chevron + status pill behaviour is unaffected — this
 * component only renders when `subagent` is present on the entry, which is true
 * for any row produced by the `subagent_*` socket events from a current core.
 */
export function SubagentActivityBlock({
  subagent,
  onView,
}: {
  subagent: SubagentActivity;
  /** Opens the full-transcript drawer for this subagent. Omitted in
   * read-only contexts (e.g. a completed snapshot with no live driver). */
  onView?: () => void;
}) {
  const { t } = useT();
  const headerBits: string[] = [];
  if (subagent.mode) headerBits.push(subagent.mode);
  if (subagent.dedicatedThread) headerBits.push(t('conversations.toolTimeline.workerThread'));
  if (subagent.childIteration != null) {
    if (subagent.childMaxIterations != null) {
      headerBits.push(
        `${t('conversations.toolTimeline.turn')} ${subagent.childIteration}/${subagent.childMaxIterations}`
      );
    } else {
      headerBits.push(`${t('conversations.toolTimeline.step')} ${subagent.childIteration}`);
    }
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

  // The ordered transcript drives the inline activity: child tool-call rows
  // and the agent's "Thoughts" (reasoning + visible narration) render in the
  // exact order they streamed, so each thought appears wherever it occurred
  // between tool calls. Falls back to the flat tool-call list when the prose
  // transcript is absent (e.g. a rehydrated/interrupted snapshot).
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
          {transcript.map((item, i) =>
            item.kind === 'tool' ? (
              <ChildToolCallCard key={item.callId} call={item} />
            ) : (
              <ThoughtBlock key={`thought-${i}`} text={item.text} />
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
            {subagent.changedFiles && subagent.changedFiles.length > 0 ? (
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
