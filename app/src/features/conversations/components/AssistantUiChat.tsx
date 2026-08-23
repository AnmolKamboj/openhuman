import { Thread, type ThreadComponents } from '@/components/assistant-ui/thread';
import { type AssistantState, useAui, useAuiState } from '@assistant-ui/react';
import { PlusIcon } from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';

import AttachmentPreview from '../../../components/chat/AttachmentPreview';
import { Button } from '../../../components/ui';
import type { Attachment } from '../../../lib/attachments';
import { useRegisterAction } from '../../../lib/commands/useRegisterAction';
import { useSlashCommands } from '../../../lib/commands/useSlashCommands';
import { useT } from '../../../lib/i18n/I18nContext';
import { AssistantUiRuntimeProvider } from '../../../providers/AssistantUiRuntimeProvider';
import { emptySessionTokenUsage } from '../../../store/chatRuntimeSlice';
import { useAppSelector } from '../../../store/hooks';
import { ChatToolFallback, ChatToolGroup } from './ChatToolParts';
import { contextUsageFromTokenUsage, ContextWindowPill } from './composer/ContextWindowPill';
import {
  type ThreadGoalController,
  ThreadGoalEditorPanel,
  ThreadGoalFooterTrigger,
} from './ThreadGoalChip';

const EMPTY_TOKEN_USAGE = emptySessionTokenUsage();
const selectComposerText = (state: AssistantState) => state.composer.text;

function ComposerTextBridge({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const aui = useAui();
  const composerText = useAuiState(selectComposerText);
  const previousHostValue = useRef(value);

  useEffect(() => {
    // A host-side write (dictation, ESC restore, clear) wins for this pass.
    if (previousHostValue.current !== value) {
      previousHostValue.current = value;
      if (composerText !== value) aui.composer.setText(value);
      return;
    }
    // Otherwise the editor changed and the host draft follows it.
    if (composerText !== value) onChange(composerText);
  }, [aui, composerText, onChange, value]);

  return null;
}

/**
 * The assistant-ui `Thread`, projected from OpenHuman's Redux transcript.
 *
 * The runtime is a read-only projection; Redux and the core remain authoritative
 * for messages, streaming and persistence. Composer sends are forwarded through
 * the chat-surface registration owned by `Conversations`, so this uses the same
 * send/cancel path as the legacy composer.
 */
export function AssistantUiChat({
  threadGoal,
  model,
  modelContextWindow,
  onModelChange,
  composerHeader,
  inputValue,
  onInputValueChange,
  onEscape,
  attachments,
  onAttachFiles,
  onRemoveAttachment,
  maxAttachments,
  attachmentsEnabled,
  attachmentInteractionBlocked,
  onAttachmentOnlySend,
}: {
  threadGoal: ThreadGoalController;
  model: string | null;
  modelContextWindow?: number | null;
  onModelChange: (value: string, contextWindow?: number | null) => void;
  composerHeader?: ReactNode;
  inputValue: string;
  onInputValueChange: (value: string) => void;
  onEscape?: () => void;
  attachments: Attachment[];
  onAttachFiles: (files: FileList | File[] | null) => Promise<void>;
  onRemoveAttachment: (id: string) => void;
  maxAttachments: number;
  attachmentsEnabled: boolean;
  attachmentInteractionBlocked: boolean;
  onAttachmentOnlySend: () => void;
}) {
  const { t } = useT();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const selectedThreadId = useAppSelector(state => state.thread.selectedThreadId);
  const loadError = useAppSelector(state => state.thread.messagesError);
  const tokenUsage = useAppSelector(state =>
    selectedThreadId
      ? (state.chatRuntime.usageByThread[selectedThreadId] ?? EMPTY_TOKEN_USAGE)
      : EMPTY_TOKEN_USAGE
  );
  const contextUsage = useMemo(
    () => contextUsageFromTokenUsage(tokenUsage, modelContextWindow),
    [modelContextWindow, tokenUsage]
  );
  const openThreadGoal = threadGoal.open;

  useRegisterAction({
    id: 'chat.goal',
    label: 'Set thread goal',
    labelKey: 'conversations.composer.command.goal',
    group: 'Chat',
    handler: openThreadGoal,
    enabled: () => selectedThreadId !== null,
    keywords: ['goal', 'objective', 'thread goal'],
    slashCommand: { id: 'goal', descriptionKey: 'conversations.composer.command.goal' },
  });
  const slashCommands = useSlashCommands();

  const ComposerExtras = useCallback(
    () => (
      <>
        <ContextWindowPill usage={contextUsage} />
        <div className="absolute right-0 bottom-full left-0 pb-2">
          <ThreadGoalEditorPanel ctl={threadGoal} />
        </div>
        <ThreadGoalFooterTrigger ctl={threadGoal} />
      </>
    ),
    [contextUsage, threadGoal]
  );
  const ComposerHeader = useCallback(() => <>{composerHeader}</>, [composerHeader]);
  const ComposerAttachments = useCallback(
    () => (
      <AttachmentPreview
        attachments={attachments}
        onRemove={onRemoveAttachment}
        disabled={attachmentInteractionBlocked}
      />
    ),
    [attachmentInteractionBlocked, attachments, onRemoveAttachment]
  );
  const ComposerAddAttachment = useCallback(
    () => (
      <>
        <input
          ref={fileInputRef}
          type="file"
          multiple
          className="hidden"
          onChange={event => {
            void onAttachFiles(event.target.files);
            event.target.value = '';
          }}
        />
        <Button
          type="button"
          iconOnly
          variant="tertiary"
          size="xs"
          aria-label={t('composer.attachFile')}
          title={t('composer.attachFile')}
          disabled={attachmentInteractionBlocked || attachments.length >= maxAttachments}
          onClick={() => fileInputRef.current?.click()}>
          <PlusIcon className="h-4 w-4" />
        </Button>
      </>
    ),
    [attachmentInteractionBlocked, attachments.length, maxAttachments, onAttachFiles, t]
  );

  const components: ThreadComponents = useMemo(
    () => ({
      ToolFallback: ChatToolFallback,
      ToolGroup: ChatToolGroup,
      ComposerExtras,
      ComposerHeader,
      ...(attachmentsEnabled
        ? {
            ComposerAttachments,
            ComposerAddAttachment,
            hasComposerAttachments: attachments.length > 0,
            onComposerAttachmentSend: onAttachmentOnlySend,
          }
        : {}),
    }),
    [
      ComposerAddAttachment,
      ComposerAttachments,
      ComposerExtras,
      ComposerHeader,
      attachments.length,
      attachmentsEnabled,
      onAttachmentOnlySend,
    ]
  );

  return (
    <AssistantUiRuntimeProvider>
      <ComposerTextBridge value={inputValue} onChange={onInputValueChange} />
      <Thread
        components={components}
        model={model}
        onModelChange={onModelChange}
        loadError={loadError}
        onEscape={onEscape}
        slashCommands={slashCommands}
      />
    </AssistantUiRuntimeProvider>
  );
}

export default AssistantUiChat;
