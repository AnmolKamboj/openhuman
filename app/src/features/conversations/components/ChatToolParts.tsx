import type { ToolCallMessagePartComponent } from '@assistant-ui/react';
import { CheckIcon, ChevronDownIcon, Loader2Icon, WrenchIcon, WorkflowIcon } from 'lucide-react';
import type { FC, PropsWithChildren } from 'react';

import { cn } from '../../../components/assistant-ui/lib/utils';
import type { ThreadGroupPart } from '../../../components/assistant-ui/thread';
import {
  ToolGroupContent,
  ToolGroupRoot,
  ToolGroupTrigger,
} from '../../../components/assistant-ui/tool-group';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '../../../components/assistant-ui/ui/collapsible';
import type { SubagentActivity } from '../../../store/chatRuntimeSlice';
import { formatToolName } from '../../../utils/toolTimelineFormatting';
import { BubbleMarkdown } from './AgentMessageBubble';
import { SubagentActivityBlock } from './SubagentActivityBlock';

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
  if (completed) return { activity: completed, running: false };
  const progress =
    args && typeof args === 'object'
      ? asSubagentActivity((args as { progress?: unknown }).progress)
      : undefined;
  return { activity: progress, running: result === undefined };
}

/** Render a real OpenHuman `task` delegation using the existing activity view. */
export const SubagentCall: ToolCallMessagePartComponent = ({ args, result }) => {
  const { activity, running } = readSubagentState(args, result);
  const description = (args as { description?: string } | undefined)?.description;
  const name =
    activity?.displayName ??
    activity?.agentId ??
    (args as { subagent_type?: string } | undefined)?.subagent_type ??
    'subagent';

  return (
    <Collapsible
      data-slot="aui_subagent-call"
      data-testid="assistant-ui-subagent-call"
      defaultOpen
      className={cn(
        'aui-subagent-call border-border/60 dark:border-muted-foreground/15 rounded-xl border',
        running && 'border-dashed'
      )}>
      <CollapsibleTrigger className="group/subagent text-muted-foreground hover:text-foreground flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors">
        <WorkflowIcon className="size-4 shrink-0" />
        <span className="text-start leading-none">
          Delegated to <b className="text-foreground">{name}</b>
        </span>
        {running ? (
          <span className="bg-muted text-muted-foreground flex shrink-0 items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px] leading-none">
            <Loader2Icon className="size-3 animate-spin [animation-duration:0.6s]" />
            running
          </span>
        ) : (
          <span className="text-muted-foreground flex shrink-0 items-center gap-1.5 text-[11px] leading-none">
            <CheckIcon className="size-3.5" />
            {activity?.elapsedMs != null && (
              <span className="tabular-nums">{(activity.elapsedMs / 1000).toFixed(1)}s</span>
            )}
          </span>
        )}
        <ChevronDownIcon className="ml-auto size-4 shrink-0 -rotate-90 transition-transform group-data-[state=open]/subagent:rotate-0" />
      </CollapsibleTrigger>
      <CollapsibleContent className="px-3 pb-3">
        {description && <p className="text-muted-foreground text-xs">{description}</p>}
        {activity && <SubagentActivityBlock subagent={activity} />}
      </CollapsibleContent>
    </Collapsible>
  );
};

function friendlyLabel(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/^./, char => char.toUpperCase());
}

function toolDisplayName(toolName: string, running: boolean): string {
  if (toolName === 'web_fetch') return running ? 'Fetching from the web' : 'Fetched from the web';
  if (toolName === 'web_search_tool' || toolName === 'web_search') {
    return running ? 'Searching the web' : 'Searched the web';
  }
  return formatToolName(toolName);
}

function parsedValue(value: unknown): unknown {
  if (typeof value !== 'string') return value;
  const trimmed = value.trim();
  if (!(trimmed.startsWith('{') || trimmed.startsWith('['))) return value;
  try {
    return JSON.parse(trimmed);
  } catch {
    return value;
  }
}

