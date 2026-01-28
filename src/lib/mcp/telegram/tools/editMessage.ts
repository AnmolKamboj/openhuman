import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { getChatById } from '../telegramApi';
import { validateId } from '../../validation';
import { mtprotoService } from '../../../../services/mtprotoService';

export const tool: MCPTool = {
  name: 'edit_message',
  description: 'Edit an existing message',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      message_id: { type: 'number', description: 'Message ID' },
      new_text: { type: 'string', description: 'New message text' },
    },
    required: ['chat_id', 'message_id', 'new_text'],
  },
};

export async function editMessage(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const messageId = typeof args.message_id === 'number' && Number.isInteger(args.message_id)
      ? args.message_id
      : undefined;
    const newText = typeof args.new_text === 'string' ? args.new_text : '';

    if (messageId === undefined) {
      return {
        content: [{ type: 'text', text: 'message_id must be a positive integer' }],
        isError: true,
      };
    }
    if (!newText) {
      return {
        content: [{ type: 'text', text: 'new_text is required' }],
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
      await client.editMessage(entity, { message: messageId, text: newText });
    });

    return {
      content: [{ type: 'text', text: `Message ${messageId} edited successfully.` }],
    };
  } catch (error) {
    return logAndFormatError(
      'edit_message',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.MSG,
    );
  }
}
