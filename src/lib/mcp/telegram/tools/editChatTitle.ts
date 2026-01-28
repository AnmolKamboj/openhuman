import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'edit_chat_title',
  description: 'Edit the title of a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      title: { type: 'string', description: 'New title' },
    },
    required: ['chat_id', 'title'],
  },
};

export async function editChatTitle(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const title = typeof args.title === 'string' ? args.title : '';
    if (!title) return { content: [{ type: 'text', text: 'title is required' }], isError: true };

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    if (chat.type === 'channel' || chat.type === 'supergroup') {
      await mtprotoService.withFloodWaitHandling(async () => {
        const inputChannel = await client.getInputEntity(entity);
        await client.invoke(
          new Api.channels.EditTitle({
            channel: inputChannel as Api.TypeInputChannel,
            title,
          }),
        );
      });
    } else {
      await mtprotoService.withFloodWaitHandling(async () => {
        await client.invoke(
          new Api.messages.EditChatTitle({
            chatId: BigInt(chat.id),
            title,
          }),
        );
      });
    }

    return { content: [{ type: 'text', text: `Chat title updated to "${title}".` }] };
  } catch (error) {
    return logAndFormatError(
      'edit_chat_title',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
