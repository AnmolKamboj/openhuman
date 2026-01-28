import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';
import bigInt from 'big-integer';

export const tool: MCPTool = {
  name: 'invite_to_group',
  description: 'Invite users to a group or channel',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      user_ids: { type: 'array', items: { type: 'string' }, description: 'User IDs to invite' },
    },
    required: ['chat_id', 'user_ids'],
  },
};

export async function inviteToGroup(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const userIds = Array.isArray(args.user_ids) ? args.user_ids : [];
    if (userIds.length === 0) return { content: [{ type: 'text', text: 'user_ids must not be empty' }], isError: true };

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    const users: Api.TypeInputUser[] = [];
    for (const uid of userIds) {
      const inputUser = await client.getInputEntity(String(uid));
      users.push(inputUser as unknown as Api.TypeInputUser);
    }

    const inputPeer = await client.getInputEntity(entity);

    if (chat.type === 'channel' || chat.type === 'supergroup') {
      await mtprotoService.withFloodWaitHandling(async () => {
        await client.invoke(
          new Api.channels.InviteToChannel({
            channel: inputPeer as unknown as Api.TypeInputChannel,
            users,
          }),
        );
      });
    } else {
      for (const user of users) {
        await mtprotoService.withFloodWaitHandling(async () => {
          await client.invoke(
            new Api.messages.AddChatUser({
              chatId: bigInt(chat.id),
              userId: user,
              fwdLimit: 100,
            }),
          );
        });
      }
    }

    return { content: [{ type: 'text', text: `Invited ${userIds.length} user(s) to ${chat.title ?? chatId}.` }] };
  } catch (error) {
    return logAndFormatError(
      'invite_to_group',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.GROUP,
    );
  }
}
