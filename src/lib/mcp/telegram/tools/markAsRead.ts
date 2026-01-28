import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { getChatById } from '../telegramApi';
import { validateId } from '../../validation';
import { mtprotoService } from '../../../../services/mtprotoService';

export const tool: MCPTool = {
  name: 'mark_as_read',
  description: 'Mark messages as read in a chat',
  inputSchema: {
    type: 'object',
    properties: { chat_id: { type: 'string', description: 'Chat ID or username' } },
    required: ['chat_id'],
  },
};

export async function markAsRead(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');

    const chat = getChatById(chatId);
    if (!chat) {
      return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };
    }

    const entity = chat.username ? chat.username : chat.id;
    const client = mtprotoService.getClient();

    await mtprotoService.withFloodWaitHandling(async () => {
      await client.markAsRead(entity);
    });

    return {
      content: [{ type: 'text', text: `Messages in chat ${chatId} marked as read.` }],
    };
  } catch (error) {
    return logAndFormatError(
      'mark_as_read',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
