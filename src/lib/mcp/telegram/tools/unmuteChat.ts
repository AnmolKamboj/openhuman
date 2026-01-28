import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'unmute_chat',
  description: 'Unmute notifications for a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
    },
    required: ['chat_id'],
  },
};

export async function unmuteChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    await mtprotoService.withFloodWaitHandling(async () => {
      const inputPeer = await client.getInputEntity(entity);
      await client.invoke(
        new Api.account.UpdateNotifySettings({
          peer: new Api.InputNotifyPeer({ peer: inputPeer }),
          settings: new Api.InputPeerNotifySettings({
            muteUntil: 0,
          }),
        }),
      );
    });

    return { content: [{ type: 'text', text: `Chat ${chatId} unmuted.` }] };
  } catch (error) {
    return logAndFormatError(
      'unmute_chat',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT,
    );
  }
}
