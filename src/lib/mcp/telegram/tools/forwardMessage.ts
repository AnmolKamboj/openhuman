import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { getChatById } from '../telegramApi';
import { validateId } from '../../validation';
import { mtprotoService } from '../../../../services/mtprotoService';

export const tool: MCPTool = {
  name: 'forward_message',
  description: 'Forward a message to another chat',
  inputSchema: {
    type: 'object',
    properties: {
      from_chat_id: { type: 'string', description: 'Source chat ID' },
      to_chat_id: { type: 'string', description: 'Target chat ID' },
      message_id: { type: 'number', description: 'Message ID' },
    },
    required: ['from_chat_id', 'to_chat_id', 'message_id'],
  },
};

export async function forwardMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const fromChatId = validateId(args.from_chat_id, 'from_chat_id');
    const toChatId = validateId(args.to_chat_id, 'to_chat_id');
    const messageId = typeof args.message_id === 'number' && Number.isInteger(args.message_id)
      ? args.message_id
      : undefined;

    if (messageId === undefined) {
      return {
        content: [{ type: 'text', text: 'message_id must be a positive integer' }],
        isError: true,
      };
    }

    const fromChat = getChatById(fromChatId);
    if (!fromChat) {
      return { content: [{ type: 'text', text: `Source chat not found: ${fromChatId}` }], isError: true };
    }

    const toChat = getChatById(toChatId);
    if (!toChat) {
      return { content: [{ type: 'text', text: `Target chat not found: ${toChatId}` }], isError: true };
    }

    const fromEntity = fromChat.username ? fromChat.username : fromChat.id;
    const toEntity = toChat.username ? toChat.username : toChat.id;
    const client = mtprotoService.getClient();

    await mtprotoService.withFloodWaitHandling(async () => {
      await client.forwardMessages(toEntity, {
        messages: [messageId],
        fromPeer: fromEntity,
      });
    });

    return {
      content: [{ type: 'text', text: `Message ${messageId} forwarded from ${fromChatId} to ${toChatId}.` }],
    };
  } catch (error) {
    return logAndFormatError(
      'forward_message',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
