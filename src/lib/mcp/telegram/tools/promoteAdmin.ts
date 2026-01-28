import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'promote_admin',
  description: 'Promote a user to admin in a group or channel',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      user_id: { type: 'string', description: 'User ID to promote' },
    },
    required: ['chat_id', 'user_id'],
  },
};

export async function promoteAdmin(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const userId = validateId(args.user_id, 'user_id');

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    if (chat.type !== 'channel' && chat.type !== 'supergroup') {
      return { content: [{ type: 'text', text: 'Admin promotion is only available for channels/supergroups.' }], isError: true };
    }

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    await mtprotoService.withFloodWaitHandling(async () => {
      const inputChannel = await client.getInputEntity(entity);
      const inputUser = await client.getInputEntity(userId);
      await client.invoke(
        new Api.channels.EditAdmin({
          channel: inputChannel as unknown as Api.TypeInputChannel,
          userId: inputUser as unknown as Api.TypeInputUser,
          adminRights: new Api.ChatAdminRights({
            changeInfo: true,
            deleteMessages: true,
            banUsers: true,
            inviteUsers: true,
            pinMessages: true,
            manageCall: true,
          }),
          rank: 'Admin',
        }),
      );
    });

    return { content: [{ type: 'text', text: `User ${userId} promoted to admin in ${chat.title ?? chatId}.` }] };
  } catch (error) {
    return logAndFormatError(
      'promote_admin',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.ADMIN,
    );
  }
}
