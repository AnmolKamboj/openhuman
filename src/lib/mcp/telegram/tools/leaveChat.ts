import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'leave_chat',
  description: 'Leave a group or channel',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
    },
    required: ['chat_id'],
  },
};

export async function leaveChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    if (chat.type === 'channel' || chat.type === 'supergroup') {
      await mtprotoService.withFloodWaitHandling(async () => {
        const inputChannel = await client.getInputEntity(entity);
        await client.invoke(
          new Api.channels.LeaveChannel({
            channel: inputChannel as Api.TypeInputChannel,
          }),
        );
      });
    } else {
      await mtprotoService.withFloodWaitHandling(async () => {
        const selfUser = await client.getMe();
        await client.invoke(
          new Api.messages.DeleteChatUser({
            chatId: BigInt(chat.id),
            userId: new Api.InputUserSelf(),
          }),
        );
      });
    }

    return { content: [{ type: 'text', text: `Left chat ${chat.title ?? chatId}.` }] };
  } catch (error) {
    return logAndFormatError(
      'leave_chat',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
