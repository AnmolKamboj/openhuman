import type { MCPTool, MCPToolResult } from '../../types';
import type { TelegramMCPContext } from '../types';
import { ErrorCategory, logAndFormatError } from '../../errorHandler';
import { validateId } from '../../validation';
import { getChatById } from '../telegramApi';
import { mtprotoService } from '../../../../services/mtprotoService';
import { Api } from 'telegram';

export const tool: MCPTool = {
  name: 'mute_chat',
  description: 'Mute notifications for a chat',
  inputSchema: {
    type: 'object',
    properties: {
      chat_id: { type: 'string', description: 'Chat ID or username' },
      duration: { type: 'number', description: 'Mute duration in seconds (0 = forever)', default: 0 },
    },
    required: ['chat_id'],
  },
};

export async function muteChat(
  args: Record<string, unknown>,
  _context: TelegramMCPContext,
): Promise<MCPToolResult> {
  try {
    const chatId = validateId(args.chat_id, 'chat_id');
    const duration = typeof args.duration === 'number' ? args.duration : 0;

    const chat = getChatById(chatId);
    if (!chat) return { content: [{ type: 'text', text: `Chat not found: ${chatId}` }], isError: true };

    const client = mtprotoService.getClient();
    const entity = chat.username ? chat.username : chat.id;

    const muteUntil = duration === 0 ? 2147483647 : Math.floor(Date.now() / 1000) + duration;

    await mtprotoService.withFloodWaitHandling(async () => {
      const inputPeer = await client.getInputEntity(entity);
      await client.invoke(
        new Api.account.UpdateNotifySettings({
          peer: new Api.InputNotifyPeer({ peer: inputPeer }),
          settings: new Api.InputPeerNotifySettings({
            muteUntil,
          }),
        }),
      );
    });

    return { content: [{ type: 'text', text: `Chat ${chatId} muted.` }] };
  } catch (error) {
    return logAndFormatError(
      'mute_chat',
      error instanceof Error ? error : new Error(String(error)),
      ErrorCategory.CHAT,
    );
  }
}