function hasDisplayValue(value: unknown): boolean {
  if (value === undefined || value === null || value === '') return false;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === 'object') return Object.keys(value as object).length > 0;
  return true;
}

function ToolDataView({ value }: { value: unknown }) {
  const parsed = parsedValue(value);
  if (Array.isArray(parsed)) {
    return (
      <ul className="space-y-1 text-xs">
        {parsed.map((item, index) => (
          <li key={index} className="bg-muted/50 rounded-md px-2 py-1.5">
            <ToolDataView value={item} />
          </li>
        ))}
      </ul>
    );
  }
  if (parsed && typeof parsed === 'object') {
    const entries = Object.entries(parsed);
    // Tool wrappers frequently add bookkeeping beside the actual payload
    // (`tool_call_id`, success, timing). When a semantic output field exists,
    // show that value directly and hide the wrapper entirely.
    for (const key of ['content', 'output', 'result', 'message']) {
      const semantic = entries.find(([candidate]) => candidate === key)?.[1];
      if (hasDisplayValue(semantic)) return <ToolDataView value={semantic} />;
    }
    return (
      <dl className="divide-border bg-muted/40 divide-y rounded-md px-2 text-xs">
        {entries.map(([key, item]) => (
          <div key={key} className="grid grid-cols-[minmax(7rem,auto)_1fr] gap-3 py-1.5">
            <dt className="text-muted-foreground font-medium">{friendlyLabel(key)}</dt>
            <dd className="min-w-0 wrap-break-word">
              <ToolDataView value={item} />
            </dd>
          </div>
        ))}
      </dl>
    );
  }
  if (typeof parsed === 'boolean') return <span>{parsed ? 'Yes' : 'No'}</span>;
  if (typeof parsed === 'string') return <BubbleMarkdown content={parsed} />;
  return <span className="whitespace-pre-wrap">{String(parsed ?? '')}</span>;
}

/** Rich assistant-ui-native renderer for an ordinary OpenHuman tool call. */
export const OpenHumanToolCall: ToolCallMessagePartComponent = ({
  toolName,
  args,
  argsText,
  result,
}) => {
  const running = result === undefined;
  const input = hasDisplayValue(args) ? args : parsedValue(argsText ?? '');
  const output = parsedValue(result);
  return (
    <Collapsible
      data-slot="aui_openhuman-tool-call"
      data-testid="assistant-ui-tool-call"
      defaultOpen={running}
      className={cn(
        'border-border/60 dark:border-muted-foreground/15 rounded-xl border',
        running && 'border-dashed'
      )}>
      <CollapsibleTrigger className="group/tool text-muted-foreground hover:text-foreground flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors">
        <WrenchIcon className="size-4 shrink-0" />
        <span className="text-foreground text-start font-medium">
          {toolDisplayName(toolName, running)}
        </span>
        {running ? (
          <span className="bg-muted flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px]">
            <Loader2Icon className="size-3 animate-spin [animation-duration:0.6s]" />
            running
          </span>
        ) : (
          <span className="flex items-center gap-1 text-[11px]">
            <CheckIcon className="size-3.5" /> done
          </span>
        )}
        <ChevronDownIcon className="ml-auto size-4 shrink-0 -rotate-90 transition-transform group-data-[state=open]/tool:rotate-0" />
      </CollapsibleTrigger>
      <CollapsibleContent className="space-y-2 px-3 pb-3">
        {hasDisplayValue(input) ? (
          <div>
            <p className="text-muted-foreground mb-1 text-[11px] font-medium uppercase">Input</p>
            <div className="max-h-48 overflow-auto">
              <ToolDataView value={input} />
            </div>
          </div>
        ) : null}
        {hasDisplayValue(output) ? (
          <div>
            <p className="text-muted-foreground mb-1 text-[11px] font-medium uppercase">Output</p>
            <div className="max-h-64 overflow-auto">
              <ToolDataView value={output} />
            </div>
          </div>
        ) : null}
      </CollapsibleContent>
    </Collapsible>
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
