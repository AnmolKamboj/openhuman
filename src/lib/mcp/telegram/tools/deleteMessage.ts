import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { getChatById } from '../telegramApi';
import { validateId } from '../../validation';
import { mtprotoService } from '../../../../services/mtprotoService';

export const tool: MCPTool = {
  name: 'delete_message',
  description: 'Delete a message',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      message_id: { type: 'number', description: 'Message ID' },
    },
    required: ['chat_id', 'message_id'],
  },
};

export async function deleteMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const messageId = typeof args.message_id === 'number' && Number.isInteger(args.message_id)
      ? args.message_id
      : undefined;

    if (messageId === undefined) {
      return {
        content: [{ type: 'text', text: 'message_id must be a positive integer' }],
        isError: true,
      };
    }

    const chat = getChatById(chatId);
    if (!chat) {
      return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };
    }

    const entity = chat.username ? chat.username : chat.id;
    const client = mtprotoService.getClient();

    await mtprotoService.withFloodWaitHandling(async () => {
      await client.deleteMessages(entity, [messageId], { revoke: true });
    });

    return {
      content: [{ type: 'text', text: `Message ${messageId} deleted successfully.` }],
    };
  } catch (error) {
    return logAndFormatError(
      'delete_message',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
